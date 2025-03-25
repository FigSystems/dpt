use std::{os::unix::fs::symlink, path::Path};

use crate::{dpt_file::DptFile, store::get_dpt_dir};
use anyhow::Result;

fn mkdir_p(d: &Path) -> Result<()> {
    std::fs::DirBuilder::new().recursive(true).create(&d)?;
    Ok(())
}

fn rebuild_base_(dpt: &DptFile, base_dir: &Path) -> Result<()> {
    mkdir_p(&base_dir)?;
    mkdir_p(&base_dir.join("usr/bin"))?;
    mkdir_p(&base_dir.join("usr/lib"))?;
    mkdir_p(&base_dir.join("etc"))?;
    symlink("usr/lib", &base_dir.join("lib"))?;
    symlink("usr/lib", &base_dir.join("lib64"))?;
    symlink("usr/bin", &base_dir.join("bin"))?;
    symlink("usr/bin", &base_dir.join("sbin"))?;
    symlink("bin", &base_dir.join("usr/sbin"))?;
    symlink("lib", &base_dir.join("usr/lib64"))?;

    let mut passwd = String::new();
    for user in dpt.users.iter() {
        passwd.push_str(&format!(
            "{}:x:{}:{}:{}:{}:{}\n",
            user.username,
            user.uid,
            user.gid,
            user.gecos,
            user.home_dir,
            user.shell
        ));
    }
    std::fs::write(base_dir.join("etc/passwd"), passwd)?;

    let mut group = String::new();
    for g in dpt.groups.iter() {
        let empty_string = String::new();
        let mut members_str =
            g.members.get(0).unwrap_or(&empty_string).to_string();
        for (i, m) in g.members.iter().enumerate() {
            if i == 0 {
                continue;
            }
            members_str.push_str(&format!(",{}", m));
        }
        group.push_str(&format!("{}:*:{}:{}", g.groupname, g.gid, members_str));
    }

    std::fs::write(base_dir.join("etc/group"), group)?;
    Ok(())
}

pub fn rebuild_base(dpt: &DptFile) -> Result<()> {
    let dpt_dir = get_dpt_dir();
    let base_dir = dpt_dir.join("base");
    let base_bak_dir = dpt_dir.join("base.bak");
    remove_if_exists(&base_bak_dir)?;
    if base_dir.exists() || base_dir.is_symlink() {
        std::fs::rename(&base_dir, &base_bak_dir)?;
    }
    if let Err(x) = rebuild_base_(&dpt, &base_dir) {
        std::fs::rename(&base_bak_dir, &base_dir)?;
        return Err(x);
    }
    Ok(())
}

fn remove_if_exists(p: &Path) -> Result<()> {
    if p.is_dir() {
        std::fs::remove_dir_all(&p)?;
    } else if p.is_file() || p.is_symlink() {
        std::fs::remove_file(&p)?;
    }

    Ok(())
}
