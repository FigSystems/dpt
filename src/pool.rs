use std::path::PathBuf;

pub fn get_pool_location() -> PathBuf {
    match crate::config::get_config_option(&"pool".to_string()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from("/fpkg/pool"),
    }
}
