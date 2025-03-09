use crate::{
    info::get_manually_installed,
    pkg::{onlinepackage_to_package, Package},
    repo::{resolve_dependencies_for_package, OnlinePackage},
    store::{get_installed_packages, package_to_store_location},
};
use anyhow::{anyhow, bail, Result};
use log::{debug, info};

#[derive(Debug)]
pub struct OnlinePackageWithDependCount {
    pkg: OnlinePackage,
    depends_count: u32,
    manually_installed: bool,
    dependers: Vec<OnlinePackage>,
}

pub fn uninstall_package(pkg: &Package) -> Result<()> {
    info!("Uninstalling {}", pkg);
    let store_loc = package_to_store_location(pkg);

    if !store_loc.exists() || !store_loc.is_dir() {
        bail!("{} is not installed!", &pkg);
    }

    std::fs::remove_dir_all(&store_loc)?;
    Ok(())
}

/// Returns the dependency count of each package
pub fn get_dependency_count_for_packages(
    packages: &Vec<OnlinePackage>,
) -> Result<Vec<OnlinePackageWithDependCount>> {
    let packages = packages.clone();
    let mut ret = Vec::<OnlinePackageWithDependCount>::new();

    for package in &packages {
        ret.push(OnlinePackageWithDependCount {
            pkg: package.clone(),
            depends_count: 0,
            manually_installed: get_manually_installed(
                &onlinepackage_to_package(package),
            )?,
            dependers: Vec::new(),
        })
    }

    for package in &packages {
        let dependencies = resolve_dependencies_for_package(
            &packages,
            &onlinepackage_to_package(&package),
        )?;

        for depend in dependencies {
            if &depend == package {
                continue;
            }
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
            debug!("{} \t::: \t{}", depend.name, package.name);
            ret.get_mut(index)
                .ok_or(anyhow!(
                    "Failed to get index {} from array {:#?}",
                    index,
                    &packages
                ))?
                .dependers
                .push(depend);
        }
    }

    Ok(ret)
}

pub fn uninstall_package_and_deps(package: Option<&Package>) -> Result<()> {
    let packages = get_installed_packages(false)?;
    let dep_count = get_dependency_count_for_packages(&packages)?;

    let cloned = package.clone();
    for pkg in dep_count {
        if Some(&onlinepackage_to_package(&pkg.pkg)) == cloned {
            if pkg.depends_count > 0 {
                bail!("Package is depended upon by these packages: {:#?} Failed to uninstall", pkg.dependers);
            }
            uninstall_package(&onlinepackage_to_package(&pkg.pkg))?;
            continue;
        }

        if pkg.depends_count == 0 && !pkg.manually_installed {
            uninstall_package(&onlinepackage_to_package(&pkg.pkg))?;
        }
    }
    uninstall_package_and_deps(None)?;
    Ok(())
}
