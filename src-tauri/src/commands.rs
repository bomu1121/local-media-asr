use crate::ffmpeg;
use crate::{AudioExtractArgs, FileInfo};
use serde::{Deserialize, Serialize};

#[tauri::command]
pub async fn extract_audio(args: AudioExtractArgs, window: tauri::Window) -> Result<String, String> {
    let win = window.clone();
    tokio::task::spawn_blocking(move || ffmpeg::extract_audio_sync(&args, &win))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_media_info(file_path: String) -> Result<FileInfo, String> {
    tokio::task::spawn_blocking(move || ffmpeg::get_media_info_sync(&file_path))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<String, String> {
    tokio::task::spawn_blocking(ffmpeg::check_ffmpeg_sync)
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_transcription(audio_path: String, engine_type: String, window: tauri::Window) -> Result<crate::TranscriptionResult, String> {
    use crate::asr::{AsrEngine, EngineType};
    use crate::pipeline;
    use crate::vad::VadConfig;
    let mut engine = AsrEngine::new("./models", EngineType::from_str(&engine_type));
    let _ = engine.load();
    let vad_config = VadConfig::default();
    let win = window.clone();
    tokio::task::spawn_blocking(move || pipeline::transcribe_pipeline(&audio_path, &mut engine, &vad_config, true, &win))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_result(args: crate::ExportArgs, result: crate::TranscriptionResult) -> Result<String, String> {
    tokio::task::spawn_blocking(move || crate::export::export_to_file(&result, &args))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_model(url: String, output_path: String, window: tauri::Window) -> Result<String, String> {
    let win = window.clone();
    let path_clone = output_path.clone();
    tokio::task::spawn_blocking(move || crate::download::download_file(&url, &path_clone, &win))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())?;
    Ok(output_path)
}

#[tauri::command]
pub async fn list_history(limit: i64, offset: i64) -> Result<Vec<crate::db::TaskRecord>, String> {
    tokio::task::spawn_blocking(move || crate::db::list_tasks(limit, offset))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_task(task_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::db::delete_task(&task_id))
        .await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus { pub name: String, pub installed: bool, pub size_bytes: Option<u64>, pub required: bool }

#[tauri::command]
pub async fn check_models() -> Result<Vec<ModelStatus>, String> {
    Ok(vec![
        ModelStatus { name: "SenseVoice-Small (int8)".into(), installed: false, size_bytes: Some(230_000_000), required: true },
        ModelStatus { name: "Paraformer-Large (int8)".into(), installed: false, size_bytes: Some(227_000_000), required: false },
        ModelStatus { name: "Silero-VAD".into(), installed: false, size_bytes: Some(1_000_000), required: true },
        ModelStatus { name: "CT-Transformer Punct".into(), installed: false, size_bytes: Some(100_000_000), required: false },
    ])
}

#[tauri::command]
pub async fn get_app_config() -> Result<crate::AppConfig, String> {
    Ok(crate::AppConfig { models_dir: "./models".into(), output_dir: "./output".into(), ffmpeg_path: "ffmpeg".into(), download_mirror: "https://www.modelscope.cn".into() })
}
