use crate::config::get_config_option;
use crate::pkg::Version;
use anyhow::Context;
use anyhow::{bail, Result};
use indicatif::{ProgressBar, ProgressStyle};
use kdl::{KdlDocument, KdlError, KdlNode};
use pubgrub::OfflineDependencyProvider;
use pubgrub::PubGrubError;
use pubgrub::Ranges;
use pubgrub::{DefaultStringReporter, Reporter};
use reqwest::blocking::Client;
use std::fmt::{self, Display};
use std::fs::DirBuilder;
use std::io::Read;
use std::path::PathBuf;

use crate::pkg::{self, Dependency, Package};
use crate::store::get_store_location;

type VersionSet = Ranges<Version>;

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub struct OnlinePackage {
    pub name: String,
    pub version: String,
    pub url: String,
    pub depends: Vec<Dependency>,
}

impl Display for OnlinePackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "OnlinePackage: {} {} {}",
            self.name, self.version, self.url
        )
    }
}

impl OnlinePackage {
    /// Consumes self
    pub fn to_package(self) -> Package {
        Package {
            name: self.name,
            version: self.version,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum InstallResult {
    Installed,
    Ignored,
}

/// Returns a list of repository's URLs
pub fn get_repositories() -> Result<Vec<String>> {
    let repo_file = get_config_option("repos")
        .context("Failed to read repository list!")?;

    let mut repos: Vec<String> = Vec::new();
    for line in repo_file.lines() {
        if !line.trim().is_empty() {
            repos.push(line.to_string());
        }
    }
    Ok(repos)
}

/// Reads a file from online into a vector of bytes
pub fn fetch_file(url: &str) -> Result<Vec<u8>> {
    let client = Client::new();

    let response = client.get(url).send()?;

    let total_size = match response.content_length() {
        Some(x) => x,
        None => {
            0 // return Err("Server wouldn't tell us what the content length was!".into());
        }
    };

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(crate::PROGRESS_STYLE_BYTES)?
            .progress_chars(crate::PROGRESS_CHARS),
    );
    pb.set_message(format!("{}", url));

    let mut buffer = Vec::new();

    let mut reader = response; // .take(total_size);
    let mut chunk = [0u8; 4096];
    let mut downloaded = 0;

    while let Ok(bytes_read) = reader.read(&mut chunk) {
        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);

        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!("{}", url));

    Ok(buffer)
}

/// Finds a string property that is a subnode of `node`
pub fn get_kdl_string_prop(prop_name: &str, node: &KdlNode) -> Result<String> {
    let name = match node.get(prop_name) {
        Some(x) => x,
        None => {
            bail!(
                "Package specification does not have a {} property!",
                prop_name
            );
        }
    };
    let name = match name.as_string() {
        Some(x) => x.to_string(),
        None => bail!("Property {} is not a string!", prop_name),
    };
    Ok(name)
}

/// Adds a component onto the end of a URL
pub fn push_onto_url(base: &str, ext: &str) -> String {
    if base.chars().last() == Some('/') || ext.chars().next() == Some('/') {
        base.to_owned() + ext
    } else {
        base.to_owned() + "/" + ext
    }
}

/// Parses a repositories index file into an array of OnlinePackages
pub fn parse_repository_index(
    index: &str,
    base_url: &str,
) -> Result<Vec<OnlinePackage>> {
    let doc: Result<KdlDocument, KdlError> = index.parse();
    if let Err(e) = doc {
        let diagnostics = e
            .diagnostics
            .into_iter()
            .map(|x| {
                let a = x.to_string();
                let b = x.help.unwrap_or("None".to_string());
                format!("{} help: {}\n", a, b)
            })
            .collect::<Vec<String>>()
            .concat();
        bail!(
            "Failed to parse KDL document: {}\n\n diagnostics: \n{}",
            index,
            diagnostics
        );
    }
    let doc = doc.unwrap();

    let mut ret: Vec<OnlinePackage> = Vec::new();
    for pkg in doc.nodes() {
        if pkg.name().to_string() != "package" {
            continue;
        }

        let name = get_kdl_string_prop("name", pkg)?;
        let version = get_kdl_string_prop("version", pkg)?;
        let url =
            push_onto_url(base_url, get_kdl_string_prop("path", pkg)?.as_str());

        let children = pkg.children();

        let mut depends: Vec<Dependency> = Vec::new();

        if let Some(document) = children {
            depends = crate::pkg::parse_depends(&document)?;
        }
        ret.push(OnlinePackage {
            name,
            version,
            url,
            depends,
        });
    }
    Ok(ret)
}

/// Get all packages that are available on all repositories
pub fn get_all_available_packages() -> Result<Vec<OnlinePackage>> {
    let repos = get_repositories()?;

    let mut ret: Vec<OnlinePackage> = Vec::new();
    for repo in repos {
        let index = fetch_file(&push_onto_url(repo.as_str(), "index.kdl"))?;
        let index = std::str::from_utf8(&index)?;
        let mut packages = parse_repository_index(index, &repo)?;
        ret.append(&mut packages);
    }

    Ok(ret)
}

/// Parse a version range from a string
pub fn parse_version_range(vr: &str) -> Result<Ranges<Version>> {
    Ok(if vr.len() < 5 {
        VersionSet::full()
    } else if vr.chars().next() == Some('>') {
        if vr.chars().nth(1).unwrap() == '=' {
            VersionSet::higher_than(Version::from_str(&vr[2..])?)
        } else {
            Ranges::higher_than(Version::from_str(&vr[1..])?.bump())
        }
    } else {
        let v = Version::from_str(&vr)?;
        VersionSet::singleton(v)
    })
}

/// Get the dependency provider structure for the vector of packages passed in.
pub fn get_dependency_provider_for_packages(
    packages: &Vec<OnlinePackage>,
) -> Result<OfflineDependencyProvider<String, VersionSet>> {
    let mut ret = OfflineDependencyProvider::<String, VersionSet>::new();

    for pkg in packages {
        let mut depends = Vec::<(String, VersionSet)>::new();
        for dep in &pkg.depends {
            let version = parse_version_range(&dep.version_mask)?;

            depends.push((dep.name.clone(), version));
        }

        ret.add_dependencies(
            pkg.name.clone(),
            Version::from_str(pkg.version.as_str())?,
            depends,
        );
    }

    Ok(ret)
}

/// Converts by looping through the package list to find a match. Short circuted
pub fn package_to_onlinepackage(
    package: &Package,
    packages: &Vec<OnlinePackage>,
) -> Result<OnlinePackage> {
    for pkg in packages {
        if pkg.name == package.name
            && Version::from_str(&pkg.version)
                .context("Failed to iterate through packages")?
                == Version::from_str(&package.version)
                    .context("Faield to iterate through packages")?
        {
            return Ok(pkg.clone());
        }
    }

    bail!("Package {:?} not found", package)
}

/// Finds the newest package matching the name in the array of packages
pub fn newest_package_from_name(
    package: &str,
    packages: &Vec<OnlinePackage>,
) -> Result<OnlinePackage> {
    let mut newest_version: Option<Version> = None;
    let mut newest_package: Option<OnlinePackage> = None;
    for pkg in packages {
        if pkg.name == package {
            let greator = if let Some(x) = &newest_version {
                &Version::from_str(&pkg.version)? > x
            } else {
                true
            };
            if greator {
                newest_version = Some(Version::from_str(&pkg.version)?);
                newest_package = Some(pkg.clone());
            }
        }
    }
    match newest_package {
        Some(x) => Ok(x),
        None => bail!("Package '{package}' not found"),
    }
}

/// Finds all of the packages that are required to install this package
pub fn resolve_dependencies_for_package(
    packages: &Vec<OnlinePackage>,
    package: &Package,
) -> Result<Vec<OnlinePackage>> {
    let dependency_provider = get_dependency_provider_for_packages(&packages)?;
    package_to_onlinepackage(&package, &packages)?; // Verify that the package exits in the package vec

    let resolved = pubgrub::resolve(
        &dependency_provider,
        package.name.clone(),
        Version::from_str(package.version.as_str())?,
    );

    let resolved = match resolved {
        Ok(solution) => solution,
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_no_versions();
            bail!("{}", DefaultStringReporter::report(&derivation_tree));
        }
        Err(err) => bail!("{:?}", err),
    };

