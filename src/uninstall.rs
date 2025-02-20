use crate::{
    env::pool_to_env_location,
    pkg::{onlinepackage_to_package, Package},
    pool::package_to_pool_location,
    repo::{
        package_to_onlinepackage, resolve_dependencies_for_package,
        OnlinePackage,
    },
};
use anyhow::{bail, Context, Result};

#[derive(Debug)]
pub enum UninstallResult {
    DependedUponBy(Package),
    Ok,
}

pub fn uninstall_package(pkg: &Package) -> Result<()> {
    let pool_loc = package_to_pool_location(pkg);
    let env_loc = pool_to_env_location(&pool_loc)?;

    if !pool_loc.exists() || !pool_loc.is_dir() {
        bail!("{} does not have a path in the pool!", &pkg);
    }

    if !env_loc.exists() || !env_loc.is_dir() {
        bail!("{} does not have a path in the env directory!", &pkg);
    }

    std::fs::remove_dir_all(&pool_loc)?;
    std::fs::remove_dir_all(&env_loc)?;
    Ok(())
}

pub fn uninstall_package_and_deps(
    pkg: &Package,
    pkgs: &Vec<OnlinePackage>,
) -> Result<UninstallResult> {
    let self_pkg = package_to_onlinepackage(pkg, pkgs)
        .context(format!("Failed to find package {}!", pkg))?;
    for other_pkg in pkgs {
        let other_pkg_pkg = onlinepackage_to_package(&other_pkg);
        if &other_pkg_pkg == pkg {
            continue;
        }

        let depends = resolve_dependencies_for_package(pkgs, &other_pkg_pkg)?;
        for depend in depends {
            if depend == self_pkg {
                return Ok(UninstallResult::DependedUponBy(
                    onlinepackage_to_package(other_pkg),
                ));
            }
        }
    }

    uninstall_package(pkg)?;

    Ok(UninstallResult::Ok)
}
