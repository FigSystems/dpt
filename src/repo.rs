use anyhow::{bail, Result};
use indicatif::{ProgressBar, ProgressStyle};
use kdl::{KdlDocument, KdlError, KdlNode};
use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::OfflineDependencyProvider;
use pubgrub::version::SemanticVersion;
use reqwest::blocking::Client;
use std::fs::{self, DirBuilder};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::pkg::{self, Dependency, Package};
use crate::pool::get_pool_location;
use crate::CONFIG_LOCATION;

#[derive(Debug, PartialEq, Clone)]
pub struct OnlinePackage {
    pub name: String,
    pub version: String,
    pub url: String,
    pub depends: Vec<Dependency>,
}

/// Returns a list of repository's URLs
pub fn get_repositories() -> Result<Vec<String>> {
    let repos_file_location = Path::new(CONFIG_LOCATION).join("repos");
    let repo_file = match fs::read_to_string(repos_file_location) {
        Ok(x) => x,
        Err(_) => {
            bail!("Failed to read repository list!");
        }
    };

    let mut repos: Vec<String> = Vec::new();
    for line in repo_file.lines() {
        if !line.trim().is_empty() {
            repos.push(line.to_string());
        }
    }
    Ok(repos)
}

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
            .template("{msg} [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("##-"),
    );

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

    pb.finish_with_message("Finished download!");

    Ok(buffer)
}

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

pub fn push_onto_url(base: &str, ext: &str) -> String {
    if base.chars().last() == Some('/') || ext.chars().next() == Some('/') {
        base.to_owned() + ext
    } else {
        base.to_owned() + "/" + ext
    }
}

pub fn parse_repository_index(index: &str, base_url: &str) -> Result<Vec<OnlinePackage>> {
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
        let url = push_onto_url(base_url, get_kdl_string_prop("path", pkg)?.as_str());

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

pub fn parse_version_range(vr: &str) -> Result<Range<SemanticVersion>> {
    Ok(if vr.len() < 1 {
        Range::any()
    } else if vr.chars().next() == Some('^') {
        let v = SemanticVersion::from_str(&vr[1..])?;
        Range::between(v, v.bump_major())
    } else if vr.chars().next() == Some('~') {
        let v = SemanticVersion::from_str(&vr[1..])?;
        Range::between(v, v.bump_minor())
    } else {
        let v = SemanticVersion::from_str(&vr)?;
        Range::exact(v)
    })
}

pub fn get_dependency_provider_for_packages(
    packages: &Vec<OnlinePackage>,
) -> Result<OfflineDependencyProvider<String, SemanticVersion>> {
    let mut ret = OfflineDependencyProvider::<String, SemanticVersion>::new();

    for pkg in packages {
        let mut depends = Vec::<(String, Range<SemanticVersion>)>::new();
        for dep in &pkg.depends {
            let version = parse_version_range(&dep.version_mask)?;

            depends.push((dep.name.clone(), version));
        }

        ret.add_dependencies(
            pkg.name.clone(),
            SemanticVersion::from_str(pkg.version.as_str())?,
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
        if pkg.name == package.name && pkg.version == package.version {
            return Ok(pkg.clone());
        }
    }

    bail!("Package not found")
}

pub fn newest_package_from_name(
    package: &str,
    packages: &Vec<OnlinePackage>,
) -> Result<OnlinePackage> {
    let mut newest_version = SemanticVersion::zero();
    let mut newest_package: Option<OnlinePackage> = None;
    for pkg in packages {
        if pkg.name == package {
            if SemanticVersion::from_str(&pkg.version)? > newest_version {
                newest_version = SemanticVersion::from_str(&pkg.version)?;
                newest_package = Some(pkg.clone());
            }
        }
    }
    match newest_package {
        Some(x) => Ok(x),
        None => bail!("Package not found"),
    }
}

pub fn resolve_dependencies_for_package(
    packages: &Vec<OnlinePackage>,
    package: Package,
) -> Result<Vec<OnlinePackage>> {
    let dependency_provider = get_dependency_provider_for_packages(&packages)?;
    package_to_onlinepackage(&package, &packages)?; // Verify that the package exits in the package vec

    let resolved = pubgrub::solver::resolve(
        &dependency_provider,
        package.name.clone(),
        SemanticVersion::from_str(package.version.as_str())?,
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

pub fn install_pkg(pkg: &OnlinePackage) -> Result<PathBuf> {
    let pool = get_pool_location();
    if !pool.is_dir()
    /* i.e. exists  */
    {
        DirBuilder::new().recursive(true).create(&pool)?;
    }

    let file = fetch_file(&pkg.url)?;

    let mut archive = pkg::decompress_pkg_read(&file[..])?; // Moves file

    let out_path: PathBuf = pool.join(pkg.name.clone() + "-" + &pkg.version);

    archive.unpack(&out_path)?;

    Ok(out_path)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_repository_index_1() {
        let index = r###"
package name=test version="9.11.14" path="/test.fpkg"
package name=example version="1.2.3" path="my-pkg.fpkg" {
    depends example1
    depends example2 {
        version "^10.2.0"
    }
}
            "###;
        let x = parse_repository_index(index, "https://my.repo.here/fpkg").unwrap();
        let expected: Vec<OnlinePackage> = vec![
            OnlinePackage {
                name: "test".to_string(),
                version: "9.11.14".to_string(),
                url: "https://my.repo.here/fpkg/test.fpkg".to_string(),
                depends: Vec::<Dependency>::new(),
            },
            OnlinePackage {
                name: "example".to_string(),
                version: "1.2.3".to_string(),
                url: "https://my.repo.here/fpkg/my-pkg.fpkg".to_string(),
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
                url: "https://my.repo.pkg/fpkg/1.fpkg".to_string(),
                depends: vec![],
            },
            OnlinePackage {
                name: "2".to_string(),
                version: "4.5.6".to_string(),
                url: "https://my.repo.pkg/fpkg/2.fpkg".to_string(),
                depends: vec![Dependency {
                    name: "1".to_string(),
                    version_mask: "^1.0.0".to_string(),
                }],
            },
            OnlinePackage {
                name: "goal".to_string(),
                version: "7.8.9".to_string(),
                url: "https://my.repo.pkg/fpkg/goal.fpkg".to_string(),
                depends: vec![Dependency {
                    name: "2".to_string(),
                    version_mask: "~4.5.0".to_string(),
                }],
            },
        ];

        let resolved = resolve_dependencies_for_package(
            &packages,
            Package {
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
