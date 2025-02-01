use kdl::{KdlDocument, KdlError, KdlValue};
use std::path::Path;

pub fn verify_pkg_config(file: String) -> Result<(), String> {
    let doc: Result<KdlDocument, KdlError> = file.parse();
    if let Err(e) = doc {
        return Err(e.to_string());
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
