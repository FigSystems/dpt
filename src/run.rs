use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use libmount::BindMount;
use log::info;
use rand::prelude::*;

use crate::{
    env::get_env_location,
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
    Ok(a.join(
        b.strip_prefix("/")
            .context("Pool location is not an absolute path!")?,
    ))
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

    let env_dir = package_to_pool_location(pkg);

    // Bind mount fpkg pool inside the out_dir
    let pool = get_pool_location();
    let pool_target = join_proper(&out_dir, &pool)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&pool_target)?;
    match BindMount::new(pool, pool_target).mount() {
        Err(x) => return Err(anyhow::anyhow!(x.to_string())),
        Ok(_) => {}
    }

    // Bind mount fpkg envs inside the out_dir
    let env = get_env_location();
    let env_target = join_proper(&out_dir, &env)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&env_target)?;
    match BindMount::new(env, env_target).mount() {
        Err(x) => return Err(anyhow::anyhow!(x.to_string())),
        Ok(_) => {}
    }

    Ok(())
}
