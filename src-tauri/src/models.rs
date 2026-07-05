// Model management: download, check, and list ASR models

use serde::{Deserialize, Serialize};

/// Status of a single model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelStatus {
    pub name: String,
    pub model_id: String,
    pub installed: bool,
    pub size_bytes: Option<u64>,
    pub required: bool,
    pub download_url: Option<String>,
}

/// Available models registry
pub struct ModelRegistry;

impl ModelRegistry {
    /// List all available models with their status
    pub fn list_models(models_dir: &str) -> Vec<ModelStatus> {
        let dir = std::path::Path::new(models_dir);
        vec![
            ModelStatus {
                name: "SenseVoice-Small (int8)".into(),
                model_id: "sense-voice-small".into(),
                installed: dir.join("sense-voice-small").join("model.int8.onnx").exists(),
                size_bytes: Some(230_000_000),
                required: true,
                download_url: Some(
                    "https://modelscope.cn/models/k2-fsa/sense-voice-small-onnx/raw/master/model.int8.onnx"
                        .into(),
                ),
            },
            ModelStatus {
                name: "Paraformer-Large (int8)".into(),
                model_id: "paraformer-large".into(),
                installed: dir.join("paraformer-large").join("model.int8.onnx").exists(),
                size_bytes: Some(227_000_000),
                required: false,
                download_url: Some(
                    "https://modelscope.cn/models/k2-fsa/paraformer-large-onnx/raw/master/model.int8.onnx"
                        .into(),
                ),
            },
            ModelStatus {
                name: "Silero-VAD".into(),
                model_id: "silero-vad".into(),
                installed: dir.join("silero-vad").join("silero_vad.onnx").exists(),
                size_bytes: Some(2_200_000),
                required: true,
                download_url: Some(
                    "https://modelscope.cn/models/k2-fsa/sherpa-onnx-silero-vad-onnx/raw/master/silero_vad.onnx"
                        .into(),
                ),
            },
            ModelStatus {
                name: "CT-Transformer (标点)".into(),
                model_id: "punct-ct-transformer".into(),
                installed: dir.join("punct-ct-transformer").join("model.onnx").exists(),
                size_bytes: Some(100_000_000),
                required: false,
                download_url: Some(
                    "https://modelscope.cn/models/k2-fsa/punct-ct-transformer-onnx/raw/master/model.onnx"
                        .into(),
                ),
            },
        ]
    }

    /// Get the models directory (relative to app data)
    pub fn default_models_dir() -> String {
        "./models".to_string()
    }
}