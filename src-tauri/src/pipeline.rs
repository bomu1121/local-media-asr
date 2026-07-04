// Transcription pipeline: VAD + ASR joint inference with smart chunking
// Implements the optimized pipeline from the dev document:
//   VAD detect -> smart chunk -> ASR recognize -> post-process

use crate::asr::{AsrEngine, EngineType};
use crate::vad::{SpeechSegment, VadConfig, VadDetector};
use crate::{TaskProgress, TranscriptionResult, TranscriptionSegment};
use anyhow::{Context, Result};
use std::io::BufReader;
use std::path::Path;
use tauri::Emitter;

/// Chunk of audio to be fed to ASR, with overlap to avoid boundary loss
#[derive(Debug, Clone)]
struct AudioChunk {
    samples: Vec<f32>,
    start_time: f64,
    end_time: f64,
}

/// Load 16kHz mono PCM WAV file into f32 samples
fn read_wav_samples(path: &str) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path).context("Failed to open audio file")?;
    let mut reader = BufReader::new(file);
    let (header, data) = wav::read(&mut reader).context("Failed to parse WAV file")?;

    let sample_rate = header.sampling_rate;

    // Convert to f32 samples
    let samples: Vec<f32> = match header.bits_per_sample {
        16 => data
            .try_into_sixteen()
            .map_err(|_| anyhow::anyhow!("Expected 16-bit PCM"))?
            .iter()
            .map(|&s| s as f32 / 32768.0)
            .collect(),
        32 => data
            .try_into_thirty_two_float()
            .map_err(|_| anyhow::anyhow!("Expected 32-bit float PCM"))?
            .to_vec(),
        _ => return Err(anyhow::anyhow!("Unsupported bit depth: {}", header.bits_per_sample)),
    };

    Ok((samples, sample_rate))
}

/// Run the full transcription pipeline on a WAV audio file
pub fn transcribe_pipeline(audio_path: &str, engine: &mut AsrEngine,
    vad_config: &VadConfig,
    use_vad: bool,
    window: &tauri::Window,
) -> Result<TranscriptionResult> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();

    // Emit start event
    let _ = window.emit(
        "transcribe-progress",
        TaskProgress {
            task_id: task_id.clone(),
            stage: "loading".into(),
            progress: 0.0,
            message: "Loading audio...".into(),
        },
    );

    // Step 1: Read audio
    let (samples, sample_rate) = read_wav_samples(audio_path)?;

    let _ = window.emit(
        "transcribe-progress",
        TaskProgress {
            task_id: task_id.clone(),
            stage: "vad".into(),
            progress: 0.05,
            message: "Detecting speech segments...".into(),
        },
    );

    // Step 2: VAD detection (get speech segments)
    let chunks = if use_vad {
        let vad = VadDetector::new(vad_config.clone());
        vad.load("./models/silero-vad/silero_vad.onnx")
            .unwrap_or_else(|e| {
                eprintln!("VAD model not available, falling back to full audio: {}", e);
            });

        let segments = vad.detect(&samples, sample_rate)?;

        if segments.is_empty() {
            // Fallback: treat entire audio as one segment
            vec![AudioChunk {
                samples: samples.clone(),
                start_time: 0.0,
                end_time: samples.len() as f64 / sample_rate as f64,
            }]
        } else {
            // Convert VAD segments to audio chunks with 2-second overlap
            let sr = sample_rate as f64;
            let overlap_samples = (2.0 * sr) as usize;

            segments
                .iter()
                .map(|seg| {
                    let start_idx = (seg.start * sr) as usize;
                    let end_idx = (seg.end * sr) as usize;

                    // Add overlap on both sides
                    let chunk_start = start_idx.saturating_sub(overlap_samples);
                    let chunk_end = (end_idx + overlap_samples).min(samples.len());

                    AudioChunk {
                        samples: samples[chunk_start..chunk_end].to_vec(),
                        start_time: seg.start,
                        end_time: seg.end,
                    }
                })
                .collect()
        }
    } else {
        // No VAD: use the entire audio as one chunk
        vec![AudioChunk {
            samples: samples.clone(),
            start_time: 0.0,
            end_time: samples.len() as f64 / sample_rate as f64,
        }]
    };

    let total_chunks = chunks.len();

    // Step 3: ASR inference on each chunk
    let mut all_segments: Vec<TranscriptionSegment> = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let progress = 0.1 + 0.85 * (i as f32 / total_chunks as f32);
        let _ = window.emit(
            "transcribe-progress",
            TaskProgress {
                task_id: task_id.clone(),
                stage: "transcribing".into(),
                progress,
                message: format!(
                    "Transcribing... ({}/{})",
                    i + 1,
                    total_chunks
                ),
            },
        );

        let text = engine.recognize(&chunk.samples)?;

        // Filter out empty results and noise
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            all_segments.push(TranscriptionSegment {
                start: chunk.start_time,
                end: chunk.end_time,
                text: trimmed.to_string(),
            });
        }
    }

    // Step 4: Build final result
    let full_text = all_segments
        .iter()
        .map(|s| s.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    let duration = start_time.elapsed().as_secs_f64();
    let engine_name = match engine.engine_type {
        EngineType::Fast => "fast",
        EngineType::Precise => "precise",
    };

    let result = TranscriptionResult {
        text: full_text,
        segments: all_segments,
        engine: engine_name.to_string(),
        duration,
    };

    // Emit completion
    let _ = window.emit(
        "transcribe-progress",
        TaskProgress {
            task_id,
            stage: "completed".into(),
            progress: 1.0,
            message: "Transcription complete".into(),
        },
    );

    Ok(result)
}
