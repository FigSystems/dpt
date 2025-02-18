use log::error;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use log::info;
use rand::prelude::*;
use sys_mount::{unmount, Mount, MountFlags, UnmountFlags};

use crate::{
    env::{get_env_location, pool_to_env_location},
    pkg::Package,
    pool::{get_pool_location, package_to_pool_location},
};

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
pub fn mount(src: &Path, target: &Path) -> Result<sys_mount::Mount, std::io::Error> {
    Mount::builder().flags(MountFlags::BIND).mount(src, target)
}

pub fn run_pkg(pkg: &Package) -> Result<()> {
    let mut out_dir = PathBuf::from("/");
    while out_dir.exists() || out_dir == PathBuf::from("/") {
        out_dir = Path::new("/run/fpkg/env/").join(get_random_string(10));
    }
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&out_dir)?;

    info!("out_dir: {:?}", &out_dir);

    let pool_dir = package_to_pool_location(pkg);

    if !pool_dir.is_dir() {
        bail!("Package {}-{} not found!", pkg.name, pkg.version);
    }

    let env_dir = pool_to_env_location(&pool_dir).context(format!(
        "In converting path {} into it's pool location",
        &pool_dir.display()
    ))?;

    if !env_dir.is_dir() {
        bail!(
            "Package {}-{} has a location in the pool, but no environment!",
            pkg.name,
            pkg.version
        );
    }

    // Bind mount fpkg pool inside the out_dir
    let pool = get_pool_location();
    let pool_target = join_proper(&out_dir, &pool)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&pool_target)?;
    match mount(&pool, &pool_target) {
        Err(x) => bail!(x.to_string()),
        Ok(_) => {}
    }

    // Bind mount fpkg envs inside the out_dir
    let env = get_env_location();
    let env_target = join_proper(&out_dir, &env)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&env_target)?;
    match mount(&env, &env_target) {
        Err(x) => bail!(x.to_string()),
        Ok(_) => {}
    }

    let mut links = Vec::<PathBuf>::new();

    for ent in std::fs::read_dir(&env_dir)? {
        let ent = ent?;
        let ent_full_path = env_dir.join(ent.path());
        let ent_path = ent.path();
        let ent_relative_path = ent_path.strip_prefix(&env_dir)?;
        if !ent_full_path.exists() {
            bail!(
                "Path {} does not exist even though it showed up in an iterator!",
                ent_full_path.display()
            );
        }

        if make_path_relative(&env).starts_with(ent_relative_path) {
            continue;
        }

        if make_path_relative(&pool).starts_with(ent_relative_path) {
            continue;
        }

        let target = join_proper(&out_dir, ent_relative_path)?;
        std::os::unix::fs::symlink(ent_full_path, &target)?;
        links.push(target);
    }

    let mut cleanup = false;

    let mut prefix = "/";
    if out_dir.join("bin").join(pkg.name.as_str()).is_file() {
        prefix = "/bin";
    } else if out_dir.join("/usr/bin").join(pkg.name.as_str()).is_file() {
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
            .arg(Path::new(prefix).join(&pkg.name))
            .spawn()?
            .wait();
    }

    for ent in &links {
        std::fs::remove_dir_all(ent)?;
    }

    unmount(&env_target, UnmountFlags::empty())?;
    unmount(&pool_target, UnmountFlags::empty())?;

    std::fs::remove_dir(&env_target)?;
    std::fs::remove_dir(&pool_target)?;

    std::fs::remove_dir_all(&out_dir)?;

    Ok(())
}
