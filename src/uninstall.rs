use crate::{
    env::pool_to_env_location,
    pkg::Package,
    pool::package_to_pool_location,
    repo::{package_to_onlinepackage, OnlinePackage},
};
use anyhow::{Context, Result};

pub fn uninstall_package(
    pkg: &Package,
    pkgs: &Vec<OnlinePackage>,
) -> Result<()> {
    let pool_loc = package_to_pool_location(pkg);
    let env_loc = pool_to_env_location(&pool_loc)?;
    let self_pkg = package_to_onlinepackage(pkg, pkgs)
        .context(format!("Failed to find package {}!", pkg))?;

    Ok(())
}
