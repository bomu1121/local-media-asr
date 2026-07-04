#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineType { Fast, Precise }

impl EngineType {
    pub fn from_str(s: &str) -> Self { match s { "precise" => EngineType::Precise, _ => EngineType::Fast } }
    pub fn model_name(&self) -> &str { match self { EngineType::Fast => "sense-voice-small", EngineType::Precise => "paraformer-large" } }
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
    use anyhow::{Context, Result};
    use sherpa_rs::sense_voice::{SenseVoiceRecognizer, SenseVoiceConfig};
    use sherpa_rs::paraformer::{ParaformerRecognizer, ParaformerConfig};
    use std::path::Path;

    enum Inner { Sense(SenseVoiceRecognizer), Para(ParaformerRecognizer) }

    pub struct AsrEngine { recognizer: Option<Inner>, pub engine_type: EngineType }

    impl AsrEngine {
        pub fn new(_dir: &str, engine_type: EngineType) -> Self { AsrEngine { recognizer: None, engine_type } }

        pub fn load(&mut self) -> Result<()> {
            let base = Path::new("./models").join(self.engine_type.model_name());
            if !base.exists() { return Err(anyhow::anyhow!("Model not found at {}", base.display())); }
            let model = base.join("model.int8.onnx").to_string_lossy().to_string();
            let tokens = base.join("tokens.txt").to_string_lossy().to_string();
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
