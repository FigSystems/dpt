use std::{error::Error, fs, path::Path};

use crate::pkg;

fn directory_exists(dir: &Path) -> bool {
    dir.is_dir()
}

pub fn gen_pkg(dir: &Path, out: &Path) -> Result<(), Box<dyn Error>> {
    if !directory_exists(&dir) {
        return Err(format!("Directory {} does not exist!", &dir.display()).into());
    }
    let config_str = fs::read_to_string(dir.join(Path::new("fpkg/pkg.kdl")))?;

    pkg::verify_pkg_config(config_str)?;
    pkg::package_pkg(&dir, &out)?;
    Ok(())
}
