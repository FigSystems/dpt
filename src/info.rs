use std::path::PathBuf;

use crate::pkg::Package;

use anyhow::Result;

pub fn get_info_location() -> PathBuf {
    match crate::config::get_config_option(&"info".to_string()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from("/fpkg/info"),
    }
}

/// Gets the pool location for a package.
pub fn package_to_info_location(pkg: &Package) -> PathBuf {
    get_info_location().join(pkg.name.clone() + "-" + &pkg.version)
}

pub fn mark_as_manually_installed(pkg: &Package) -> Result<()> {
    let loc = package_to_info_location(pkg);
    std::fs::DirBuilder::new().recursive(true).create(&loc)?;
    std::fs::write(loc.join("manually_installed"), "")?;
    Ok(())
}

pub fn get_manually_installed(pkg: &Package) -> bool {
    let loc = package_to_info_location(pkg);
    loc.join("manually_installed").is_file()
}
