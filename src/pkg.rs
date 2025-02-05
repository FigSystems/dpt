use kdl::{KdlDocument, KdlError, KdlValue};
use std::path::Path;

#[derive(Debug)]
pub struct Dependency {
    name: String,
    version_mask: String,
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Dependency) -> bool {
        self.name == other.name && self.version_mask == other.version_mask
    }
}

#[derive(Debug)]
pub struct PackageConfig {
    name: String,
    version: String,
    developer: String,
    depends: Vec<Dependency>,
}

impl PartialEq for PackageConfig {
    fn eq(&self, other: &PackageConfig) -> bool {
        self.name == other.name
            && self.version == other.version
            && self.developer == other.developer
            && self.depends == other.depends
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_pkg_config_1() {
        let s = r#"
name "abcd"
version "145.54.12"
developer "GHJK"

depends coreutils
depends python {
    version "8.9.112"
}"#
        .to_string();
        let expected = PackageConfig {
            name: "abcd".to_string(),
            version: "145.54.12".to_string(),
            developer: "GHJK".to_string(),
            depends: vec![
                Dependency {
                    name: "coreutils".to_string(),
                    version_mask: "*.*.*".to_string(),
                },
                Dependency {
                    name: "python".to_string(),
                    version_mask: "8.9.112".to_string(),
                },
            ],
        };
        let x = get_package_config(s).unwrap();
        assert_eq!(x, expected);
    }
}
