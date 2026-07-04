use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod commands;
mod ffmpeg;
mod asr;
mod vad;
mod pipeline;
mod models;
mod export;
mod punct;
mod db;
mod download;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioExtractArgs {
    pub input_path: String,
    pub output_path: String,
    pub denoise: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptionArgs {
    pub audio_path: String,
    pub engine_type: String,
    pub use_vad: bool,
    pub use_punctuation: bool,
    pub vad_threshold: f32,
    pub min_speech_duration: f32,
    pub min_silence_duration: f32,
    pub max_segment_duration: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub engine: String,
    pub duration: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskProgress {
    pub task_id: String,
    pub stage: String,
    pub progress: f32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportArgs {
    pub task_id: String,
    pub format: String,
    pub output_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub format: String,
    pub duration: Option<f64>,
    pub audio_channels: Option<i32>,
    pub sample_rate: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub models_dir: String,
    pub output_dir: String,
    pub ffmpeg_path: String,
    pub download_mirror: String,
}

pub const AUDIO_FORMATS: &[&str] = &["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus"];
pub const VIDEO_FORMATS: &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v"];

pub fn is_media_file(path: &PathBuf) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
    AUDIO_FORMATS.contains(&ext.as_str()) || VIDEO_FORMATS.contains(&ext.as_str())
}

fn get_app_data_dir() -> PathBuf {
    dirs_next::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("local-media-asr")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_data_dir = get_app_data_dir();

    tauri::Builder::default()
        .setup(move |_app| {
            let db_path = app_data_dir.join("local-media-asr.db");
            if let Err(e) = db::init(&db_path.to_string_lossy()) {
                eprintln!("Failed to initialize database: {}", e);
            }
            let _ = std::fs::create_dir_all(app_data_dir.join("models"));
            let _ = std::fs::create_dir_all(app_data_dir.join("output"));
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::extract_audio,
            commands::get_media_info,
            commands::check_ffmpeg,
            commands::start_transcription,
            commands::export_result,
            commands::list_history,
            commands::delete_task,
            commands::download_model,
            commands::check_models,
            commands::get_app_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
