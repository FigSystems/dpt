use std::{
    fs::{self},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;

use crate::{
    pkg::{onlinepackage_to_package, Package},
    repo::{
        package_to_onlinepackage, resolve_dependencies_for_package,
        OnlinePackage,
    },
    run::join_proper,
    store::get_store_location,
};

pub fn package_to_env_location(pkg: &Package) -> Result<PathBuf> {
    Ok(join_proper(
        &get_store_location(),
        &Path::new(&format!("{}-{}", pkg.name, pkg.version)),
    )?
    .join("env"))
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
    let pkg_data_dir = pkg_dir.clone().join("data");
    // let pkg_env_dir = pkg_dir.clone().join("env");

    pkg_data_dir.metadata().context(format!(
        "Package directory '{}' does not exist!",
        pkg_data_dir.display()
    ))?;

    if done_list.is_empty() {
        if let Ok(x) = std::fs::exists(out_path) {
            if x {
                std::fs::remove_dir_all(out_path)?;
            }
        }
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(out_path)?;

        for dir in vec!["lib", "bin", "sbin"] {
            std::fs::DirBuilder::new()
                .recursive(true)
                .create(out_path.join("usr").join(dir))?;
            let source = Path::new("usr").join(dir);
            let target = out_path.join(dir);
            symlink(source, target)?;
        }
    }

    for ent in WalkDir::new(&pkg_data_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let blank_path = ent.path().strip_prefix(&pkg_data_dir)?;
        if blank_path.starts_with("fpkg") {
            continue;
        }
        let target_path = out_path.join(blank_path);
        let source_path = ent.path();
        if source_path.is_file() || source_path.is_symlink() {
            if target_path.exists() {
                continue; // Another package with higher priority then us put a file here.
            }
            std::os::unix::fs::symlink(source_path, &target_path)
                .context(anyhow!("In creating an symlink for environment"))?;
        } else {
            fs::DirBuilder::new().recursive(true).create(&target_path)?;
        }
    }

    done_list.push(pkg.clone());

    // Convert dependencies into packages by version solving
    let dependencies = resolve_dependencies_for_package(&pkgs, pkg)
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
