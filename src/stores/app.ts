import { defineStore } from "pinia";
import { ref, computed } from "vue";

export type EngineType = "fast" | "precise";
export type ExportFormat = "txt" | "srt" | "lrc" | "json" | "vtt";

export interface TaskFile {
  id: string;
  name: string;
  path: string;
  size: number;
  format: string;
  status: "pending" | "extracting" | "transcribing" | "completed" | "failed";
  progress: number;
  error?: string;
  result?: TranscriptionResult;
}

export interface TranscriptionResult {
  text: string;
  segments: Array<{
    start: number;
    end: number;
    text: string;
  }>;
  engine: EngineType;
  duration: number;
}

export interface VadSettings {
  enabled: boolean;
  threshold: number;
  minSpeechDuration: number;
  minSilenceDuration: number;
  maxSegmentDuration: number;
}

export interface AppSettings {
  engine: EngineType;
  denoise: boolean;
  punctuation: boolean;
  exportFormat: ExportFormat;
  vad: VadSettings;
  outputDir: string;
}

export const useAppStore = defineStore("app", () => {
  const isDark = ref(false);
  const tasks = ref<TaskFile[]>([]);
  const activeTaskId = ref<string | null>(null);
  const activeTab = ref<"transcribe" | "history" | "settings">("transcribe");
  const activeTask = computed(() =>
    tasks.value.find((t) => t.id === activeTaskId.value) ?? null
  );

  const settings = ref<AppSettings>({
    engine: "fast",
    denoise: false,
    punctuation: true,
    exportFormat: "txt",
    vad: {
      enabled: true,
      threshold: 0.5,
      minSpeechDuration: 0.3,
      minSilenceDuration: 0.5,
      maxSegmentDuration: 60,
    },
    outputDir: "",
  });

  function addTask(file: Omit<TaskFile, "id" | "status" | "progress">) {
    const task: TaskFile = {
      ...file,
      id: crypto.randomUUID(),
      status: "pending",
      progress: 0,
    };
    tasks.value.unshift(task);
    activeTaskId.value = task.id;
    return task;
  }

  function removeTask(id: string) {
    tasks.value = tasks.value.filter((t) => t.id !== id);
    if (activeTaskId.value === id) {
      activeTaskId.value = tasks.value[0]?.id ?? null;
    }
  }

  function selectTask(id: string) {
    activeTaskId.value = id;
  }

  function toggleDark() {
    isDark.value = !isDark.value;
  }

  return {
    isDark,
    tasks,
    activeTaskId,
    activeTab,
    settings,
    activeTask,
    addTask,
    removeTask,
    selectTask,
    toggleDark,
  };
});
