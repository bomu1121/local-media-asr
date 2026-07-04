<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import {
  NUploadDragger,
  NButton,
  NSpace,
  NText,
  NIcon,
  NTag,
  NProgress,
  NList,
  NListItem,
  NScrollbar,
  NPopconfirm,
  useMessage,
} from "naive-ui";
import {
  CloudUploadOutline,
  TrashOutline,
  PlayOutline,
  MusicalNotesOutline,
  FilmOutline,
  CheckmarkCircleOutline,
  CloseCircleOutline,
  TimeOutline,
  WarningOutline,
} from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import { extractAudio, getMediaInfo, checkFfmpeg } from "../utils/invoke";
import { onExtractProgress } from "../utils/events";
import type { TaskFile } from "../stores/app";
import type { UnlistenFn } from "@tauri-apps/api/event";

const store = useAppStore();
const message = useMessage();

const acceptFormats =
  ".mp3,.wav,.flac,.aac,.ogg,.wma,.m4a,.opus,.mp4,.mkv,.avi,.mov,.wmv,.flv,.webm,.m4v";

const tasks = computed(() => store.tasks);
const ffmpegReady = ref<boolean | null>(null);
const ffmpegVersion = ref("");
let unlistenProgress: UnlistenFn | null = null;

onMounted(async () => {
  // Check FFmpeg on mount
  try {
    const ver = await checkFfmpeg();
    ffmpegVersion.value = ver;
    ffmpegReady.value = true;
  } catch {
    ffmpegReady.value = false;
  }

  // Listen to extraction progress events
  unlistenProgress = await onExtractProgress((progress) => {
    const task = tasks.value.find((t) => t.id === store.activeTaskId);
    if (task && task.status === "extracting") {
      task.progress = Math.round(progress.progress * 100);
    }
  });
});

onUnmounted(() => {
  unlistenProgress?.();
});

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function isAudio(format: string): boolean {
  return ["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus"].includes(
    format.toLowerCase()
  );
}

function statusColor(status: TaskFile["status"]): string {
  const colors: Record<string, string> = {
    pending: "default",
    extracting: "info",
    transcribing: "info",
    completed: "success",
    failed: "error",
  };
  return colors[status] || "default";
}

function statusLabel(status: TaskFile["status"]): string {
  const labels: Record<string, string> = {
    pending: "等待中",
    extracting: "提取音频",
    transcribing: "转写中",
    completed: "已完成",
    failed: "失败",
  };
  return labels[status] || status;
}

function handleFileSelect(option: { file: any; fileList: any[] }) {
  const file = option.file;
  const ext = file.name.split(".").pop() || "";
  store.addTask({
    name: file.name,
    path: (file as any).path || file.name,
    size: file.size || 0,
    format: ext,
  });
  message.success(`已添加: ${file.name}`);
}

async function startExtraction(task: TaskFile) {
  if (!ffmpegReady.value) {
    message.error("FFmpeg 未就绪，请先在设置中检查");
    return;
  }

  store.selectTask(task.id);
  
  // Determine output path in app data directory
  const baseName = task.name.replace(/\.[^.]+$/, "");
  const outputPath = `${baseName}_extracted.wav`;

  task.status = "extracting";
  task.progress = 0;

  try {
    const result = await extractAudio({
      input_path: task.path,
      output_path: outputPath,
      denoise: store.settings.denoise,
    });

    // After extraction, probe media info
    const info = await getMediaInfo(task.path);
    
    task.status = "pending"; // ready for transcription
    task.progress = 100;
    message.success(`音频提取完成: ${task.name}`);
  } catch (err: any) {
    task.status = "failed";
    task.error = String(err);
    message.error(`提取失败: ${err}`);
  }
}
</script>

<template>
  <div class="file-manager">
    <!-- FFmpeg Status Banner -->
    <div v-if="ffmpegReady === false" class="ffmpeg-banner error">
      <NIcon size="18"><WarningOutline /></NIcon>
      <NText>FFmpeg 未就绪，请确保 FFmpeg 已安装或通过 Sidecar 提供</NText>
    </div>

    <!-- Drop Zone -->
    <NUploadDragger
      :accept="acceptFormats"
      :multiple="true"
      :max="50"
      :show-file-list="false"
      @change="handleFileSelect"
    >
      <div class="drop-zone-content">
        <NIcon size="48" color="#999">
          <CloudUploadOutline />
        </NIcon>
        <NText depth="3" style="font-size: 16px; margin-top: 12px;">
          拖拽音视频文件到此处
        </NText>
        <NText depth="3" style="font-size: 13px; margin-top: 4px;">
          或点击选择文件 (MP3, WAV, MP4, MKV, FLAC, AAC...)
        </NText>
      </div>
    </NUploadDragger>

    <!-- Task List -->
    <div v-if="tasks.length > 0" class="task-section">
      <NText strong style="font-size: 14px; margin-bottom: 12px; display: block;">
        任务列表 ({{ tasks.length }})
      </NText>
      <NScrollbar style="max-height: 460px;">
        <NList hoverable clickable>
          <NListItem
            v-for="task in tasks"
            :key="task.id"
            @click="store.selectTask(task.id)"
          >
            <template #prefix>
              <NIcon size="20" :color="task.status === 'failed' ? '#d03050' : undefined">
                <MusicalNotesOutline v-if="isAudio(task.format)" />
                <FilmOutline v-else />
              </NIcon>
            </template>
            <div class="task-item">
              <div class="task-info">
                <NText class="task-name">{{ task.name }}</NText>
                <NSpace :size="8" style="margin-top: 2px;">
                  <NText depth="3" style="font-size: 12px;">{{ task.format.toUpperCase() }}</NText>
                  <NText depth="3" style="font-size: 12px;">{{ formatSize(task.size) }}</NText>
                </NSpace>
                <NProgress
                  v-if="task.status === 'extracting' || task.status === 'transcribing'"
                  :percentage="task.progress"
                  :height="4"
                  :border-radius="2"
                  style="margin-top: 6px; max-width: 200px;"
                />
                <NText
                  v-if="task.error"
                  type="error"
                  depth="3"
                  style="font-size: 12px;"
                >
                  {{ task.error }}
                </NText>
              </div>
              <NSpace :size="8" align="center">
                <NTag
                  :type="statusColor(task.status)"
                  size="small"
                  :bordered="false"
                >
                  {{ statusLabel(task.status) }}
                </NTag>
                <NButton
                  v-if="task.status === 'pending'"
                  size="tiny"
                  type="primary"
                  @click.stop="startExtraction(task)"
                >
                  <template #icon>
                    <NIcon><PlayOutline /></NIcon>
                  </template>
                  提取
                </NButton>
                <NPopconfirm @positive-click="store.removeTask(task.id)">
                  <template #trigger>
                    <NButton size="tiny" quaternary type="error">
                      <template #icon>
                        <NIcon><TrashOutline /></NIcon>
                      </template>
                    </NButton>
                  </template>
                  确定删除此任务？
                </NPopconfirm>
              </NSpace>
            </div>
          </NListItem>
        </NList>
      </NScrollbar>
    </div>
  </div>
</template>

<style scoped>
.file-manager {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 800px;
}

.ffmpeg-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: 6px;
  font-size: 13px;
}

.ffmpeg-banner.error {
  background: rgba(208, 48, 80, 0.08);
  color: #d03050;
}

.drop-zone-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
}

.task-section {
  margin-top: 8px;
}

.task-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.task-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.task-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 300px;
}
</style>
