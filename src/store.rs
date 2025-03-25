use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::dpt_file::read_dpt_lock_file;
use crate::pkg::{get_package_config, Package};
use crate::repo::OnlinePackage;
use anyhow::{anyhow, Result};

pub fn get_dpt_dir() -> PathBuf {
    if let Ok(x) = fs::read_to_string("/etc/dpt/dir") {
        PathBuf::from_str(&x)
            .expect("Malformed directory path in `/etc/dpt/dir`!")
    } else {
        PathBuf::from_str("/dpt").unwrap()
    }
}

pub fn get_store_location() -> PathBuf {
    get_dpt_dir().join("store")
}

pub fn get_installed_packages_without_dpt_file() -> Result<Vec<OnlinePackage>> {
    let store = get_store_location();
    let entries = fs::read_dir(store)?;
    let mut packages = Vec::<OnlinePackage>::new();

    for ent in entries {
        let path = ent?.path();

        let url = path
            .to_str()
            .ok_or(anyhow!("Failed to parse path into string"))?
            .to_string();

        let doc = fs::read_to_string(path.join("dpt/pkg.kdl"));
        if let Err(_) = doc {
            log::warn!(
                "Failed to read the configuration file for package {}!",
                path.display()
            );
            continue;
        }
        let doc = doc.unwrap();

        let pkg_config = get_package_config(&doc);
        if let Err(_) = pkg_config {
            log::warn!(
                "Malformed package config for package {}!",
                path.display()
            );
            continue;
        }
        let pkg_config = pkg_config.unwrap();

        packages.push(OnlinePackage {
            name: pkg_config.name,
            version: pkg_config.version,
            url,
            depends: pkg_config.depends,
        })
    }
    Ok(packages)
}

/// Gets a list of all packages that are installed and in the dpt configuration.
pub fn get_installed_packages() -> Result<Vec<OnlinePackage>> {
    let dpt = read_dpt_lock_file()?;
    Ok(get_installed_packages_without_dpt_file()?
        .iter()
        .filter(|x| {
            dpt.packages.contains(&Package {
                name: x.name.clone(),
                version: x.version.clone(),
            })
        })
        .map(|x| x.to_owned())
        .collect::<Vec<OnlinePackage>>())
}
