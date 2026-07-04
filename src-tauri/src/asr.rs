#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineType { Fast, Precise }

impl EngineType {
    pub fn from_str(s: &str) -> Self { match s { "precise" => EngineType::Precise, _ => EngineType::Fast } }
    pub fn model_name(&self) -> &str { match self { EngineType::Fast => "sense-voice-small", EngineType::Precise => "paraformer-large" } }
}

/// Walk up from `start` looking for a directory containing `models/`
fn find_models_dir(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();
    for _ in 0..6 {
        let candidate = current.join("models");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() { break; }
    }
    None
}

#[cfg(not(feature = "asr"))]
mod engine {
    use super::EngineType;
    pub struct AsrEngine { pub engine_type: EngineType, loaded: bool }
    impl AsrEngine {
        pub fn new(_dir: &str, engine_type: EngineType) -> Self { AsrEngine { engine_type, loaded: false } }
        pub fn load(&mut self) -> anyhow::Result<()> { self.loaded = true; Ok(()) }
        pub fn is_loaded(&self) -> bool { self.loaded }
        pub fn recognize(&mut self, _samples: &[f32]) -> anyhow::Result<String> { Ok("[ASR not available]".into()) }
    }
}

#[cfg(feature = "asr")]
mod engine {
    use super::EngineType;
    use super::find_models_dir;
    use anyhow::{Context, Result};
    use sherpa_rs::sense_voice::{SenseVoiceRecognizer, SenseVoiceConfig};
    use sherpa_rs::paraformer::{ParaformerRecognizer, ParaformerConfig};
    use std::path::Path;

    enum Inner { Sense(SenseVoiceRecognizer), Para(ParaformerRecognizer) }

    pub struct AsrEngine { recognizer: Option<Inner>, pub engine_type: EngineType }

    impl AsrEngine {
        pub fn new(_dir: &str, engine_type: EngineType) -> Self { AsrEngine { recognizer: None, engine_type } }

        pub fn load(&mut self) -> Result<()> {
            // Resolve models dir: env var > CWD walk-up > exe walk-up
            let models_root = if let Ok(dir) = std::env::var("LOCAL_ASR_MODELS_DIR") {
                Path::new(&dir).to_path_buf()
            } else {
                let cwd = std::env::current_dir().unwrap_or_default();
                find_models_dir(&cwd)
                    .or_else(|| {
                        std::env::current_exe().ok()
                            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                            .and_then(|exe_dir| find_models_dir(&exe_dir))
                    })
                    .unwrap_or_else(|| cwd.join("models"))
            };

            let model_dir = models_root.join(self.engine_type.model_name());
            if !model_dir.exists() {
                return Err(anyhow::anyhow!("Model dir not found at {}", model_dir.display()));
            }

            // Auto-discover nested model subdirectory (e.g. sense-voice-small/sherpa-onnx-xxx-2025-09-09/)
            let inner = std::fs::read_dir(&model_dir)
                .context("Failed to read model dir")?
                .filter_map(|e| e.ok())
                .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .unwrap_or(model_dir.clone());

            let model = inner.join("model.int8.onnx").to_string_lossy().to_string();
            let tokens = inner.join("tokens.txt").to_string_lossy().to_string();

            if !Path::new(&model).exists() { return Err(anyhow::anyhow!("Model file not found: {}", model)); }
            if !Path::new(&tokens).exists() { return Err(anyhow::anyhow!("Tokens file not found: {}", tokens)); }

            let n = std::thread::available_parallelism().map(|n| n.get().saturating_sub(1).max(1) as i32).unwrap_or(4);

            match self.engine_type {
                EngineType::Fast => {
                    let cfg = SenseVoiceConfig { model, tokens, num_threads: Some(n), language: "zh".into(), ..Default::default() };
                    self.recognizer = Some(Inner::Sense(SenseVoiceRecognizer::new(cfg).map_err(|e| anyhow::anyhow!("{}", e))?));
                }
                EngineType::Precise => {
                    let cfg = ParaformerConfig { model, tokens, num_threads: Some(n), ..Default::default() };
                    self.recognizer = Some(Inner::Para(ParaformerRecognizer::new(cfg).map_err(|e| anyhow::anyhow!("{}", e))?));
                }
            }
            Ok(())
        }

        pub fn is_loaded(&self) -> bool { self.recognizer.is_some() }

        pub fn recognize(&mut self, samples: &[f32]) -> Result<String> {
            let rec = self.recognizer.as_mut().ok_or_else(|| anyhow::anyhow!("ASR not loaded"))?;
            let result = match rec { Inner::Sense(r) => r.transcribe(16000, samples), Inner::Para(r) => r.transcribe(16000, samples) };
            Ok(result.text)
        }
    }
}

pub use engine::AsrEngine;
