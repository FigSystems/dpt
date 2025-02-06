use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use tokio;

/// Displays a progress bar
pub async fn fetch_file_inner(url: &str, local_path: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut res = client.get(url).send().await?.error_for_status()?;

    let total_size = res.content_length().unwrap_or_default();
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("##-"),
    );
    pb.set_message("Downloading...");

    let mut file = File::create(local_path)?;
    let mut downloaded: u64 = 0;

    // let mut stream = res.chunk();
    while let Some(chunk) = res.chunk().await? {
        let chunk = chunk;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }
    pb.finish_with_message("Downloaded!");
    Ok(())
}

pub fn fetch_file(url: &str, local_path: &str) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    match rt.block_on(async { fetch_file_inner(url, local_path).await }) {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to fetch file {}: {}", url, e)),
    }
}
