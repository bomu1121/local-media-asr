use crate::{
    AudioExtractArgs, ExportArgs, FileInfo, TaskProgress, TranscriptionArgs, TranscriptionResult,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

// ============================================================
// FFmpeg / Audio extraction commands (Phase 2)
// ============================================================

#[tauri::command]
pub async fn extract_audio(
    args: AudioExtractArgs,
    window: tauri::Window,
) -> Result<String, String> {
    let task_id = uuid::Uuid::new_v4().to_string();

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(&args.input_path);

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
        &args.output_path,
    ]);

    let output = cmd
        .output()
        .map_err(|e| format!("FFmpeg execution failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Audio extraction failed: {}", stderr));
    }

    // Emit progress event
    let _ = window.emit(
        "extract-progress",
        TaskProgress {
            task_id: task_id.clone(),
            stage: "extraction".into(),
            progress: 1.0,
            message: "Audio extracted successfully".into(),
        },
    );

    Ok(args.output_path)
}

#[tauri::command]
pub async fn get_media_info(file_path: String) -> Result<FileInfo, String> {
    let path = std::path::Path::new(&file_path);
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
    let metadata =
        std::fs::metadata(&file_path).map_err(|e| format!("File not accessible: {}", e))?;

    let mut info = FileInfo {
        path: file_path.clone(),
        name,
        size: metadata.len(),
        format: ext,
        duration: None,
        audio_channels: None,
        sample_rate: None,
    };

    // Probe with FFprobe for duration and stream info
    if let Ok(output) = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &file_path,
        ])
        .output()
    {
        if output.status.success() {
            if let Ok(probe) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(dur) = probe["format"]["duration"].as_str() {
                    info.duration = dur.parse::<f64>().ok();
                }
                if let Some(streams) = probe["streams"].as_array() {
                    for stream in streams {
                        if stream["codec_type"] == "audio" {
                            info.audio_channels = stream["channels"].as_i64().map(|c| c as i32);
                            info.sample_rate = stream["sample_rate"].as_str().and_then(|s| s.parse::<i32>().ok());
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(info)
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<String, String> {
    match Command::new("ffmpeg").arg("-version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            Ok(version.lines().next().unwrap_or("FFmpeg found").to_string())
        }
        Ok(output) => Err(format!(
            "FFmpeg not properly installed: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(e) => Err(format!("FFmpeg not found: {}", e)),
    }
}

// ============================================================
// Model management (Phase 3-4)
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub name: String,
    pub installed: bool,
    pub size_bytes: Option<u64>,
    pub required: bool,
}

#[tauri::command]
pub async fn check_models() -> Result<Vec<ModelStatus>, String> {
    // Placeholder - will check model files in resources dir
    Ok(vec![
        ModelStatus {
            name: "SenseVoice-Small (int8)".into(),
            installed: false,
            size_bytes: Some(230_000_000),
            required: true,
        },
        ModelStatus {
            name: "Paraformer-Large (int8)".into(),
            installed: false,
            size_bytes: Some(227_000_000),
            required: false,
        },
        ModelStatus {
            name: "Silero-VAD".into(),
            installed: false,
            size_bytes: Some(1_000_000),
            required: true,
        },
        ModelStatus {
            name: "CT-Transformer Punct".into(),
            installed: false,
            size_bytes: Some(100_000_000),
            required: false,
        },
    ])
}

#[tauri::command]
pub async fn get_app_config() -> Result<crate::AppConfig, String> {
    Ok(crate::AppConfig {
        models_dir: "./models".into(),
        output_dir: "./output".into(),
        ffmpeg_path: "ffmpeg".into(),
        download_mirror: "https://www.modelscope.cn".into(),
    })
}
