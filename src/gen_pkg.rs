use std::{fs, path::Path};

use crate::pkg;

fn directory_exists(dir: &Path) -> bool {
    if let Ok(result) = fs::exists(&dir) {
        if result {
            if Path::is_dir(Path::new(&dir)) {
                return true;
            }
        }
    }
    false
}

pub fn gen_pkg(dir: &Path, out: &Path) -> Result<(), String> {
    if !directory_exists(&dir) {
        return Err(format!("Directory {} does not exist!", &dir.display()));
    }
    let config_str = fs::read_to_string(dir.join(Path::new("fpkg/pkg.kdl")));
    if let Err(error) = config_str {
        return Err(error.to_string());
    }
    let config_str = config_str.unwrap();

    pkg::verify_pkg_config(config_str)?;
    pkg::package_pkg(&dir, &out)?;
    Ok(())
}
