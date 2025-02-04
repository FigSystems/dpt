use kdl::{KdlDocument, KdlError, KdlValue};
use std::path::Path;

fn check_kdl_value_string(doc: &KdlDocument, field: String) -> Result<(), String> {
    let field_value: Vec<&KdlValue> = doc.iter_args(&field).collect();
    if field_value.len() < 1 {
        return Err(format!("{} is not specified", field));
    }
    if !field_value.get(1).unwrap().is_string() {
        return Err(format!("{} is not a string", field));
    }

    Ok(())
}

pub fn verify_pkg_config(file: String) -> Result<(), String> {
    let doc: Result<KdlDocument, KdlError> = file.parse();
    if let Err(e) = doc {
        return Err(e.to_string());
    }
    let doc = doc.unwrap();

    check_kdl_value_string(&doc, "name".to_string())?;
    check_kdl_value_string(&doc, "version".to_string())?;
    check_kdl_value_string(&doc, "developer".to_string())?;

    for node in doc.nodes().into_iter() {
        if node.name().value() == "depends" {
            let name = node.get(1);
            if let None = name {
                return Err("Name not specified in dependency!".to_string());
            }
            let name = name.unwrap();
            log::debug!("depends {}", name);
        }
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
