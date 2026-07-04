use crate::ffmpeg;
use crate::{AudioExtractArgs, FileInfo};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[tauri::command] pub async fn extract_audio(args: AudioExtractArgs, window: tauri::Window) -> Result<String, String> {
    let win = window.clone();
    tokio::task::spawn_blocking(move || ffmpeg::extract_audio_sync(&args, &win)).await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}
#[tauri::command] pub async fn get_media_info(file_path: String) -> Result<FileInfo, String> {
    tokio::task::spawn_blocking(move || ffmpeg::get_media_info_sync(&file_path)).await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}
#[tauri::command] pub async fn check_ffmpeg() -> Result<String, String> {
    tokio::task::spawn_blocking(ffmpeg::check_ffmpeg_sync).await.map_err(|e| format!("Task join error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn process_media(file_path: String, engine_type: String, window: tauri::Window) -> Result<crate::TranscriptionResult, String> {
    use crate::asr::{AsrEngine, EngineType};
    use crate::pipeline;
    use crate::vad::VadConfig;

    let fp_save = file_path.clone();
    let eng_save = engine_type.clone();
    let audio_exts = ["wav", "mp3", "flac", "aac", "ogg", "wma", "m4a", "opus"];
    let ext = Path::new(&file_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let is_wav = ext == "wav";

    let wav_path = if is_wav { file_path.clone() } else {
        let parent = Path::new(&file_path).parent().and_then(|p| p.to_str()).unwrap_or(".");
        let base = Path::new(&file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        format!("{}\\{}_extracted.wav", parent, base)
    };

    if !is_wav {
        let args = crate::AudioExtractArgs { input_path: file_path, output_path: wav_path.clone(), denoise: false };
        let wc = window.clone();
        tokio::task::spawn_blocking(move || ffmpeg::extract_audio_sync(&args, &wc))
            .await.map_err(|e| format!("Join: {}", e))?.map_err(|e| format!("Extract: {}", e))?;
    }

    let mut engine = AsrEngine::new("./models", EngineType::from_str(&engine_type));
    engine.load().map_err(|e| format!("ASR load: {}", e))?;
    let vad_config = VadConfig::default();
    let win2 = window.clone();
    let result = tokio::task::spawn_blocking(move || pipeline::transcribe_pipeline(&wav_path, &mut engine, &vad_config, true, &win2))
        .await.map_err(|e| format!("Join: {}", e))?.map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command] pub async fn export_result_string(format: String, result: crate::TranscriptionResult) -> Result<String, String> {
    crate::export::export_to_string(&result, &format).map_err(|e| format!("Export: {}", e))
}
#[tauri::command] pub async fn save_export_file(format: String, output_path: String, result: crate::TranscriptionResult) -> Result<String, String> {
    tokio::task::spawn_blocking(move || crate::export::export_to_file(&result, &crate::ExportArgs { task_id: String::new(), format, output_path }).map_err(|e| e.to_string()))
        .await.map_err(|e| format!("Join: {}", e))?
}
#[tauri::command] pub async fn list_history(limit: i64, offset: i64) -> Result<Vec<crate::db::TaskRecord>, String> {
    tokio::task::spawn_blocking(move || crate::db::list_tasks(limit, offset)).await.map_err(|e| format!("Join: {}", e))?.map_err(|e| e.to_string())
}
#[tauri::command] pub async fn delete_task(task_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::db::delete_task(&task_id)).await.map_err(|e| format!("Join: {}", e))?.map_err(|e| e.to_string())
}
#[tauri::command] pub async fn download_ffmpeg() -> Result<String, String> {
    tokio::task::spawn_blocking(|| ffmpeg_sidecar::download::auto_download().map(|_| "ok".to_string()).map_err(|e| e.to_string()))
        .await.map_err(|e| format!("Join: {}", e))?
}
#[tauri::command] pub async fn open_folder(path: String) -> Result<(), String> {
    std::process::Command::new("explorer").arg(&path).spawn().map_err(|e| e.to_string())?; Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus { pub name: String, pub installed: bool, pub size_bytes: Option<u64>, pub required: bool }

fn find_models_dir() -> std::path::PathBuf {
    let start = std::env::current_dir().unwrap_or_default();
    let mut current = start.clone();
    for _ in 0..6 { let c = current.join("models"); if c.exists() { return c; } if !current.pop() { break; } }
    start.join("models")
}
#[tauri::command] pub async fn check_models() -> Result<Vec<ModelStatus>, String> {
    let d = find_models_dir();
    let ck = |s:&str,f:&str|->bool { d.join(s).exists() && std::fs::read_dir(d.join(s)).ok().and_then(|mut e| e.find_map(|x| x.ok().and_then(|x| { let p=x.path(); if p.is_dir() { p.join(f).exists().then_some(true) } else { (p.file_name().map_or(false,|n|n==f)).then_some(true) } }))).unwrap_or(false) };
    Ok(vec![
        ModelStatus { name: "SenseVoice-Small (快速引擎)".into(), installed: ck("sense-voice-small","model.int8.onnx"), size_bytes: Some(233_000_000), required: true },
        ModelStatus { name: "Paraformer-Large (精准引擎)".into(), installed: ck("paraformer-large","model.int8.onnx"), size_bytes: Some(231_000_000), required: false },
        ModelStatus { name: "Silero-VAD (语音分段)".into(), installed: ck("silero-vad","silero_vad.onnx"), size_bytes: Some(1_000_000), required: true },
        ModelStatus { name: "CT-Transformer (标点)".into(), installed: ck("punct-ct-transformer","model.onnx"), size_bytes: Some(100_000_000), required: false },
    ])
}
#[tauri::command] pub async fn get_app_config() -> Result<crate::AppConfig, String> {
    Ok(crate::AppConfig { models_dir: "./models".into(), output_dir: "./output".into(), ffmpeg_path: "ffmpeg".into(), download_mirror: "https://www.modelscope.cn".into() })
}
#[tauri::command] pub async fn download_model(url: String, output_path: String, window: tauri::Window) -> Result<String, String> {
    let win = window.clone(); let pc = output_path.clone();
    tokio::task::spawn_blocking(move || crate::download::download_file(&url, &pc, &win)).await.map_err(|e| format!("Join: {}", e))?.map_err(|e| e.to_string())?;
    Ok(output_path)
}
