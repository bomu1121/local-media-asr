use crate::ffmpeg;
use tauri::Manager;
use crate::{AudioExtractArgs, FileInfo};
use serde::{Deserialize, Serialize};

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

#[tauri::command]
pub fn get_resource_path(app: tauri::AppHandle) -> Result<String, String> {
    let resource_dir = app.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    Ok(resource_dir.to_string_lossy().to_string())
}


#[tauri::command]
pub fn run_asr(wav_path: String, resource_dir: String) -> Result<String, String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;

    let exe_dir = std::env::current_exe()
        .ok().and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let asr_exe = exe_dir.join("asr_worker.exe");

    if !asr_exe.exists() {
        return Err(format!("ASR worker not found at: {}", asr_exe.display()));
    }
    if !std::path::Path::new(&wav_path).exists() {
        return Err(format!("WAV file not found: {}", wav_path));
    }
    let models_dir = std::path::Path::new(&resource_dir).join("models");
    let models_dir_str = models_dir.to_string_lossy().to_string();
    if !models_dir.exists() {
        return Err(format!("Models dir not found: {}", models_dir_str));
    }

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new(&asr_exe)
        .args(["--wav", &wav_path, "--model", "paraformer", "--models-dir", &models_dir_str])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to spawn ASR worker at {}: {}", asr_exe.display(), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("ASR worker exited with code: {:?}\nstdout: {}\nstderr: {}", output.status.code(), stdout, stderr));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in ASR output: {}", e))?;
    if stdout.trim().is_empty() {
        return Err("ASR worker completed but produced no output. The model may have failed silently.".to_string());
    }
    Ok(stdout)
}


#[derive(Debug, serde::Serialize)]
pub struct EnvCheck {
    pub ok: bool,
    pub items: Vec<EnvCheckItem>,
}

#[derive(Debug, serde::Serialize)]
pub struct EnvCheckItem {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[tauri::command]
pub fn check_environment(app: tauri::AppHandle) -> EnvCheck {
    let mut items = Vec::new();
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // 1. ASR worker exe
    let exe_dir = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let asr_exe = exe_dir.join("asr_worker.exe");
    items.push(EnvCheckItem {
        name: "asr_worker.exe".into(),
        passed: asr_exe.exists(),
        detail: asr_exe.display().to_string(),
    });

    // 2. Resource dir
    let resource_dir = app.path().resource_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|e| format!("error: {}", e));
    let resource_exists = std::path::Path::new(&resource_dir).exists();
    items.push(EnvCheckItem {
        name: "Resource directory".into(),
        passed: resource_exists,
        detail: resource_dir.clone(),
    });

    // 3. Models
    let models_dir = std::path::Path::new(&resource_dir).join("models");
    let models_exist = models_dir.exists();
    let model_detail = if models_exist {
        let para = models_dir.join("paraformer-large").join("model.int8.onnx");
        let sense = models_dir.join("sense-voice-small").join("model.int8.onnx");
        format!("paraformer: {}, sensevoice: {}", para.exists(), sense.exists())
    } else {
        models_dir.display().to_string()
    };
    items.push(EnvCheckItem {
        name: "ASR models".into(),
        passed: models_exist,
        detail: model_detail,
    });

    // 4. FFmpeg
    let ffmpeg_check = std::process::Command::new("ffmpeg")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-version")
        .output();
    items.push(EnvCheckItem {
        name: "FFmpeg".into(),
        passed: ffmpeg_check.as_ref().map(|o| o.status.success()).unwrap_or(false),
        detail: ffmpeg_check.as_ref().ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().next().map(|s| s.to_string()))
            .unwrap_or_else(|| "not found".to_string()),
    });

    // 5. Python (dev only)
    let python_check = std::process::Command::new("python")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("--version")
        .output();
    items.push(EnvCheckItem {
        name: "Python (dev)".into(),
        passed: python_check.as_ref().map(|o| o.status.success()).unwrap_or(false),
        detail: python_check.as_ref().ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().to_string().into())
            .unwrap_or_else(|| "not found".to_string()),
    });

    // 6. E2E transcription test (same flow as run_asr)
    let e2e_result = if asr_exe.exists() && models_exist {
        let test_wav = std::env::temp_dir().join("asr_env_check.wav");
        let sample_rate: u32 = 16000;
        let duration_secs = 2.0;
        let num_samples = (sample_rate as f64 * duration_secs) as usize;
        let mut wav_data = Vec::with_capacity(44 + num_samples * 2);
        wav_data.extend_from_slice(b"RIFF");
        wav_data.extend_from_slice(&((36 + num_samples * 2) as u32).to_le_bytes());
        wav_data.extend_from_slice(b"WAVE");
        wav_data.extend_from_slice(b"fmt ");
        wav_data.extend_from_slice(&16u32.to_le_bytes());
        wav_data.extend_from_slice(&1u16.to_le_bytes());
        wav_data.extend_from_slice(&1u16.to_le_bytes());
        wav_data.extend_from_slice(&sample_rate.to_le_bytes());
        wav_data.extend_from_slice(&(sample_rate * 2u32).to_le_bytes());
        wav_data.extend_from_slice(&2u16.to_le_bytes());
        wav_data.extend_from_slice(&16u16.to_le_bytes());
        wav_data.extend_from_slice(b"data");
        wav_data.extend_from_slice(&((num_samples * 2) as u32).to_le_bytes());
        for i in 0..num_samples {
            let t = i as f64 / sample_rate as f64;
            let val = (16000.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin()) as i16;
            wav_data.extend_from_slice(&val.to_le_bytes());
        }
        let _ = std::fs::write(&test_wav, &wav_data);

        let output = std::process::Command::new(&asr_exe)
            .creation_flags(CREATE_NO_WINDOW)
            .args(["--wav", &test_wav.to_string_lossy(), "--model", "paraformer", "--models-dir", &models_dir.to_string_lossy()])
            .output();

        let _ = std::fs::remove_file(&test_wav);

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if stdout.contains("\"type\"") && stdout.contains("\"result\"") {
                    "E2E transcription: PASS".to_string()
                } else {
                    let preview: String = stdout.chars().take(200).collect();
                    format!("E2E: no result JSON. stdout: {}", preview)
                }
            }
            Ok(o) => {
                let err: String = String::from_utf8_lossy(&o.stderr).chars().take(200).collect();
                format!("E2E: exit code {:?}, stderr: {}", o.status.code(), err)
            }
            Err(e) => format!("E2E: spawn failed: {}", e),
        }
    } else {
        "SKIP (missing dependencies)".to_string()
    };

    items.push(EnvCheckItem {
        name: "E2E transcription test".into(),
        passed: e2e_result == "E2E transcription: PASS",
        detail: e2e_result,
    });

    let ok = items.iter().all(|i| i.passed);
    EnvCheck { ok, items }
}
