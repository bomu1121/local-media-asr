import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TaskProgress } from "./types";

export function onExtractProgress(
  callback: (progress: TaskProgress) => void
): Promise<UnlistenFn> {
  return listen<TaskProgress>("extract-progress", (event) => {
    callback(event.payload as TaskProgress);
  });
}

export function onTranscribeProgress(
  callback: (progress: TaskProgress) => void
): Promise<UnlistenFn> {
  return listen<TaskProgress>("transcribe-progress", (event) => {
    callback(event.payload as TaskProgress);
  });
}
