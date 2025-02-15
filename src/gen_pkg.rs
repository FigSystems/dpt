use anyhow::{bail, Context, Result};
use std::{fs, path::Path};

use crate::pkg;

fn directory_exists(dir: &Path) -> bool {
    dir.is_dir()
}

pub fn gen_pkg(dir: &Path, out: &Path) -> Result<()> {
    if !directory_exists(&dir) {
        bail!("Directory {} does not exist!", &dir.display());
    }
    let config_str =
        fs::read_to_string(dir.join(Path::new("fpkg/pkg.kdl"))).context("pkg.kdl not found")?;
    let config_str = config_str.as_str();

    pkg::verify_pkg_config(config_str)?;
    pkg::package_pkg(&dir, &out)?;
    Ok(())
}
