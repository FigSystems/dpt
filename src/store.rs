use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::fpkg_file::read_fpkg_lock_file;
use crate::pkg::{get_package_config, Package};
use crate::repo::OnlinePackage;
use anyhow::{anyhow, Result};

pub fn get_fpkg_dir() -> PathBuf {
    if let Ok(x) = fs::read_to_string("/etc/fpkg/dir") {
        PathBuf::from_str(&x)
            .expect("Malformed directory path in `/etc/fpkg/dir`!")
    } else {
        PathBuf::from_str("/fpkg").unwrap()
    }
}

pub fn get_store_location() -> PathBuf {
    get_fpkg_dir().join("store")
}

/// Gets a list of all packages that are installed and in the fpkg configuration.
pub fn get_installed_packages() -> Result<Vec<OnlinePackage>> {
    let store = get_store_location();
    let entries = fs::read_dir(store)?;
    let mut packages = Vec::<OnlinePackage>::new();
    let fpkg = read_fpkg_lock_file()?;

    for ent in entries {
        let path = ent?.path();

        let url = path
            .to_str()
            .ok_or(anyhow!("Failed to parse path into string"))?
            .to_string();

        let doc = fs::read_to_string(path.join("data/fpkg/pkg.kdl"));
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

        if !fpkg.packages.contains(&Package {
            name: pkg_config.name.clone(),
            version: pkg_config.version.clone(),
        }) {
            continue;
        }

        packages.push(OnlinePackage {
            name: pkg_config.name,
            version: pkg_config.version,
            url,
            depends: pkg_config.depends,
        })
    }
    Ok(packages)
}
