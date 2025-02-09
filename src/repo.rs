use indicatif::{ProgressBar, ProgressStyle};
use kdl::{KdlDocument, KdlNode};
use reqwest::blocking::Client;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::pkg::Dependency;
use crate::CONFIG_LOCATION;

#[derive(Debug, PartialEq)]
pub struct OnlinePackage {
    name: String,
    version: String,
    url: String,
    depends: Vec<Dependency>,
}

/// Returns a list of repository's URLs
pub fn get_repositories() -> Result<Vec<String>, Box<dyn Error>> {
    let repos_file_location = Path::new(CONFIG_LOCATION).join("repos");
    let repo_file = match fs::read_to_string(repos_file_location) {
        Ok(x) => x,
        Err(_) => {
            return Err("Failed to read repository list!".into());
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

pub fn fetch_file(url: String) -> Result<Vec<u8>, Box<dyn Error>> {
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
            .template("{bar:40} {percent}% {msg}")?
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

pub fn get_kdl_string_prop(prop_name: &str, node: &KdlNode) -> Result<String, Box<dyn Error>> {
    let name = match node.get(prop_name) {
        Some(x) => x,
        None => {
            return Err(format!(
                "Package specification does not have a {} property!",
                prop_name
            )
            .into())
        }
    };
    let name = match name.as_string() {
        Some(x) => x.to_string(),
        None => return Err(format!("Property {} is not a string!", prop_name).into()),
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

pub fn parse_repository_index(
    index: &str,
    base_url: &str,
) -> Result<Vec<OnlinePackage>, Box<dyn Error>> {
    let doc: KdlDocument = index.parse()?;

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
