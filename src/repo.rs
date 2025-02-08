use indicatif::{ProgressBar, ProgressStyle};
use kdl::KdlDocument;
use reqwest::blocking::Client;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::pkg::Dependency;
use crate::CONFIG_LOCATION;

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
