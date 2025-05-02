use std::{
    fs::{self, hard_link, read_link},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use log::warn;
use walkdir::WalkDir;

use crate::{
    pkg::{self, Glue, Package},
    repo::{resolve_dependencies_for_packages, OnlinePackage},
    run::join_proper,
    store::get_dpt_dir,
};

/// Generates the environment for a package, version solving to find dependencies.
pub fn generate_environment_for_packages(
    pkgs_selected: &Vec<Package>,
    pkgs: &Vec<OnlinePackage>,
    out_path: &Path,
) -> Result<()> {
    let packages_resolved =
        resolve_dependencies_for_packages(&pkgs, &pkgs_selected)?;
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

    let mut glues: Vec<Glue> = Vec::new();
    let mut pkg_dirs: Vec<PathBuf> = Vec::new();
    for x in packages_resolved {
        let config =
            pkg::get_package_config(&std::fs::read_to_string(&x.url)?)?;
        for glue in config.glue {
            if glues.contains(&glue) {
                continue;
            }
            glues.push(glue);
        }
        let p = Path::new(&x.url);
        generate_environment_for_directory(p, &out_path)?;
        pkg_dirs.push(p.to_path_buf());
    }

    for glue in glues {
        generate_glue_for_directory(&glue, &pkg_dirs, &out_path)?;
    }

    Ok(())
}

pub fn generate_glue_for_directory(
    glue: &Glue,
    pkg_dirs: &Vec<PathBuf>,
    out_path: &Path,
) -> Result<()> {
    match glue {
        Glue::Bin => {}
        Glue::Glob(x) => {
            for dir in pkg_dirs {
                for g in x {
                    for f in glob::glob(
                        join_proper(&dir, &Path::new(&g))?
                            .to_str()
                            .ok_or(anyhow!("Improper characters in glob!"))?,
                    )? {
                        if let Ok(file) = &f {
                            if !Path::new(file).is_file() {
                                continue;
                            }
                            let source = file;
                            let target = out_path
                                .join(Path::new(&file).strip_prefix(&dir)?);
                            std::fs::hard_link(&source, &target).context(
                                anyhow!(
                                    "Failed to hardlink {:?} -> {:?}!",
                                    source,
                                    target
                                ),
                            )?;
                        }
                    }
                }
            }
        }
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
                hard_link(source_path, &target_path).context(anyhow!(
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
