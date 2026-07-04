// ASR (Automatic Speech Recognition) module via sherpa-onnx
// When the "asr" feature is enabled, this module uses sherpa-rs for real inference.
// Without it, it provides a no-op stub so the project compiles cleanly.

use anyhow::Result;
use std::sync::Mutex;

/// Engine type matching the frontend settings
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineType {
    Fast,    // SenseVoice-Small
    Precise, // Paraformer-Large
}

impl EngineType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "precise" => EngineType::Precise,
            _ => EngineType::Fast,
        }
    }

    pub fn model_name(&self) -> &str {
        match self {
            EngineType::Fast => "sense-voice-small",
            EngineType::Precise => "paraformer-large",
        }
    }
}

#[cfg(not(feature = "asr"))]
mod engine {
    use super::{EngineType, Result};
    use std::sync::Mutex;

    pub struct AsrEngine {
        engine_type: EngineType,
        model_dir: String,
        loaded: Mutex<bool>,
    }

    impl AsrEngine {
        pub fn new(model_dir: &str, engine_type: EngineType) -> Self {
            AsrEngine {
                engine_type,
                model_dir: model_dir.to_string(),
                loaded: Mutex::new(false),
            }
        }

        pub fn load(&self) -> Result<()> {
            *self.loaded.lock().unwrap() = true;
            Ok(())
        }

        pub fn is_loaded(&self) -> bool {
            *self.loaded.lock().unwrap()
        }

        pub fn recognize(&self, _samples: &[f32]) -> Result<String> {
            Ok("[ASR engine not available - enable 'asr' feature and download models]".to_string())
        }

        pub fn engine_type(&self) -> EngineType {
            self.engine_type
        }
    }
}

#[cfg(feature = "asr")]
mod engine {
    use super::{EngineType, Result};
    use anyhow::Context;
    use sherpa_rs::offline::*;
    use std::path::Path;
    use std::sync::Mutex;

    pub struct AsrEngine {
        recognizer: Mutex<Option<OfflineRecognizer>>,
        engine_type: EngineType,
        model_dir: String,
    }

    impl AsrEngine {
        pub fn new(model_dir: &str, engine_type: EngineType) -> Self {
            AsrEngine {
                recognizer: Mutex::new(None),
                engine_type,
                model_dir: model_dir.to_string(),
            }
        }

        pub fn load(&self) -> Result<()> {
            let model_path = Path::new(&self.model_dir).join(self.engine_type.model_name());

            if !model_path.exists() {
                return Err(anyhow::anyhow!(
                    "Model not found at {}. Download the model first.",
                    model_path.display()
                ));
            }

            let config = OfflineRecognizerConfig {
                model: OfflineModelConfig {
                    sense_voice: OfflineSenseVoiceModelConfig {
                        model: model_path.join("model.int8.onnx").to_string_lossy().to_string(),
                        ..Default::default()
                    },
                    debug: false,
                    num_threads: num_cpu_threads(),
                    provider: "cpu".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            };

            let recognizer = OfflineRecognizer::new(config)
                .context("Failed to create OfflineRecognizer")?;

            *self.recognizer.lock().unwrap() = Some(recognizer);
            Ok(())
        }

        pub fn is_loaded(&self) -> bool {
            self.recognizer.lock().unwrap().is_some()
        }

        pub fn recognize(&self, samples: &[f32]) -> Result<String> {
            let mut guard = self.recognizer.lock().unwrap();
            let recognizer = guard
                .as_mut()
                .context("ASR model not loaded")?;

            let stream = recognizer.create_stream();
            stream.accept_waveform(samples);
            recognizer.decode_stream(&stream);

            let text = recognizer.get_result(&stream)
                .map(|r| r.text)
                .unwrap_or_default();

            Ok(text)
        }

        pub fn engine_type(&self) -> EngineType {
            self.engine_type
        }
    }

    fn num_cpu_threads() -> i32 {
        std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1) as i32)
            .unwrap_or(4)
    }
}

pub use engine::AsrEngine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_type_from_str() {
        assert_eq!(EngineType::from_str("fast"), EngineType::Fast);
        assert_eq!(EngineType::from_str("precise"), EngineType::Precise);
        assert_eq!(EngineType::from_str("unknown"), EngineType::Fast);
    }
}
