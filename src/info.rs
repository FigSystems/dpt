use crate::run::join_proper;
use crate::store::get_store_location;
use std::path::{Path, PathBuf};

use crate::pkg::Package;

use anyhow::Result;

/// Gets the info location for a package
pub fn package_to_info_location(pkg: &Package) -> Result<PathBuf> {
    Ok(join_proper(
        &get_store_location(),
        &Path::new(&format!("{}-{}", pkg.name, pkg.version)),
    )?
    .join("info"))
}

pub fn mark_as_manually_installed(pkg: &Package) -> Result<()> {
    let loc = package_to_info_location(pkg)?;
    std::fs::DirBuilder::new().recursive(true).create(&loc)?;
    std::fs::write(loc.join("manually_installed"), "")?;
    Ok(())
}

pub fn get_manually_installed(pkg: &Package) -> Result<bool> {
    let loc = package_to_info_location(pkg)?;
    Ok(loc.join("manually_installed").is_file())
}
