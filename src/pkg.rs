use anyhow::{anyhow, bail, Context, Result};
use ron;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{self, Display},
    io::BufRead,
};
use tar::Archive;

#[derive(Debug, Clone, Hash, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
}

impl Package {
    pub fn new(name: String, version: String) -> Package {
        Package { name, version }
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Package: {} {}", self.name, self.version)
    }
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Dependency) -> bool {
        self.name == other.name && self.version == other.version
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Glue {
    Bin,
    Glob(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub depends: Vec<Dependency>,
    pub glue: Vec<Glue>,
}

impl PartialEq for PackageConfig {
    fn eq(&self, other: &PackageConfig) -> bool {
        self.name == other.name
            && self.version == other.version
            && self.depends == other.depends
    }
}

#[derive(PartialEq, Debug, Clone, Eq, Serialize, Deserialize)]
pub struct Version {
    n: Vec<u32>,
}

impl Version {
    pub fn new(n: Vec<u32>) -> Self {
        Version { n }
    }
    pub fn from_str(s: &str) -> Result<Self> {
        let str_split = s.split(".").into_iter().collect::<Vec<&str>>();
        if str_split.len() < 1 {
            bail!("Empty version strings are invalid!");
        }
        let mut n: Vec<u32> = Vec::new();
        for i in 0..str_split.len() {
            n.push(index_or_err_str(&str_split, i)?.parse()?);
        }
        Ok(Version { n })
    }

    pub fn bump(&self) -> Self {
        let len = self.n.len();
        let mut n = self.n.clone();
        n[len - 1] += 1;
        Version { n }
    }

    #[allow(dead_code)]
    pub fn zero() -> Self {
        Version::new(vec![0])
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let mut i = 0;
        loop {
            let mine = self.n.get(i);
            let theirs = other.n.get(i);
            if mine.is_none() || theirs.is_none() {
                return std::cmp::Ordering::Equal;
            }
            let mine = mine.unwrap();
            let theirs = theirs.unwrap();

            if mine > theirs {
                return std::cmp::Ordering::Greater;
            } else if mine < theirs {
                return std::cmp::Ordering::Less;
            }
            i += 1;
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut i = 0;
        loop {
            let mine = self.n.get(i);
            let theirs = other.n.get(i);
            if mine.is_none() || theirs.is_none() {
                return Some(Ordering::Equal);
            }
            let mine = mine.unwrap();
            let theirs = theirs.unwrap();

            if mine > theirs {
                return Some(Ordering::Greater);
            } else if mine < theirs {
                return Some(Ordering::Less);
            }
            i += 1;
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        if self.n.len() < 1 {
            return write!(f, "0");
        }
        s.push_str(&self.n[0].to_string());
        for (i, e) in self.n.iter().enumerate() {
            if i.to_owned() == 0 {
                continue;
            }
            s.push_str(&format!(".{}", e.to_string()));
        }
        write!(f, "{}", s)
    }
}

pub fn index_or_err_str(s: &Vec<&str>, i: usize) -> Result<String> {
    Ok((s
        .to_owned()
        .get(i as usize)
        .ok_or(anyhow!("Failed to get index {i}, from string {:#?}", s))?)
    .to_string())
}

/// Parses the package configuration and bails if not valid.
pub fn get_package_config(file: &str) -> Result<PackageConfig> {
    Ok(ron::from_str(file)?)
}

/// Parses the name and version from a string.
pub fn string_to_package(s: &str) -> Result<Package> {
    let version = s
        .split("-")
        .last()
        .ok_or(anyhow!("Failed to parse version from string {}", s))?;
    Version::from_str(version)
        .context(anyhow!("Failed to parse version string {}", version))?;
    let name: Vec<String> = s.split("-").map(|x| x.to_string() + "-").collect(); // Collect each segment, adding a '-' after each
    let name = &name[..name.len() - 1].concat(); // Concat each segment
    let name = &name[..name.len() - 1]; // Trim off last '-'
    Ok(Package {
        name: name.to_string(),
        version: version.to_string(),
    })
}

/// Decompresses a package from something implementing std::io::Read
pub fn decompress_pkg_read<'a>(
    pkg: impl std::io::Read,
) -> Result<Archive<zstd::Decoder<'a, impl BufRead>>> {
    let zstrm = zstd::Decoder::new(pkg)?;

    let mut archive = tar::Archive::new(zstrm);
    archive.set_unpack_xattrs(true);
    archive.set_preserve_permissions(true);
    archive.set_preserve_ownerships(true);
    archive.set_overwrite(true);

    Ok(archive)
}

#[cfg(test)]
mod tests {
    use crate::pkg::string_to_package;

    use super::*;

    #[test]
    fn get_pkg_config_1() {
        let s = r###"
(
    name: "abcd",
    version: "145.54.12",

    depends: [
        (
            name: "coreutils",
            version: "",
        ),
        (
            name: "python",
            version: "^8.9.112"
        )
    ],

    glue: [
        Bin,
        Glob([
            "/usr/lib/systemd/system/*.service",
            "/usr/lib/systemd/system/*.socket"
        ])
    ]
)
"###;
        let expected = PackageConfig {
            name: "abcd".to_string(),
            version: "145.54.12".to_string(),
            depends: vec![
                Dependency {
                    name: "coreutils".to_string(),
                    version: "".to_string(),
                },
                Dependency {
                    name: "python".to_string(),
                    version: "^8.9.112".to_string(),
                },
            ],
            glue: vec![
                Glue::Bin,
                Glue::Glob(vec![
                    "/usr/lib/systemd/system/*.service".to_string(),
                    "/usr/lib/systemd/system/*.socket".to_string(),
                ]),
            ],
        };
        let x = get_package_config(s).unwrap();
        assert_eq!(x, expected);
    }

    #[test]
    fn string_to_package_1() {
        assert_eq!(
            string_to_package("a-b-c-d-e-1.2.3").unwrap(),
            Package {
                name: "a-b-c-d-e".to_string(),
                version: "1.2.3".to_string()
            }
        );
        assert_eq!(
            string_to_package("testing-123-0.4.3").unwrap(),
            Package {
                name: "testing-123".to_string(),
                version: "0.4.3".to_string()
            }
        );
    }

    #[test]
    fn test_version_from_str() {
        assert_eq!(
            Version::from_str("0.0.0").unwrap(),
            Version::new(vec![0, 0, 0])
        );
        assert_eq!(
            Version::from_str("1.2.3").unwrap(),
            Version::new(vec![1, 2, 3])
        );
        assert_eq!(
            Version::from_str("300.22.11.00").unwrap(),
            Version::new(vec![300, 22, 11, 0])
        );
        assert_eq!(
            Version::from_str("12.11").unwrap(),
            Version::new(vec![12, 11])
        );
        assert_eq!(Version::from_str("531").unwrap(), Version::new(vec![531]));
        assert_eq!(
            Version::from_str("13.1").unwrap(),
            Version::new(vec![13, 1])
        );
    }

    #[test]
    fn test_version_cmp() {
        assert!(
            Version::from_str("531").unwrap() > Version::new(vec![0, 531, 0])
        );
        assert!(Version::new(vec![0, 1, 2]) > Version::new(vec![0, 1, 1]));
        assert!(Version::new(vec![6, 5, 4]) > Version::new(vec![0, 22, 500]));
        assert!(
            Version::new(vec![98, 54, 97, 100])
                == Version::new(vec![98, 54, 97, 100])
        );
        assert!(
            Version::new(vec![98, 54, 97]) >= Version::new(vec![98, 54, 97])
        );
        assert!(
            Version::new(vec![0, 0, 0, 2]) < Version::new(vec![0, 0, 0, 3])
        );
        assert!(!(Version::new(vec![0, 0, 2]) < Version::new(vec![0, 0, 2])));
    }

    #[test]
    pub fn test_version_invalid() {
        Version::from_str("").expect_err("Input was ''");
        Version::from_str("45a.22").expect_err("Input was '45a.22'");
    }
}
