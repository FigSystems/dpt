use std::{
    fs::{self, read_link},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use log::warn;
use walkdir::WalkDir;

use crate::{
    pkg::Package,
    repo::{
        package_to_onlinepackage, resolve_dependencies_for_package,
        OnlinePackage,
    },
    store::get_dpt_dir,
};

/// Generates the environment for a package, version solving to find dependencies.
pub fn generate_environment_for_package(
    pkg: &Package,
    pkgs: &Vec<OnlinePackage>,
    out_path: &Path,
    done_list: &mut Vec<Package>,
) -> Result<()> {
    let package = package_to_onlinepackage(pkg, pkgs)?;
    let pkg_dir = PathBuf::from_str(&package.url)?;
    let pkg_data_dir = pkg_dir;

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
        if get_dpt_dir().join("base").is_dir() {
            generate_environment_for_directory(
                &get_dpt_dir().join("base"),
                &out_path,
            )?;
        } else {
            warn!("`base` is not found!");
        }
    }

    generate_environment_for_directory(&pkg_data_dir, &out_path)?;

    done_list.push(pkg.clone());

    // Convert dependencies into packages by version solving
    let dependencies = resolve_dependencies_for_package(&pkgs, pkg)
        .context(anyhow!("Failed to resolve dependencies"))?;

    for dependency in dependencies {
        if done_list.contains(&dependency.clone().to_package()) {
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

pub fn generate_environment_for_directory(
    pkg_data_dir: &Path,
    out_path: &Path,
) -> Result<()> {
    for ent in WalkDir::new(&pkg_data_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let blank_path = ent.path().strip_prefix(&pkg_data_dir)?;
        if blank_path.starts_with("dpt") {
            continue;
        }
        let target_path = out_path.join(blank_path);
        let source_path = ent.path();

        if source_path.is_file() || source_path.is_symlink() {
            if target_path.exists() || target_path.is_symlink() {
                continue; // Another package with higher priority then us put a file here.
            }
            if source_path.is_file() {
                symlink(source_path, &target_path).context(anyhow!(
                    "In creating an symlink for environment [{} -> {}]",
                    source_path.display(),
                    target_path.display()
                ))?;
            } else {
                symlink(read_link(&source_path)?, &target_path)?;
            }
        } else {
            if target_path.symlink_metadata().is_err() {
                fs::DirBuilder::new().recursive(true).create(&target_path)?;
            }
        }
    }

    Ok(())
}
