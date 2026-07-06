import { invoke } from "@tauri-apps/api/core";
import type { AudioExtractArgs, FileInfo, AppConfig, ModelStatus, TaskRecord, TranscriptionResult } from "./types";

export async function extractAudio(args: AudioExtractArgs): Promise<string> {
  return invoke("extract_audio", { args });
}
export async function getMediaInfo(filePath: string): Promise<FileInfo> {
  return invoke("get_media_info", { filePath });
}
export async function checkFfmpeg(): Promise<string> {
  return invoke("check_ffmpeg");
}
export async function downloadFfmpeg(): Promise<string> {
  return invoke("download_ffmpeg");
}
/** Unified pipeline: extract + transcribe in one go */
export async function processMedia(filePath: string, engineType: string): Promise<TranscriptionResult> {
  return invoke("process_media", { filePath, engineType });
}

/** Get formatted export content for preview (TXT/SRT/VTT/LRC/JSON) */
export async function exportResultString(format: string, result: TranscriptionResult): Promise<string> {
  return invoke("export_result_string", { format, result });
}
/** Save export content to a file on disk */
export async function saveExportFile(format: string, outputPath: string, result: TranscriptionResult): Promise<string> {
  return invoke("save_export_file", { format, outputPath, result });
}
export async function checkModels(): Promise<ModelStatus[]> {
  return invoke("check_models");
}
export async function getAppConfig(): Promise<AppConfig> {
  return invoke("get_app_config");
}
export async function listHistory(limit: number, offset: number): Promise<TaskRecord[]> {
  return invoke("list_history", { limit, offset });
}
export async function deleteTask(taskId: string): Promise<void> {
  return invoke("delete_task", { taskId });
}

/** Download a model by id and URL, saving to the models directory */
export async function downloadModel(url: string, outputPath: string): Promise<string> {
  return invoke("download_model", { url, outputPath });
}

/** Download a model by model_id with the given URL */
export async function downloadModelById(modelId: string, downloadUrl: string): Promise<string> {
  return invoke("download_model_by_id", { modelId, downloadUrl });
}

/** AI text refinement via DeepSeek API */
export async function refineAsrText(rawText: string, apiKey: string): Promise<string> {
  return invoke("refine_asr_text", { rawText, apiKey });
}
/** Check environment: ASR worker, models, FFmpeg, Python */
export async function checkEnvironment(): Promise<import("./types").EnvCheck> {
  return invoke("check_environment");
}