    let mut ret = Vec::<OnlinePackage>::new();

    // Locate actual online packages from the resulting package list
    for (name, version) in resolved {
        ret.push(package_to_onlinepackage(
            &Package {
                name,
                version: version.to_string(),
            },
            &packages,
        )?)
    }
    Ok(ret)
}

/// Install a single package into the pool. Does NOT handle dependencies
pub fn install_pkg(
    pkg: &OnlinePackage,
    reinstall: bool,
) -> Result<InstallResult> {
    let store = get_store_location();
    if !store.is_dir() {
        DirBuilder::new().recursive(true).create(&store)?;
    }

    let out_path: PathBuf = store.join(pkg.name.clone() + "-" + &pkg.version);

    if out_path.exists() {
        if reinstall {
            std::fs::remove_dir_all(&out_path)?;
        } else {
            return Ok(InstallResult::Ignored);
        }
    }

    let file = fetch_file(&pkg.url)?;

    let mut archive = pkg::decompress_pkg_read(&file[..])?;

    archive.unpack(&out_path)?;

    Ok(InstallResult::Installed)
}

/// Install a package and all of it's dependencies into the pool
pub fn install_pkg_and_dependencies(
    pkg: &OnlinePackage,
    pkgs: &Vec<OnlinePackage>,
    done_list: &mut Vec<(OnlinePackage, InstallResult)>,
    reinstall: bool,
) -> Result<()> {
    let ires = install_pkg(pkg, reinstall)?;
    done_list.push((pkg.clone(), ires));
    let dependencies =
        resolve_dependencies_for_package(&pkgs, &pkg.clone().to_package())?;

    for depends in dependencies {
        if done_list
            .iter()
            .map(|x| &x.0)
            .collect::<Vec<&OnlinePackage>>()
            .contains(&&depends)
        {
            continue;
        }
        install_pkg_and_dependencies(&depends, pkgs, done_list, false)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_repository_index_1() {
        let index = r###"
package name=test version="9.11.14" path="/test.dpt"
package name=example version="1.2.3" path="my-pkg.dpt" {
    depends example1
    depends example2 version="^10.2.0"
}
            "###;
        let x =
            parse_repository_index(index, "https://my.repo.here/dpt").unwrap();
        let expected: Vec<OnlinePackage> = vec![
            OnlinePackage {
                name: "test".to_string(),
                version: "9.11.14".to_string(),
                url: "https://my.repo.here/dpt/test.dpt".to_string(),
                depends: Vec::<Dependency>::new(),
            },
            OnlinePackage {
                name: "example".to_string(),
                version: "1.2.3".to_string(),
                url: "https://my.repo.here/dpt/my-pkg.dpt".to_string(),
                depends: vec![
                    Dependency {
                        name: "example1".to_string(),
                        version_mask: "".to_string(),
                    },
                    Dependency {
                        name: "example2".to_string(),
                        version_mask: "^10.2.0".to_string(),
                    },
                ],
            },
        ];

        assert_eq!(x, expected);
    }

    #[test]
    fn resolve_1() {
        let packages = vec![
            OnlinePackage {
                name: "1".to_string(),
                version: "1.2.3".to_string(),
                url: "https://my.repo.pkg/dpt/1.dpt".to_string(),
                depends: vec![],
            },
            OnlinePackage {
                name: "2".to_string(),
                version: "4.5.6".to_string(),
                url: "https://my.repo.pkg/dpt/2.dpt".to_string(),
                depends: vec![Dependency {
                    name: "1".to_string(),
                    version_mask: ">=1.0.0".to_string(),
                }],
            },
            OnlinePackage {
                name: "goal".to_string(),
                version: "7.8.9".to_string(),
                url: "https://my.repo.pkg/dpt/goal.dpt".to_string(),
                depends: vec![Dependency {
                    name: "2".to_string(),
                    version_mask: ">4.5.0".to_string(),
                }],
            },
        ];

        let resolved = resolve_dependencies_for_package(
            &packages,
            &Package {
                name: "goal".to_string(),
                version: "7.8.9".to_string(),
            },
        )
        .unwrap();

        assert_eq!(resolved.len(), 3);

        for pkg in resolved {
            assert!(packages.contains(&pkg.clone()));
        }
    }
}
