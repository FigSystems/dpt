use crate::{
    env::pool_to_env_location,
    info::{get_manually_installed, package_to_info_location},
    pkg::{onlinepackage_to_package, Package},
    pool::{get_installed_packages, package_to_pool_location},
    repo::{resolve_dependencies_for_package, OnlinePackage},
};
use anyhow::{anyhow, bail, Result};
use log::info;

#[derive(Debug)]
pub struct OnlinePackageWithDependCount {
    pkg: OnlinePackage,
    depends_count: u32,
    manually_installed: bool,
}

pub fn uninstall_package(pkg: &Package) -> Result<()> {
    info!("Uninstalling {}", pkg);
    let pool_loc = package_to_pool_location(pkg);
    let env_loc = pool_to_env_location(&pool_loc)?;
    let info_loc = package_to_info_location(pkg);

    if !pool_loc.exists() || !pool_loc.is_dir() {
        bail!("{} does not have a path in the pool!", &pkg);
    }

    if !env_loc.exists() || !env_loc.is_dir() {
        bail!("{} does not have a path in the env directory!", &pkg);
    }

    if info_loc.is_dir() {
        std::fs::remove_dir_all(&info_loc)?;
    }

    std::fs::remove_dir_all(&pool_loc)?;
    std::fs::remove_dir_all(&env_loc)?;
    Ok(())
}

/// Returns the amount of packages that depend on this package
pub fn get_dependency_count_for_package(
) -> Result<Vec<OnlinePackageWithDependCount>> {
    let packages = get_installed_packages()?;
    let mut ret = Vec::<OnlinePackageWithDependCount>::new();

    for package in &packages {
        ret.push(OnlinePackageWithDependCount {
            pkg: package.clone(),
            depends_count: 0,
            manually_installed: get_manually_installed(
                &onlinepackage_to_package(package),
            ),
        })
    }

    for package in &packages {
        let mut dependencies = resolve_dependencies_for_package(
            &packages,
            &onlinepackage_to_package(&package),
        )?;

        let index =
            dependencies
                .iter()
                .position(|x| x == package)
                .ok_or(anyhow!(
                "Failed to find package in dependencies returned by resolve"
            ))?;
        dependencies.swap_remove(index);

        for depend in dependencies {
            let index =
                packages.iter().position(|x| x == &depend).ok_or(anyhow!(
                "Failed to find package in dependencies returned by resolve"
            ))?;

            ret.get_mut(index)
                .ok_or(anyhow!(
                    "Failed to get index {} from array {:#?}",
                    index,
                    &packages
                ))?
                .depends_count += 1;
        }
    }

    Ok(ret)
}

pub fn uninstall_package_and_deps(package: &Package) -> Result<()> {
    let dep_count = get_dependency_count_for_package()?;
    let cloned = package.clone();
    for pkg in dep_count {
        if onlinepackage_to_package(&pkg.pkg) == cloned {
            if pkg.depends_count > 0 {
                bail!("Package is depended upon! Failed to uninstall");
            }
            uninstall_package(&onlinepackage_to_package(&pkg.pkg))?;
            continue;
        }

        let user_depends = if pkg.manually_installed { 1 } else { 0 };
        if pkg.depends_count + user_depends == 0 {
            uninstall_package(&onlinepackage_to_package(&pkg.pkg))?;
        }
    }
    Ok(())
}
