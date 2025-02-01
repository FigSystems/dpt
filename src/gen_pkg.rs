use std::{fs, path::Path};

fn directory_exists(dir: &String) -> bool {
    if let Ok(result) = fs::exists(&dir) {
        if result {
            if Path::is_dir(Path::new(&dir)) {
                return true;
            }
        }
    }
    false
}

pub fn gen_pkg(dir: String) -> Result<(), String> {
    if !directory_exists(&dir) {
        return Err(format!("Directory {} does not exist!", &dir));
    }
    // verify the environment configuration

    Ok(())
}
