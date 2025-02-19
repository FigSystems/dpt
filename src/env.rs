use std::{
    fs::{self},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;

use crate::{
    pkg::{onlinepackage_to_package, Package},
    pool::get_pool_location,
    repo::{
        package_to_onlinepackage, resolve_dependencies_for_package,
        OnlinePackage,
    },
};

/// Get the location of the environment directory
pub fn get_env_location() -> PathBuf {
    match crate::config::get_config_option(&"env".to_string()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from("/fpkg/env"),
    }
}

/// Convert a path in the pool to it's equivalent path in the environment directory
pub fn pool_to_env_location(pool_path: &Path) -> Result<PathBuf> {
    let out_path = pool_path.strip_prefix(get_pool_location())?;
    let out_path = get_env_location().join(out_path);
    Ok(out_path)
}

/// Generates the environment for a package, version solving to find dependencies.
pub fn generate_environment_for_package(
    pkg: &Package,
    pkgs: &Vec<OnlinePackage>,
    out_path: &Path,
    done_list: &mut Vec<Package>,
) -> Result<()> {
    let package = package_to_onlinepackage(pkg, pkgs)?;
    let pkg_dir = PathBuf::from_str(&package.url)?;

    pkg_dir.metadata().context(format!(
        "Package directory '{}' does not exist!",
        pkg_dir.display()
    ))?;

    if done_list.is_empty() {
        if let Ok(x) = std::fs::exists(out_path) {
            if x {
                std::fs::remove_dir_all(out_path)?;
            }
        }
    }

    for ent in WalkDir::new(&pkg_dir).into_iter().filter_map(|e| e.ok()) {
        let blank_path = ent.path().strip_prefix(&pkg_dir)?;
        if blank_path.starts_with("fpkg") {
            continue;
        }
        let target_path = out_path.join(blank_path);
        let source_path = ent.path();
        if !source_path.is_dir() {
            let _ = std::fs::remove_file(&target_path); // It is okay if this fails
            std::os::unix::fs::symlink(source_path, &target_path)
                .context(anyhow!("In creating an symlink for environment"))?;
        } else {
            fs::DirBuilder::new().recursive(true).create(&target_path)?;
        }
    }

    done_list.push(pkg.clone());

    // Convert dependencies into packages by version solving
    let dependencies = resolve_dependencies_for_package(&pkgs, pkg.clone())
        .context(anyhow!("Failed to resolve dependencies"))?;

    for dependency in dependencies {
        if done_list.contains(&onlinepackage_to_package(&dependency)) {
            continue;
        }
        let p = Package {
            name: dependency.name,
            version: dependency.version,
        };
        generate_environment_for_package(&p, pkgs, out_path, done_list)?;
        done_list.push(p);
    }

    Ok(())
}
