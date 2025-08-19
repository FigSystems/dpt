use log::error;
use nix::mount::MsFlags;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{bail, Context, Result};
use rand::prelude::*;
use sys_mount::{unmount, UnmountFlags};

use crate::{
    pkg::Package,
    store::{get_installed_packages, get_installed_packages_without_dpt_file},
};

pub fn get_run_location() -> PathBuf {
    match crate::config::get_config_option(&"run".to_string()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from("/dpt/run/"),
    }
}

pub fn get_random_string(length: usize) -> String {
    let mut rng = rand::rng();
    let mut ret = String::new();
    for _ in 0..length {
        ret.push(rng.sample(rand::distr::Alphanumeric) as char)
    }
    ret
}

pub fn join_proper(a: &Path, b: &Path) -> Result<PathBuf> {
    Ok(a.join(make_path_relative(b)))
}

pub fn make_path_relative(a: &Path) -> PathBuf {
    match a.strip_prefix("/") {
        Ok(x) => x.to_path_buf(),
        Err(_) => a.to_path_buf(), // a is already a relative path
    }
}

/// Makes src's contents show up at target
pub fn bind_mount_(
    src: &Path,
    target: &Path,
    recursive: bool,
) -> Result<(), std::io::Error> {
    let mut flags = MsFlags::MS_BIND;
    if recursive {
        flags.insert(MsFlags::MS_REC);
    }
    nix::mount::mount(
        Option::<&Path>::Some(src),
        target,
        Option::<&Path>::None,
        flags,
        Option::<&Path>::None,
    )?;
    let mut flags2 = MsFlags::MS_SLAVE;
    if recursive {
        flags2.insert(MsFlags::MS_REC);
    }
    nix::mount::mount(
        Option::<&Path>::None,
        target,
        Option::<&Path>::None,
        flags2,
        Option::<&Path>::None,
    )?;
    Ok(())
}

pub fn unmount_recursive<P: AsRef<Path>>(target: P) -> Result<()> {
    let target_path = target.as_ref().canonicalize()?; // get absolute path

    // Step 1: Read /proc/self/mountinfo
    let file = File::open("/proc/self/mountinfo")?;
    let reader = BufReader::new(file);

    let mut mounts = Vec::new();

    for line in reader.lines() {
        let line = line?;
        // Format: ID parentID major:minor root mount_point ...
        // Example: 27 24 0:23 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let mount_point = PathBuf::from(parts[4]);
        if mount_point.starts_with(&target_path) {
            mounts.push(mount_point);
        }
    }

    // Step 2: Sort deepest paths first so we unmount children before parents
    mounts.sort_by(|a, b| b.components().count().cmp(&a.components().count()));

    // Step 3: Unmount each path
    for mnt in mounts {
        if let Err(e) = unmount(&mnt, UnmountFlags::empty()) {
            eprintln!("Failed to unmount {}: {}", mnt.display(), e);
        }
    }

    Ok(())
}

pub fn bind_mount(src: &Path, target: &Path, recursive: bool) -> Result<()> {
    if src.is_dir() {
        std::fs::DirBuilder::new().recursive(true).create(&target)?;
    } else {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&target.parent().unwrap_or(Path::new("/")))?;
        std::fs::File::create(&target)?;
    }
    match bind_mount_(&src, &target, recursive) {
        Err(x) => bail!(x.to_string()),
        Ok(_) => Ok(()),
    }
}

pub fn run_pkg(
    pkg: &Package,
    uid: u32,
    args: Vec<String>,
    cmd: Option<&str>,
    allow_non_dpt_file: bool,
    replace_current_process: bool,
) -> Result<i32> {
    run_multiple_packages(
        &vec![pkg.clone()],
        uid,
        args,
        cmd,
        allow_non_dpt_file,
        replace_current_process,
    )
}

pub fn run_pkg_(
    out_dir: &Path,
    uid: u32,
    args: Vec<String>,
    cmd: &str,
    replace_current_process: bool,
) -> Result<i32> {
    let mut cleanup = false;

    let mut prefix = "/";
    if out_dir.join("bin").join(&cmd).is_file()
        || out_dir.join("bin").join(&cmd).is_symlink()
    {
        prefix = "/bin";
    } else if out_dir.join("usr/bin").join(&cmd).is_file()
        || out_dir.join("usr/bin").join(&cmd).is_symlink()
    {
        prefix = "/usr/bin";
    } else {
        error!("No executable found!");
        cleanup = true;
    }
    let mut code: i32 = 0;
    if !cleanup {
        let mut proc = std::process::Command::new(
            std::env::current_exe().unwrap_or(PathBuf::from("/dpt/dpt")),
        );
        let proc = proc
            .arg("run-pkg-second-stage-not-intended-for-interactive-use")
            .arg(&out_dir.to_str().ok_or(anyhow::anyhow!(
                "Failed to parse directory {} into string!",
                &out_dir.display()
            ))?)
            .arg(uid.to_string())
            .arg(Path::new(prefix).join(&cmd))
            .arg(if replace_current_process {
                "replace"
            } else {
                "new"
            })
            .args(args);
        if replace_current_process {
            let err = proc.exec();
            bail!("Failed to run process! Error: {err}");
        } else {
            let proc: Arc<Mutex<std::process::Child>> =
                Arc::new(Mutex::new(proc.spawn()?));

            let proc_arc_clone = Arc::clone(&proc);
            ctrlc::set_handler(move || {
                let l = proc_arc_clone.lock();
                if let Ok(mut x) = l {
                    let _ = x.kill();
                };
            })
            .context("Failed to register ctrlc signal handler")?;
            let l = proc.lock();
            if let Ok(mut x) = l {
                code = x.wait()?.code().unwrap_or(89);
            } else {
                code = 243;
            }
        }
    }

    Ok(code)
}

pub fn run_multiple_packages(
    pkgs: &Vec<Package>,
    uid: u32,
    args: Vec<String>,
    cmd: Option<&str>,
    allow_non_dpt_file: bool,
    replace_current_process: bool,
) -> Result<i32> {
    if pkgs.is_empty() {
        bail!("No packages specified!");
    }

    let mut pkg_path = get_run_location().join(get_random_string(10));
    while pkg_path.exists() {
        pkg_path = get_run_location().join(get_random_string(10));
    }
    let pkg_path = pkg_path;

    let installed_packages = if allow_non_dpt_file == false {
        get_installed_packages()?
    } else {
        get_installed_packages_without_dpt_file()?
    };

    crate::env::generate_environment_for_packages(
        pkgs,
        &installed_packages,
        &pkg_path,
        allow_non_dpt_file,
    )?;

    let cmd = cmd.unwrap_or(&pkgs[0].name);

    let code = run_pkg_(&pkg_path, uid, args, cmd, replace_current_process)?;
    std::fs::remove_dir_all(pkg_path)?;

    Ok(code)
}
