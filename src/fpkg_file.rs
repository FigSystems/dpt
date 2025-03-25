use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use kdl::KdlDocument;
use kdl::KdlEntry;
use kdl::KdlNode;
use kdl::KdlValue;

use crate::pkg::parse_kdl;
use crate::pkg::Package;
use crate::store::get_fpkg_dir;

#[derive(Debug, PartialEq, Eq)]
pub struct User {
    pub username: String,
    pub password: String,
    pub uid: u64,
    pub gid: u64,
    pub gecos: String,
    pub home_dir: String,
    pub shell: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    pub groupname: String,
    pub gid: u64,
    pub members: Vec<String>,
}

pub struct FpkgFile {
    pub packages: Vec<Package>,
    pub users: Vec<User>,
    pub groups: Vec<Group>,
}

fn kdlvalue_as_string(v: &KdlValue, n: &str) -> Result<String> {
    Ok(v.as_string()
        .ok_or(anyhow!("Value {n} is not a string!"))?
        .to_string())
}

fn kdlvalue_as_u64(v: &KdlValue, n: &str) -> Result<u64> {
    Ok(v.as_integer()
        .ok_or(anyhow!("Value {n} is not an integer!"))?
        .try_into()
        .context("In converting an integer in the fpkg file to a u64")?)
}

pub fn parse_fpkg_file(file: &KdlDocument) -> Result<FpkgFile> {
    let mut packages: Vec<Package> = Vec::new();
    let mut users: Vec<User> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    for x in file
        .get("packages")
        .unwrap_or(&KdlNode::new("packages"))
        .children()
        .unwrap_or(&KdlDocument::new())
        .nodes()
    {
        let name = x.name().value().to_owned();
        let version = x
            .entries()
            .get(0)
            .unwrap_or(&KdlEntry::new(""))
            .value()
            .as_string()
            .ok_or(anyhow!("Version field of package is not a string!"))?
            .to_owned();
        packages.push(Package::new(name, version));
    }

    for x in file
        .get("users")
        .unwrap_or(&KdlNode::new("users"))
        .children()
        .unwrap_or(&KdlDocument::new())
        .nodes()
    {
        let username = x.name().value().to_owned();
        let ents = x
            .entries()
            .iter()
            .map(|x| x.value())
            .collect::<Vec<&KdlValue>>();
        if ents.len() < 6 {
            bail!(
                "Not enough entries in user declaration! Need 6, found {}",
                ents.len()
            );
        }
        users.push(User {
            username,
            password: kdlvalue_as_string(
                ents[0],
                "password in user declaration",
            )?,
            uid: kdlvalue_as_u64(ents[1], "uid in user declaration")?,
            gid: kdlvalue_as_u64(ents[2], "gid in user declaration")?,
            gecos: kdlvalue_as_string(ents[3], "gecos in user declaration")?,
            home_dir: kdlvalue_as_string(
                ents[4],
                "home directory in user declaration",
            )?,
            shell: kdlvalue_as_string(ents[5], "shell in user configuration")?,
        });
    }

    for x in file
        .get("groups")
        .unwrap_or(&KdlNode::new("groups"))
        .children()
        .unwrap_or(&KdlDocument::new())
        .nodes()
    {
        let groupname = x.name().to_string();
        let gid: u64 = x
            .entries()
            .get(0)
            .ok_or(anyhow!("GID not specified in group declaration!"))?
            .value()
            .as_integer()
            .ok_or(anyhow!("GID is not an integer in group declaration!"))?
            .try_into()
            .context("Failed to convert GID to u64!")?;
        let members = x
            .children()
            .unwrap_or(&KdlDocument::new())
            .nodes()
            .iter()
            .map(|x| x.name().to_string())
            .collect();
        groups.push(Group {
            groupname,
            gid,
            members,
        });
    }

    Ok(FpkgFile {
        packages,
        users,
        groups,
    })
}

pub fn get_fpkg_file_location() -> PathBuf {
    get_fpkg_dir().join("fpkg.kdl")
}

pub fn get_fpkg_lock_location() -> PathBuf {
    get_fpkg_dir().join("fpkg.lock")
}

pub fn read_fpkg_file() -> Result<FpkgFile> {
    parse_fpkg_file(&parse_kdl(&std::fs::read_to_string(
        get_fpkg_file_location(),
    )?)?)
}

pub fn read_fpkg_lock_file() -> Result<FpkgFile> {
    parse_fpkg_file(&parse_kdl(&std::fs::read_to_string(
        get_fpkg_lock_location(),
    )?)?)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn package_array() {
        let doc: KdlDocument = r#"
packages {
    gcc
    binutils
    fish "4.0.0"
    yazi
}
        "#
        .parse()
        .expect("Failed to parse KDL!");

        let out = parse_fpkg_file(&doc).unwrap();
        assert_eq!(
            out.packages,
            vec![
                Package::new("gcc".into(), "".into()),
                Package::new("binutils".into(), "".into()),
                Package::new("fish".into(), "4.0.0".into()),
                Package::new("yazi".into(), "".into())
            ]
        )
    }

    #[test]
    fn users_array() {
        let doc: KdlDocument = r#"
users {
    john "Hashed password" 1000 1000 "John, Room 5" "/home/john" "/usr/bin/fish"
    george "HashBrowns" 1001 1002 "George, Room 8" "/home/george" "/bin/bash"
}
        "#
        .parse()
        .expect("Failed to parse KDL!");

        let out = parse_fpkg_file(&doc).unwrap();
        assert_eq!(
            out.users,
            vec![
                User {
                    username: "john".into(),
                    password: "Hashed password".into(),
                    uid: 1000,
                    gid: 1000,
                    gecos: "John, Room 5".into(),
                    home_dir: "/home/john".into(),
                    shell: "/usr/bin/fish".into()
                },
                User {
                    username: "george".into(),
                    password: "HashBrowns".into(),
                    uid: 1001,
                    gid: 1002,
                    gecos: "George, Room 8".into(),
                    home_dir: "/home/george".into(),
                    shell: "/bin/bash".into()
                }
            ]
        )
    }

    #[test]
    fn groups_array() {
        let doc: KdlDocument = r#"
groups {
    me 1 {
        someone
        someone_else
    }
    nobody 65536 {
        noone
    }
    empty 2
}
        "#
        .parse()
        .unwrap();

        let out = parse_fpkg_file(&doc).unwrap();

        assert_eq!(
            out.groups,
            vec![
                Group {
                    groupname: "me".into(),
                    gid: 1,
                    members: vec!["someone".into(), "someone_else".into()]
                },
                Group {
                    groupname: "nobody".into(),
                    gid: 65536,
                    members: vec!["noone".into()]
                },
                Group {
                    groupname: "empty".into(),
                    gid: 2,
                    members: vec![]
                }
            ]
        )
    }
}
