// Tauri event listeners for real-time progress from Rust backend
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TaskProgress } from "./types";

/** Listen to audio extraction progress events from FFmpeg */
export function onExtractProgress(
  callback: (progress: TaskProgress) => void
): Promise<UnlistenFn> {
  return listen<TaskProgress>("extract-progress", (event) => {
    callback(event.payload);
  });
}

/** Listen to transcription progress events */
export function onTranscribeProgress(
  callback: (progress: TaskProgress) => void
): Promise<UnlistenFn> {
  return listen<TaskProgress>("transcribe-progress", (event) => {
    callback(event.payload);
  });
}
