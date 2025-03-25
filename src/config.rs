use crate::store::get_dpt_dir;

/// Gets a configuration option from the system
pub fn get_config_option(name: &str) -> Option<String> {
    let path = get_dpt_dir().join(name);
    if !path.is_file() {
        return None;
    }
    if let Ok(out) = std::fs::read_to_string(path) {
        Some(out)
    } else {
        None
    }
}
