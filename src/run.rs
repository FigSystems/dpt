use log::error;
use nix::mount::MsFlags;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{bail, Context, Result};
use rand::prelude::*;
use sys_mount::{unmount, UnmountFlags};

use crate::{
    pkg::Package,
    store::{
        get_dpt_dir, get_installed_packages,
        get_installed_packages_without_dpt_file,
    },
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
pub fn bind_mount_(src: &Path, target: &Path) -> Result<(), std::io::Error> {
    let mut flags = MsFlags::MS_BIND;
    flags.insert(MsFlags::MS_REC);
    nix::mount::mount(
        Option::<&Path>::Some(src),
        target,
        Option::<&Path>::None,
        flags,
        Option::<&Path>::None,
    )?;
    nix::mount::mount(
        Option::<&Path>::None,
        target,
        Option::<&Path>::None,
        MsFlags::MS_SLAVE.union(MsFlags::MS_REC),
        Option::<&Path>::None,
    )?;
    Ok(())
}

pub fn bind_mount(src: &Path, target: &Path) -> Result<()> {
    if src.is_dir() {
        std::fs::DirBuilder::new().recursive(true).create(&target)?;
    } else {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&target.parent().unwrap_or(Path::new("/")))?;
        std::fs::File::create(&target)?;
    }
    match bind_mount_(&src, &target) {
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
) -> Result<i32> {
    run_multiple_packages(
        &vec![pkg.clone()],
        uid,
        args,
        cmd,
        allow_non_dpt_file,
    )
}

pub fn run_pkg_(
    out_dir: &Path,
    uid: u32,
    args: Vec<String>,
    cmd: &str,
) -> Result<i32> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&out_dir)?;

    let fpkg_dir = get_dpt_dir();

    // Bind mount dpt dir inside the out_dir
    let fpkg_target = join_proper(&out_dir, &fpkg_dir)?;
    bind_mount(&fpkg_dir, &fpkg_target)?;

    let mut binds = Vec::<PathBuf>::new();

    for bind in vec![
        "dev", "mnt", "media", "run", "var", "home", "tmp", "proc", "tmp",
    ] {
        let dir = Path::new("/").join(bind);
        let dir_target = out_dir.join(bind);
        if dir_target.exists() {
            continue;
        }
        if !dir.exists() {
            continue;
        }
        bind_mount(&dir, &dir_target)?;
        binds.push(dir_target);
    }

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
        let mut proc = std::process::Command::new(std::env::current_exe()?);
        let proc: Arc<Mutex<std::process::Child>> = Arc::new(Mutex::new(
            proc.arg("chroot-not-intended-for-interactive-use")
                .arg(&out_dir.to_str().ok_or(anyhow::anyhow!(
                    "Failed to parse directory {} into string!",
                    &out_dir.display()
                ))?)
                .arg(uid.to_string())
                .arg(Path::new(prefix).join(&cmd))
                .args(args)
                .spawn()?,
        ));
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
    let mut binds2: Vec<PathBuf> = Vec::new();
    let mut binds = binds;
    binds.push(fpkg_target);

    for _ in 0..10 {
        for bind in &binds {
            let e = unmount(&bind, UnmountFlags::DETACH);
            if e.is_err() {
                binds2.push(bind.clone());
            } else {
                if bind.is_dir() {
                    if bind.read_dir()?.next().is_some() {
                        for p in walkdir::WalkDir::new(&bind) {
                            if let Ok(p) = p {
                                let _ =
                                    unmount(&p.path(), UnmountFlags::empty());
                            }
                        }

                        binds2.push(bind.clone());
                    }
                }
            }
        }
        binds = binds2.clone();
        binds2 = Vec::new();
    }

    assert!(binds2.is_empty(), "Terminated to prevent data loss");

    Ok(code)
}

pub fn run_multiple_packages(
    pkgs: &Vec<Package>,
    uid: u32,
    args: Vec<String>,
    cmd: Option<&str>,
    allow_non_dpt_file: bool,
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

    let code = run_pkg_(&pkg_path, uid, args, cmd)?;
    std::fs::remove_dir_all(pkg_path)?;

    Ok(code)
}
