// Commands module - thin wrappers that delegate to service modules.
// Each command is a #[tauri::command] that validates input,
// calls the sync service function inside spawn_blocking,
// and returns Result<T, String> for the frontend.

use crate::ffmpeg;
use crate::{AudioExtractArgs, FileInfo};
use serde::{Deserialize, Serialize};

// ============================================================
// FFmpeg / Audio extraction commands (Phase 2)
// ============================================================

#[tauri::command]
pub async fn extract_audio(
    args: AudioExtractArgs,
    window: tauri::Window,
) -> Result<String, String> {
    let win = window.clone();
    // Run CPU-bound FFmpeg work on a blocking thread
    tokio::task::spawn_blocking(move || ffmpeg::extract_audio_sync(&args, &win))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_media_info(file_path: String) -> Result<FileInfo, String> {
    tokio::task::spawn_blocking(move || ffmpeg::get_media_info_sync(&file_path))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<String, String> {
    tokio::task::spawn_blocking(ffmpeg::check_ffmpeg_sync)
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| e.to_string())
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
