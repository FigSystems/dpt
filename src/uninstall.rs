use crate::{
    env::pool_to_env_location,
    info::get_manually_installed,
    pkg::{onlinepackage_to_package, Package},
    pool::{get_installed_packages, package_to_pool_location},
    repo::{resolve_dependencies_for_package, OnlinePackage},
};
use anyhow::{anyhow, bail, Result};

#[derive(Debug)]
pub enum UninstallResult {
    DependedUponBy(Package),
    Ok,
}

#[derive(Debug)]
pub struct OnlinePackageWithDependCount {
    pkg: OnlinePackage,
    depends_count: u32,
    manually_installed: bool,
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

//pub fn uninstall_package_and_deps(
//    pkg: &Package,
//    pkgs: &Vec<OnlinePackage>,
//) -> Result<UninstallResult> {
//    let self_pkg = package_to_onlinepackage(pkg, pkgs)
//        .context(format!("Failed to find package {}!", pkg))?;
//    for other_pkg in pkgs {
//        let other_pkg_pkg = onlinepackage_to_package(&other_pkg);
//        if &other_pkg_pkg == pkg {
//            continue;
//        }
//
//        let depends = resolve_dependencies_for_package(pkgs, &other_pkg_pkg)?;
//        for depend in depends {
//            if depend == self_pkg {
//                return Ok(UninstallResult::DependedUponBy(
//                    onlinepackage_to_package(other_pkg),
//                ));
//            }
//        }
//    }
//
//    uninstall_package(pkg)?;
//
//    Ok(UninstallResult::Ok)
//}

/// Returns the amount of packages that depend on this package
pub fn get_dependency_count_for_package() -> Result<()> {
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

    Ok(())
}
