use std::{os::unix::fs::symlink, path::Path};

use crate::{dpt_file::DptFile, store::get_dpt_dir};
use anyhow::Result;

fn mkdir_p(d: &Path) -> Result<()> {
    std::fs::DirBuilder::new().recursive(true).create(&d)?;
    Ok(())
}

fn rebuild_base_(dpt: &DptFile, base_dir: &Path) -> Result<()> {
    build_directory_structure(&base_dir)?;

    let passwd = build_passwd(&dpt);
    std::fs::write(base_dir.join("etc/passwd"), passwd)?;

    let group = build_group(&dpt);
    std::fs::write(base_dir.join("etc/group"), group)?;

    let login_dot_defs = build_login_dot_defs();
    std::fs::write(base_dir.join("etc/login.defs"), login_dot_defs)?;
    Ok(())
}

fn build_directory_structure(base_dir: &Path) -> Result<()> {
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
    Ok(())
}

fn build_passwd(dpt: &DptFile) -> String {
    let mut passwd = String::new();
    for user in dpt.users.iter() {
        passwd.push_str(&format!(
            "{}:x:{}:{}:{}:{}:{}\n",
            user.username,
            user.uid,
            user.gid,
            user.gecos,
            user.home,
            user.shell
        ));
    }
    passwd
}

fn build_group(dpt: &DptFile) -> String {
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
    group
}

fn build_login_dot_defs() -> String {
    r#"
FAIL_DELAY            3
FAILLOG_ENAB          yes
LOG_UNKFAIL_ENAB      no
LOG_OK_LOGINS         no
LASTLOG_ENAB          yes
MAIL_CHECK_ENAB       yes
OBSCURE_CHECKS_ENAB   yes
PORTTIME_CHECKS_ENAB  yes
QUOTAS_ENAB           yes
SYSLOG_SU_ENAB        yes
SYSLOG_SG_ENAB        yes
CONSOLE               /etc/securetty
MOTD_FILE             /etc/motd
FTMP_FILE             /var/log/btmp
NOLOGINS_FILE         /etc/nologin
SU_NAME               su
MAIL_DIR              /var/mail
HUSHLOGIN_FILE        .hushlogin
ENV_HZ                HZ=100
ENV_SUPATH            PATH=/usr/sbin:/usr/bin
ENV_PATH              PATH=/usr/bin
TTYGROUP              tty
TTYPERM               0600
ERASECHAR             0177
KILLCHAR              025
UMASK                 022
PASS_MAX_DAYS         99999
PASS_MIN_DAYS         0
PASS_MIN_LEN          5
PASS_WARN_AGE         7
SU_WHEEL_ONLY         no
UID_MIN               1000
UID_MAX               60000
SYS_UID_MIN           101
SYS_UID_MAX           999
SUB_UID_MIN           100000
SUB_UID_MAX           600100000
SUB_UID_COUNT         65536
GID_MIN               1000
GID_MAX               60000
SYS_GID_MIN           101
SYS_GID_MAX           999
SUB_GID_MIN           100000
SUB_GID_MAX           600100000
SUB_GID_COUNT         65536
LOGIN_RETRIES         5
LOGIN_TIMEOUT         60
PASS_CHANGE_TRIES     5
PASS_ALWAYS_WARN      yes
CHFN_AUTH             yes
CHFN_RESTRICT         rwh
ENCRYPT_METHOD        YESCRYPT
DEFAULT_HOME          yes
NONEXISTENT           /nonexistent
ENVIRON_FILE          /etc/environment
USERGROUPS_ENAB       yes
PREVENT_NO_AUTH       superuser
"#
    .to_string()
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
