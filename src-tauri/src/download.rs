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

    // Get remote file size via HEAD request
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(30))
        .timeout_read(std::time::Duration::from_secs(300))
        .build();

    // First, check total size
    let total_size = match agent.head(url).call() {
        Ok(resp) => resp
            .header("content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0),
        Err(_) => 0,
    };

    // If file already complete, skip download
    if existing_size > 0 && existing_size == total_size {
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

    // Start download request with optional Range header
    let req = if existing_size > 0 {
        agent
            .get(url)
            .set("Range", &format!("bytes={}-", existing_size))
    } else {
        agent.get(url)
    };

    let resp = req.call().context("Failed to connect to download server")?;

    let content_length = resp
        .header("content-length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let effective_total = existing_size + content_length;

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
