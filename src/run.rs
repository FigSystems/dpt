use log::error;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use rand::prelude::*;
use sys_mount::{unmount, Mount, MountFlags, UnmountFlags};

use crate::{
    env::package_to_env_location,
    pkg::Package,
    store::{get_store_location, package_to_store_location},
};

pub fn get_run_location() -> PathBuf {
    match crate::config::get_config_option(&"run".to_string()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from("/fpkg/run/"),
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
pub fn mount(
    src: &Path,
    target: &Path,
) -> Result<sys_mount::Mount, std::io::Error> {
    Mount::builder().flags(MountFlags::BIND).mount(src, target)
}

pub fn bind_mount(src: &Path, target: &Path) -> Result<()> {
    std::fs::DirBuilder::new().recursive(true).create(&target)?;
    match mount(&src, &target) {
        Err(x) => bail!(x.to_string()),
        Ok(_) => Ok(()),
    }
}

pub fn run_pkg(pkg: &Package, uid: u32, args: Vec<String>) -> Result<()> {
    let mut out_dir = PathBuf::from("/");
    while out_dir.exists() || out_dir == PathBuf::from("/") {
        out_dir = get_run_location().join(get_random_string(10));
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&out_dir)?;

    let store_dir = get_store_location();
    let pkg_store_dir = package_to_store_location(pkg);
    if !pkg_store_dir.is_dir() {
        bail!("Package {}-{} not found!", pkg.name, pkg.version);
    }

    let env_dir = package_to_env_location(pkg)?;

    if !env_dir.is_dir() {
        bail!(
            "Package {}-{} does not have an environment!",
            pkg.name,
            pkg.version
        );
    }

    // Bind mount fpkg store inside the out_dir
    let store_target = join_proper(&out_dir, &store_dir)?;
    bind_mount(&store_dir, &store_target)?;

    // Bind mount previous root directory
    let root = PathBuf::from("/");
    let root_target = join_proper(&out_dir, Path::new("fpkg-root"))?;
    bind_mount(&root, &root_target)?;

    let mut binds = Vec::<PathBuf>::new();

    for ent in std::fs::read_dir(&env_dir)? {
        let ent = ent?;
        let ent_full_path = env_dir.join(ent.path());
        let ent_path = ent.path();
        let ent_relative_path = ent_path.strip_prefix(&env_dir)?;
        if !ent_full_path.exists() {
            bail!(
                "Path {} does not exist even though it showed up in the directory listing!",
                ent_full_path.display()
            );
        }

        let target = join_proper(&out_dir, ent_relative_path)?;
        bind_mount(&ent_full_path, &target)?;
        binds.push(target);
    }

    for bind in vec!["dev", "mnt", "media", "run", "var", "home", "tmp"] {
        let dir = Path::new("/").join(bind);
        let dir_target = out_dir.join(bind);
        if dir_target.exists() {
            continue;
        }
        bind_mount(&dir, &dir_target)?;
        binds.push(dir_target);
    }

    let mut cleanup = false;

    let mut prefix = "/";
    if out_dir.join("bin").join(pkg.name.as_str()).is_file()
        || out_dir.join("bin").join(pkg.name.as_str()).is_symlink()
    {
        prefix = "/bin";
    } else if out_dir.join("usr/bin").join(pkg.name.as_str()).is_file()
        || out_dir.join("usr/bin").join(pkg.name.as_str()).is_symlink()
    {
        prefix = "/usr/bin";
    } else {
        error!("Warning! No executable found!");
        cleanup = true;
    }

    if !cleanup {
        let _ = std::process::Command::new(std::env::current_exe()?)
            .arg("chroot-not-intended-for-interactive-use")
            .arg(&out_dir.to_str().ok_or(anyhow::anyhow!(
                "Failed to parse directory {} into string!",
                &out_dir.display()
            ))?)
            .arg(uid.to_string())
            .arg(Path::new(prefix).join(&pkg.name))
            .args(args)
            .spawn()?
            .wait();
    }

    let mut binds2: Vec<PathBuf> = Vec::new();
    let mut binds = binds;
    binds.push(root_target);
    binds.push(store_target);

    for _ in 0..5 {
        for bind in &binds {
            let e = unmount(&bind, UnmountFlags::empty());
            if e.is_err() {
                binds2.push(bind.clone());
            } else {
                assert!(bind.read_dir()?.next().is_none());
            }
        }
        binds = binds2.clone();
        binds2 = Vec::new();
    }

    assert!(binds2.is_empty(), "Terminated to prevent data loss");

    std::fs::remove_dir_all(&out_dir)?;

    Ok(())
}
