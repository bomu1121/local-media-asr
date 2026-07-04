// Export format generators: TXT, SRT, VTT, LRC, JSON
// Converts TranscriptionResult into formatted file content

use crate::{ExportArgs, TranscriptionResult, TranscriptionSegment};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Format a single timestamp in SRT/VTT style: HH:MM:SS,mmm
fn format_timestamp(seconds: f64) -> String {
    let h = (seconds / 3600.0) as u32;
    let m = ((seconds % 3600.0) / 60.0) as u32;
    let s = seconds % 60.0;
    format!("{:02}:{:02}:{:06.3}", h, m, s)
}

/// Format timestamp for LRC: [MM:SS.xx]
fn format_lrc_timestamp(seconds: f64) -> String {
    let m = (seconds / 60.0) as u32;
    let s = seconds % 60.0;
    format!("[{:02}:{:05.2}]", m, s)
}

/// Export as plain text (TXT)
fn export_txt(result: &TranscriptionResult) -> String {
    if result.segments.is_empty() {
        return result.text.clone();
    }
    result
        .segments
        .iter()
        .map(|s| s.text.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Export as SRT subtitle format
fn export_srt(result: &TranscriptionResult) -> String {
    let segments = if result.segments.is_empty() {
        // Create a single segment from the full text if no timestamps
        vec![TranscriptionSegment {
            start: 0.0,
            end: result.duration,
            text: result.text.clone(),
        }]
    } else {
        result.segments.clone()
    };

    segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                i + 1,
                format_timestamp(seg.start).replace('.', ","),
                format_timestamp(seg.end).replace('.', ","),
                seg.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Export as WebVTT subtitle format
fn export_vtt(result: &TranscriptionResult) -> String {
    let mut output = String::from("WEBVTT\n\n");

    let segments = if result.segments.is_empty() {
        vec![TranscriptionSegment {
            start: 0.0,
            end: result.duration,
            text: result.text.clone(),
        }]
    } else {
        result.segments.clone()
    };

    for (i, seg) in segments.iter().enumerate() {
        output.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            format_timestamp(seg.start),
            format_timestamp(seg.end),
            seg.text
        ));
    }

    output
}

/// Export as LRC lyrics format
fn export_lrc(result: &TranscriptionResult) -> String {
    let segments = if result.segments.is_empty() {
        // Split text into lines with estimated timing
        let lines: Vec<&str> = result.text.lines().collect();
        let line_count = lines.len().max(1) as f64;
        lines
            .iter()
            .enumerate()
            .map(|(i, &line)| TranscriptionSegment {
                start: (i as f64 / line_count) * result.duration,
                end: ((i + 1) as f64 / line_count) * result.duration,
                text: line.to_string(),
            })
            .collect()
    } else {
        result.segments.clone()
    };

    let mut output = String::from("[ti:Transcription]\n[ar:local-media-asr]\n\n");
    for seg in &segments {
        for line in seg.text.lines() {
            if !line.trim().is_empty() {
                output.push_str(&format!("{}{}\n", format_lrc_timestamp(seg.start), line));
            }
        }
    }

    output
}

/// Export as JSON (structured with metadata)
fn export_json(result: &TranscriptionResult) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "engine": result.engine,
        "duration_seconds": result.duration,
        "full_text": result.text,
        "segments": result.segments.iter().map(|s| {
            serde_json::json!({
                "start": s.start,
                "end": s.end,
                "text": s.text,
            })
        }).collect::<Vec<_>>(),
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

/// Export transcription result to the specified format and write to file
pub fn export_to_file(result: &TranscriptionResult, args: &ExportArgs) -> Result<String> {
    let content = match args.format.as_str() {
        "txt" => export_txt(result),
        "srt" => export_srt(result),
        "vtt" => export_vtt(result),
        "lrc" => export_lrc(result),
        "json" => export_json(result),
        _ => return Err(anyhow::anyhow!("Unsupported format: {}", args.format)),
    };

    let output_path = &args.output_path;

    // Ensure directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).context("Failed to create export directory")?;
    }

    fs::write(output_path, &content).context("Failed to write export file")?;

    Ok(output_path.clone())
}

/// Export to string (for preview / clipboard)
pub fn export_to_string(result: &TranscriptionResult, format: &str) -> Result<String> {
    match format {
        "txt" => Ok(export_txt(result)),
        "srt" => Ok(export_srt(result)),
        "vtt" => Ok(export_vtt(result)),
        "lrc" => Ok(export_lrc(result)),
        "json" => Ok(export_json(result)),
        _ => Err(anyhow::anyhow!("Unsupported format: {}", format)),
    }
}
