use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use kdl::KdlDocument;
use kdl::KdlEntry;
use kdl::KdlNode;

use crate::pkg::parse_kdl;
use crate::pkg::Package;
use crate::store::get_fpkg_dir;

pub struct FpkgFile {
    pub packages: Vec<Package>,
}

pub fn parse_fpkg_file(file: &KdlDocument) -> Result<FpkgFile> {
    let mut packages: Vec<Package> = Vec::new();
    for x in file
        .get("packages")
        .unwrap_or(&KdlNode::new("packages"))
        .children()
        .unwrap_or(&KdlDocument::new())
        .nodes()
    {
        let name = x.name().value().to_owned();
        let version = x
            .entries()
            .get(0)
            .unwrap_or(&KdlEntry::new(""))
            .value()
            .as_string()
            .ok_or(anyhow!("Version field of package is not a string!"))?
            .to_owned();
        packages.push(Package { name, version });
    }

    Ok(FpkgFile { packages })
}

pub fn get_fpkg_file_location() -> PathBuf {
    get_fpkg_dir().join("fpkg.kdl")
}

pub fn get_fpkg_lock_location() -> PathBuf {
    get_fpkg_dir().join("fpkg.lock")
}

pub fn read_fpkg_file() -> Result<FpkgFile> {
    parse_fpkg_file(&parse_kdl(&std::fs::read_to_string(
        get_fpkg_file_location(),
    )?)?)
}

pub fn read_fpkg_lock_file() -> Result<FpkgFile> {
    parse_fpkg_file(&parse_kdl(&std::fs::read_to_string(
        get_fpkg_lock_location(),
    )?)?)
}
