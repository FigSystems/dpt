use std::path::Path;

use crate::CONFIG_LOCATION;

pub fn get_config_option(name: &str) -> Option<String> {
    let cfg_loc: String = CONFIG_LOCATION.to_string();
    let path = Path::new(&cfg_loc).join(name);
    if !path.is_file() {
        return None;
    }
    if let Ok(out) = std::fs::read_to_string(path) {
        Some(out)
    } else {
        None
    }
}
