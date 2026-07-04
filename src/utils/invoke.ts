// Tauri command invoke wrappers - typed frontend API to Rust backend
import { invoke } from "@tauri-apps/api/core";
import type {
  AudioExtractArgs,
  FileInfo,
  AppConfig,
  ModelStatus,
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

/** List available ASR models and their installation status */
export async function checkModels(): Promise<ModelStatus[]> {
  return invoke("check_models");
}

/** Get application configuration */
export async function getAppConfig(): Promise<AppConfig> {
  return invoke("get_app_config");
}
