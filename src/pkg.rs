use kdl::{KdlDocument, KdlError, KdlValue};
use std::path::Path;

pub struct Dependency {
    name: String,
    version_mask: String,
}

pub struct PackageConfig {
    name: String,
    version: String,
    developer: String,
    depends: Vec<Dependency>,
}

fn check_kdl_value_string(doc: &KdlDocument, field: String) -> Result<String, String> {
    let field_value: Vec<&KdlValue> = doc.iter_args(&field).collect();
    if field_value.len() < 1 {
        return Err(format!("{} is not specified", field));
    }
    if !field_value.get(0).unwrap().is_string() {
        return Err(format!("{} is not a string", field));
    }

    Ok(field_value.get(0).unwrap().to_string())
}

pub fn verify_pkg_config(file: String) -> Result<(), String> {
    match get_package_config(file) {
        Err(x) => Err(x),
        Ok(_) => Ok(()),
    }
}

pub fn get_package_config(file: String) -> Result<PackageConfig, String> {
    let doc: Result<KdlDocument, KdlError> = file.parse();
    if let Err(e) = doc {
        return Err(e.to_string());
    }
    let doc = doc.unwrap();

    let name = check_kdl_value_string(&doc, "name".to_string())?;
    let version = check_kdl_value_string(&doc, "version".to_string())?;
    let developer = check_kdl_value_string(&doc, "developer".to_string())?;

    let mut depends: Vec<Dependency> = Vec::new();
    for node in doc.nodes().into_iter() {
        if node.name().value() == "depends" {
            let name = node.get(0);
            if let None = name {
                return Err("Name not specified for dependency!".to_string());
            }
            let name = name.unwrap();
            log::debug!("depends {}", name);

            let version = {
                let x = node.children();
                if !x.is_none() {
                    let x = x.unwrap();
                    check_kdl_value_string(&x, "version".to_string())?
                } else {
                    "*.*.*".to_string()
                }
            };
            depends.push(Dependency {
                name: name.to_string(),
                version_mask: version,
            });
        }
    }
    Ok(PackageConfig {
        name,
        version,
        developer,
        depends,
    })
}

/// Tars the directory and compresses it into a .fpkg
pub fn package_pkg(dir: &Path, out: &Path) -> Result<(), String> {
    Ok(())
}

/// Extracts the .fpkg into a directory
pub fn extract_pkg(pkg: &Path, out: &Path) -> Result<(), String> {
    Ok(())
}
