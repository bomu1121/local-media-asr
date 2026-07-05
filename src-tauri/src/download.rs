// Model download manager with progress reporting
// Downloads model files from URLs with resume support and Tauri event progress

use crate::TaskProgress;
use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use tauri::Emitter;

/// Download a file from a URL with progress callbacks via Tauri events.
/// Supports resuming interrupted downloads via Range header.
pub fn download_file(
    url: &str,
    output_path: &str,
    window: &tauri::Window,
) -> Result<()> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let file_name = Path::new(output_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Ensure output directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).context("Failed to create download directory")?;
    }

    // Check existing file size for resume
    let existing_size = if Path::new(output_path).exists() {
        fs::metadata(output_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout_read(std::time::Duration::from_secs(600))
        .redirects(5)
        .build();

    // Probe the URL: verify it returns binary content, not an HTML page
    let total_size = {
        let resp = agent
            .head(url)
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .call()
            .ok();
        resp.and_then(|r| r.header("content-length").and_then(|v| v.parse::<u64>().ok()))
            .unwrap_or(0)
    };

    // If server doesn't give content-length via HEAD, do a quick GET probe
    let is_valid = if total_size == 0 {
        match agent
            .get(url)
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .set("Accept", "application/octet-stream, */*")
            .call()
        {
            Ok(resp) => {
                let mut reader = resp.into_reader();
                let mut probe = [0u8; 4];
                let n = reader.read(&mut probe).unwrap_or(0);
                // Check if response is HTML (starts with <) - if so, the URL needs auth
                if n > 0 && probe[0] == b'<' {
                    return Err(anyhow::anyhow!(
                        "Download URL returned HTML instead of binary data. The file may require authentication or the URL may be incorrect."
                    ));
                }
                // Probe passed — but we consumed the response. Fall through to re-download.
                // For models < probe size, we already have them
                if n < 4 { probe[0] != b'<' } else { true }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to connect to download server: {}", e));
            }
        }
    } else {
        true
    };

    if !is_valid {
        return Err(anyhow::anyhow!("Download source returned unexpected content"));
    }

    // If file already complete, skip download
    if existing_size > 0 && total_size > 0 && existing_size >= total_size {
        let _ = window.emit(
            "download-progress",
            TaskProgress {
                task_id: task_id.clone(),
                stage: "download".into(),
                progress: 1.0,
                message: format!("{} already downloaded", file_name),
            },
        );
        return Ok(());
    }

    // Start download with proper headers
    let req = if existing_size > 0 {
        agent
            .get(url)
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .set("Accept", "application/octet-stream, */*")
            .set("Range", &format!("bytes={}-", existing_size))
    } else {
        agent
            .get(url)
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .set("Accept", "application/octet-stream, */*")
    };

    let resp = req.call().context("Failed to connect to download server")?;

    // Verify response is valid binary
    let content_type = resp.header("content-type").unwrap_or("");
    if content_type.contains("text/html") {
        return Err(anyhow::anyhow!(
            "Server returned HTML instead of file data — the download source may require authentication. Please visit {} in your browser to download the file manually.",
            url
        ));
    }

    let content_length = resp
        .header("content-length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let effective_total = if total_size > 0 { total_size } else { existing_size + content_length };

    // Open file for appending (resume) or create new
    let mut file = if existing_size > 0 {
        OpenOptions::new()
            .append(true)
            .open(output_path)
            .context("Failed to open file for resume")?
    } else {
        File::create(output_path).context("Failed to create download file")?
    };

    // Read response body in chunks and report progress
    let mut reader = resp.into_reader();
    let mut buffer = [0u8; 65536]; // 64KB chunks
    let mut downloaded = existing_size;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        let progress = if effective_total > 0 {
            (downloaded as f64 / effective_total as f64) as f32
        } else {
            0.5 // Unknown size
        };

        let _ = window.emit(
            "download-progress",
            TaskProgress {
                task_id: task_id.clone(),
                stage: "download".into(),
                progress,
                message: format!(
                    "Downloading {}... {:.1}%",
                    file_name,
                    progress * 100.0
                ),
            },
        );
    }

    // Final completion event
    let _ = window.emit(
        "download-progress",
        TaskProgress {
            task_id,
            stage: "download".into(),
            progress: 1.0,
            message: format!("{} download complete", file_name),
        },
    );

    Ok(())
}