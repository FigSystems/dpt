use std::{path::PathBuf, str::FromStr};

use crate::pkg::Package;
use crate::store::get_dpt_dir;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
    pub uid: u64,
    pub gid: u64,
    pub gecos: String,
    pub home: String,
    pub shell: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Group {
    pub groupname: String,
    pub gid: u64,
    pub members: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DptFile {
    pub packages: Vec<Package>,
    pub users: Vec<User>,
    pub groups: Vec<Group>,
    pub services: Option<HashMap<String, Vec<String>>>,
}

pub fn get_dpt_file_location() -> PathBuf {
    get_dpt_dir().join("dpt.ron")
}

pub fn get_dpt_lock_location() -> PathBuf {
    get_dpt_dir().join("dpt.lock")
}

pub fn read_dpt_file() -> Result<DptFile> {
    Ok(parse_dpt_file(&std::fs::read_to_string(
        get_dpt_file_location(),
    )?)?)
}

pub fn read_dpt_lock_file() -> Result<DptFile> {
    Ok(parse_dpt_file(&std::fs::read_to_string(
        get_dpt_lock_location(),
    )?)?)
}

pub fn parse_dpt_file(file: &str) -> Result<DptFile> {
    let dpt_file = String::from_str("#![enable(implicit_some)]\n")?;
    Ok(ron::from_str(&(dpt_file + file))?)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn package_array() {
        let doc = r#"
(
    packages: [
        (
            name: "gcc",
            version: ""
        ),
        (
            name: "binutils",
            version: ""
        ),
        (
            name: "fish",
            version: "4.0.0"
        ),
        (
            name: "yazi",
            version: ""
        )
    ],
    users: [],
    groups: [],
    services: {}
)
        "#;

        let out = parse_dpt_file(&doc).unwrap();
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
        let doc = r#"
(
    users: [
        (
            username: "john",
            password: "Hashed password",
            uid: 1000,
            gid: 1000,
            gecos: "John, Room 5",
            home: "/home/john",
            shell: "/usr/bin/fish"
        ),
        (
            username: "george",
            password: "HashBrowns",
            uid: 1001,
            gid: 1002,
            gecos: "George, Room 8",
            home: "/home/george",
            shell: "/bin/bash"
        )
    ],
    packages: [],
    groups: [],
    services: {}
)
        "#;

        let out = parse_dpt_file(&doc).unwrap();
        assert_eq!(
            out.users,
            vec![
                User {
                    username: "john".into(),
                    password: "Hashed password".into(),
                    uid: 1000,
                    gid: 1000,
                    gecos: "John, Room 5".into(),
                    home: "/home/john".into(),
                    shell: "/usr/bin/fish".into()
                },
                User {
                    username: "george".into(),
                    password: "HashBrowns".into(),
                    uid: 1001,
                    gid: 1002,
                    gecos: "George, Room 8".into(),
                    home: "/home/george".into(),
                    shell: "/bin/bash".into()
                }
            ]
        )
    }

    #[test]
    fn groups_array() {
        let doc = r#"
(
    groups: [
        (
            groupname: "me",
            gid: 1,
            members: [
                "someone",
                "someone_else"
            ]
        ),
        (
            groupname: "nobody",
            gid: 65536,
            members: [
                "noone"
            ]
        ),
        (
            groupname: "empty",
            gid: 2,
            members: []
        )
    ],
    users: [],
    packages: [],
    services: {}
)
        "#;

        let out = parse_dpt_file(&doc).unwrap();

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

    #[test]
    fn service_array_1() {
        let doc = r#"
(
    packages: [],
    groups: [],
    users: [],
    services: {
        "multi-user.target": [
            "getty@12.service",
            "getty@4.service",
            "random.socket"
        ],
        "graphical.target": [
            "graphical.socket",
            "stuff.service"
        ]
    }
)
        "#;

        let out = parse_dpt_file(&doc).unwrap();

        assert_eq!(
            out.services,
            Some(HashMap::from([
                (
                    "multi-user.target".to_string(),
                    vec![
                        "getty@12.service".to_string(),
                        "getty@4.service".to_string(),
                        "random.socket".to_string()
                    ]
                ),
                (
                    "graphical.target".to_string(),
                    vec![
                        "graphical.socket".to_string(),
                        "stuff.service".to_string()
                    ]
                )
            ]))
        );
    }
}
