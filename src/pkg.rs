use kdl::{KdlDocument, KdlError};
use std::path::Path;

pub fn verify_pkg_config(file: String) -> Result<(), String> {
    let doc: Result<KdlDocument, KdlError> = file.parse();
    if let Err(e) = doc {
        return Err(e.to_string());
    }
    let doc = doc.unwrap();
    let name = doc.get("name");
    if let None = name {
        return Err(String::from("Name not found in package spec"));
    }
    Ok(())
}

/// Tars the directory and compresses it into a .fpkg
pub fn package_pkg(dir: &Path, out: &Path) -> Result<(), String> {
    Ok(())
}

/// Extracts the .fpkg into a directory
pub fn extract_pkg(pkg: &Path, out: &Path) -> Result<(), String> {
    Ok(())
}
