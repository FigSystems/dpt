use std::fs;
use std::path::PathBuf;

use crate::pkg::{get_package_config, Package};
use crate::repo::OnlinePackage;
use anyhow::{anyhow, Result};

pub fn get_pool_location() -> PathBuf {
    match crate::config::get_config_option(&"pool".to_string()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from("/fpkg/pool"),
    }
}

pub fn package_to_pool_location(pkg: &Package) -> PathBuf {
    get_pool_location().join(pkg.name.clone() + "-" + &pkg.version)
}

pub fn package_to_pool_package(pkg: &Package) -> Result<OnlinePackage> {
    let url = package_to_pool_location(pkg);
    let name = pkg.name.clone();
    let version = pkg.version.clone();
    let depends = crate::pkg::parse_depends(&crate::pkg::parse_kdl(&fs::read_to_string(
        url.join("fpkg/pkg.kdl"),
    )?)?)?;
    Ok(OnlinePackage {
        name,
        version,
        url: url
            .to_str()
            .ok_or(anyhow!("Invalid file path!"))?
            .to_string(),
        depends,
    })
}

pub fn get_installed_packages() -> Result<Vec<OnlinePackage>> {
    let pool = get_pool_location();
    let entries = fs::read_dir(pool)?;
    let mut packages = Vec::<OnlinePackage>::new();

    for ent in entries {
        let path = ent?.path();

        let url = path
            .to_str()
            .ok_or(anyhow!("Failed to parse path into string"))?
            .to_string();

        let doc = &fs::read_to_string(path.join("fpkg/pkg.kdl"))?;
        let pkg_config = get_package_config(&doc)?;

        packages.push(OnlinePackage {
            name: pkg_config.name,
            version: pkg_config.version,
            url,
            depends: pkg_config.depends,
        })
    }
    Ok(packages)
}
