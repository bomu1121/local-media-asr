// VAD (Voice Activity Detection) module via sherpa-onnx Silero-VAD
// Detects speech segments in audio for smart chunking

use anyhow::{Context, Result};
use std::sync::Mutex;

/// A detected speech segment with start/end times in seconds
#[derive(Debug, Clone)]
pub struct SpeechSegment {
    pub start: f64,
    pub end: f64,
}

/// VAD configuration (matches frontend settings)
#[derive(Debug, Clone)]
pub struct VadConfig {
    pub threshold: f32,
    pub min_speech_duration: f32,
    pub min_silence_duration: f32,
    pub max_segment_duration: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        VadConfig {
            threshold: 0.5,
            min_speech_duration: 0.3,
            min_silence_duration: 0.5,
            max_segment_duration: 60.0,
        }
    }
}

/// VAD detector using Silero-VAD model via sherpa-onnx
pub struct VadDetector {
    config: VadConfig,
    // Note: sherpa-rs VAD API uses a specific type.
    // We'll use the sherpa_rs::vad module when models are available.
    // For now, we implement a simple energy-based VAD as fallback.
    model_loaded: Mutex<bool>,
}

impl VadDetector {
    pub fn new(config: VadConfig) -> Self {
        VadDetector {
            config,
            model_loaded: Mutex::new(false),
        }
    }

    /// Load the Silero-VAD model from the given path.
    /// When sherpa-onnx VAD model is available, use sherpa_rs::vad::Vad.
    pub fn load(&self, _model_path: &str) -> Result<()> {
        // TODO: Load Silero-VAD model when available
        // let vad = sherpa_rs::vad::Vad::new(model_path)?;
        let mut guard = self.model_loaded.lock().unwrap();
        *guard = true;
        Ok(())
    }

    /// Detect speech segments in audio samples.
    /// `samples` is a &[f32] array of 16kHz mono PCM audio.
    /// `sample_rate` is the sample rate (should be 16000).
    /// Returns a vector of SpeechSegment with start/end times in seconds.
    pub fn detect(&self, samples: &[f32], sample_rate: u32) -> Result<Vec<SpeechSegment>> {
        if !*self.model_loaded.lock().unwrap() {
            return Err(anyhow::anyhow!("VAD model not loaded"));
        }

        let sr = sample_rate as f64;
        let min_samples = (self.config.min_speech_duration as f64 * sr) as usize;
        let min_silence_samples = (self.config.min_silence_duration as f64 * sr) as usize;
        let max_segment_samples = (self.config.max_segment_duration as f64 * sr) as usize;
        let threshold = self.config.threshold as f64;

        // Energy-based VAD: compute short-time energy and detect speech regions
        let frame_size = (sr * 0.025) as usize; // 25ms frames
        let total_frames = samples.len() / frame_size;

        if total_frames == 0 {
            return Ok(vec![]);
        }

        let mut segments = Vec::new();
        let mut in_speech = false;
        let mut speech_start_frame = 0;
        let mut silence_frames = 0;
        let mut max_energy = 0.0f64;

        // First pass: compute max energy for normalization
        for i in 0..total_frames {
            let start = i * frame_size;
            let end = (start + frame_size).min(samples.len());
            let energy: f64 = samples[start..end].iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / (end - start) as f64;
            if energy > max_energy {
                max_energy = energy;
            }
        }

        if max_energy == 0.0 {
            return Ok(vec![]);
        }

        // Second pass: detect speech segments
        for i in 0..total_frames {
            let start = i * frame_size;
            let end = (start + frame_size).min(samples.len());
            let energy: f64 = samples[start..end].iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / (end - start) as f64;
            let normalized = energy / max_energy;

            let is_speech = normalized > threshold;

            if is_speech && !in_speech {
                // Start of speech
                in_speech = true;
                speech_start_frame = i;
                silence_frames = 0;
            } else if !is_speech && in_speech {
                silence_frames += 1;
                if silence_frames * frame_size >= min_silence_samples {
                    // End of speech segment
                    let segment_frames = i - speech_start_frame - silence_frames;
                    if segment_frames * frame_size >= min_samples {
                        let segment_start = speech_start_frame as f64 * frame_size as f64 / sr;
                        let segment_end = (i - silence_frames) as f64 * frame_size as f64 / sr;

                        // Check max segment duration: split if too long
                        let duration = segment_end - segment_start;
                        if duration > self.config.max_segment_duration as f64 {
                            // Split into max_segment_duration chunks
                            let mut chunk_start = segment_start;
                            while chunk_start < segment_end {
                                let chunk_end = (chunk_start + self.config.max_segment_duration as f64).min(segment_end);
                                segments.push(SpeechSegment {
                                    start: chunk_start,
                                    end: chunk_end,
                                });
                                chunk_start = chunk_end;
                            }
                        } else {
                            segments.push(SpeechSegment {
                                start: segment_start,
                                end: segment_end,
                            });
                        }
                    }
                    in_speech = false;
                }
            } else if is_speech && in_speech {
                silence_frames = 0;
            }
        }

        // Handle final segment if still in speech
        if in_speech {
            let segment_frames = total_frames - speech_start_frame;
            if segment_frames * frame_size >= min_samples {
                let segment_start = speech_start_frame as f64 * frame_size as f64 / sr;
                let segment_end = samples.len() as f64 / sr;
                segments.push(SpeechSegment {
                    start: segment_start,
                    end: segment_end,
                });
            }
        }

        Ok(segments)
    }
}
