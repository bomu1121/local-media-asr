use crate::{AudioExtractArgs, FileInfo, TaskProgress};
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use tauri::Emitter;

/// Duration regex: matches ffmpeg "Duration: HH:MM:SS.ms" output
fn parse_duration(line: &str) -> Option<f64> {
    let prefix = "Duration: ";
    let pos = line.find(prefix)?;
    let rest = &line[pos + prefix.len()..];
    let time_str = rest.split(',').next()?;
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 3 {
        let h: f64 = parts[0].parse().ok()?;
        let m: f64 = parts[1].parse().ok()?;
        let s: f64 = parts[2].parse().ok()?;
        Some(h * 3600.0 + m * 60.0 + s)
    } else {
        None
    }
}

/// Time regex: matches ffmpeg "time=HH:MM:SS.ms" progress output
fn parse_time(line: &str) -> Option<f64> {
    let prefix = "time=";
    let pos = line.find(prefix)?;
    let rest = &line[pos + prefix.len()..];
    let time_str = rest.split_whitespace().next()?;
    parse_hms(time_str)
}

fn parse_hms(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let h: f64 = parts[0].parse().ok()?;
        let m: f64 = parts[1].parse().ok()?;
        let s: f64 = parts[2].parse().ok()?;
        Some(h * 3600.0 + m * 60.0 + s)
    } else {
        None
    }
}

/// Extract audio from a media file using FFmpeg.
/// Converts to 16kHz mono PCM WAV, with optional noise reduction.
/// Reports progress in real-time via Tauri events.
pub fn extract_audio_sync(
    args: &AudioExtractArgs,
    window: &tauri::Window,
) -> Result<String> {
    let input_path = &args.input_path;
    let output_path = &args.output_path;

    // Ensure output directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create output directory")?;
    }

    // First pass: probe duration via ffprobe
    let total_duration = probe_duration(input_path);

    // Emit starting event
    let task_id = uuid::Uuid::new_v4().to_string();
    let _ = window.emit(
        "extract-progress",
        TaskProgress {
            task_id: task_id.clone(),
            stage: "extraction".into(),
            progress: 0.0,
            message: "Starting audio extraction...".into(),
        },
    );

    // Build ffmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(input_path)
        .arg("-progress")
        .arg("pipe:1")
        .arg("-nostats");

    if args.denoise {
        cmd.arg("-af").arg("afftdn=nr=15:nf=-25:tn=1");
    }

    cmd.args([
        "-ac",
        "1",
        "-ar",
        "16000",
        "-acodec",
        "pcm_s16le",
        "-y",
        output_path,
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("Failed to start FFmpeg")?;

    let stderr = child
        .stderr
        .take()
        .context("Failed to capture FFmpeg stderr")?;
    let stdout = child
        .stdout
        .take()
        .context("Failed to capture FFmpeg stdout")?;

    // Read progress from stdout (ffmpeg -progress pipe:1)
    let duration = total_duration;
    let window_clone = window.clone();
    let tid = task_id.clone();

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(time) = parse_time(&line) {
                if let Some(dur) = duration {
                    let progress = (time / dur).min(1.0);
                    let _ = window_clone.emit(
                        "extract-progress",
                        TaskProgress {
                            task_id: tid.clone(),
                            stage: "extraction".into(),
                            progress: progress as f32,
                            message: format!(
                                "Extracting audio... {:.0}%",
                                progress * 100.0
                            ),
                        },
                    );
                }
            }
        }
    });

    let output = child.wait().context("FFmpeg process failed")?;

    if !output.success() {
        let stderr_output = BufReader::new(stderr)
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow::anyhow!(
            "FFmpeg exited with error:\n{}",
            stderr_output
        ));
    }

    // Verify output exists
    if !Path::new(output_path).exists() {
        return Err(anyhow::anyhow!("Output file was not created"));
    }

    // Emit completion event
    let _ = window.emit(
        "extract-progress",
        TaskProgress {
            task_id: task_id.clone(),
            stage: "extraction".into(),
            progress: 1.0,
            message: "Audio extraction complete".into(),
        },
    );

    Ok(output_path.clone())
}

/// Probe media file duration using ffprobe (quick, JSON-based)
fn probe_duration(file_path: &str) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            file_path,
        ])
        .output()
        .ok()?;

    if output.status.success() {
        if let Ok(probe) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            return probe["format"]["duration"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
        }
    }
    None
}

/// Get detailed media file information using ffprobe
pub fn get_media_info_sync(file_path: &str) -> Result<FileInfo> {
    let path = Path::new(file_path);
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let metadata = std::fs::metadata(file_path)
        .context("File not accessible")?;

    let mut info = FileInfo {
        path: file_path.to_string(),
        name,
        size: metadata.len(),
        format: ext,
        duration: None,
        audio_channels: None,
        sample_rate: None,
    };

    // Probe with ffprobe for detailed stream info
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            file_path,
        ])
        .output()
        .context("Failed to run ffprobe")?;

    if output.status.success() {
        if let Ok(probe) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(dur) = probe["format"]["duration"].as_str() {
                info.duration = dur.parse::<f64>().ok();
            }
            if let Some(streams) = probe["streams"].as_array() {
                for stream in streams {
                    if stream["codec_type"] == "audio" {
                        info.audio_channels =
                            stream["channels"].as_i64().map(|c| c as i32);
                        info.sample_rate =
                            stream["sample_rate"]
                                .as_str()
                                .and_then(|s| s.parse::<i32>().ok());
                        break;
                    }
                }
            }
        }
    }

    Ok(info)
}

/// Check if FFmpeg is available and return version string
pub fn check_ffmpeg_sync() -> Result<String> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("FFmpeg not found. Please install FFmpeg or use the sidecar bundle.")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version
            .lines()
            .next()
            .unwrap_or("FFmpeg found")
            .to_string())
    } else {
        Err(anyhow::anyhow!(
            "FFmpeg returned error: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
