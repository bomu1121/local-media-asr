// Tauri command invoke wrappers - typed frontend API to Rust backend
import { invoke } from "@tauri-apps/api/core";
import type {
  AudioExtractArgs,
  FileInfo,
  AppConfig,
  ModelStatus,
  TaskRecord,
} from "./types";

/** Extract audio from media file, converting to 16kHz mono WAV */
export async function extractAudio(args: AudioExtractArgs): Promise<string> {
  return invoke("extract_audio", { args });
}

/** Probe media file metadata (duration, streams, codec info) */
export async function getMediaInfo(filePath: string): Promise<FileInfo> {
  return invoke("get_media_info", { filePath });
}

/** Check FFmpeg availability and return version */
export async function checkFfmpeg(): Promise<string> {
  return invoke("check_ffmpeg");
}

/** Auto-download FFmpeg binary via ffmpeg-sidecar */
export async function downloadFfmpeg(): Promise<string> {
  return invoke("download_ffmpeg");
}

/** Start ASR transcription on extracted audio */
export async function startTranscription(
  audioPath: string,
  engineType: string,
): Promise<TranscriptionResult> {
  return invoke("start_transcription", { audioPath, engineType });
}

/** List available ASR models and their installation status */
export async function checkModels(): Promise<ModelStatus[]> {
  return invoke("check_models");
}

/** Get application configuration */

/** Start ASR transcription on extracted audio */
export async function getAppConfig(): Promise<AppConfig> {
  return invoke("get_app_config");
}

/** List task history from SQLite database */
export async function listHistory(limit: number, offset: number): Promise<TaskRecord[]> {
  return invoke("list_history", { limit, offset });
}

/** Delete a task and its transcription from the database */
export async function deleteTask(taskId: string): Promise<void> {
  return invoke("delete_task", { taskId });
}
