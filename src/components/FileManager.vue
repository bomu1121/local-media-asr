<script setup lang="ts">
import { ref, computed } from "vue";
import {
  NUpload,
  NUploadDragger,
  NCard,
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
  NEmpty,
  useMessage,
  type UploadOnFinish,
  type UploadOnError,
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
  DocumentTextOutline,
} from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import type { TaskFile } from "../stores/app";

const store = useAppStore();
const message = useMessage();

const acceptFormats =
  ".mp3,.wav,.flac,.aac,.ogg,.wma,.m4a,.opus,.mp4,.mkv,.avi,.mov,.wmv,.flv,.webm,.m4v";

const tasks = computed(() => store.tasks);

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

function statusIcon(status: TaskFile["status"]) {
  switch (status) {
    case "pending":
      return TimeOutline;
    case "completed":
      return CheckmarkCircleOutline;
    case "failed":
      return CloseCircleOutline;
    case "extracting":
    case "transcribing":
      return CloudUploadOutline;
  }
}

function statusColor(status: TaskFile["status"]): string {
  switch (status) {
    case "pending":
      return "default";
    case "completed":
      return "success";
    case "failed":
      return "error";
    case "extracting":
    case "transcribing":
      return "info";
  }
}

function statusLabel(status: TaskFile["status"]): string {
  switch (status) {
    case "pending":
      return "等待中";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
    case "extracting":
      return "提取音频";
    case "transcribing":
      return "转写中";
  }
}

// Handle native drag-and-drop from Tauri
async function onDropFiles(e: DragEvent) {
  const files = e.dataTransfer?.files;
  if (!files) return;
  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    const ext = file.name.split(".").pop() || "";
    store.addTask({
      name: file.name,
      path: (file as any).path || file.name,
      size: file.size,
      format: ext,
    });
  }
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
}

function startTranscription(task: TaskFile) {
  store.selectTask(task.id);
  message.info(`开始转写: ${task.name}`);
  // Will invoke Tauri command in Phase 3
}
</script>

<template>
  <div class="file-manager" @drop.prevent="onDropFiles" @dragover.prevent>
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
      <NScrollbar style="max-height: 400px;">
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
              </div>
              <NSpace :size="8" align="center">
                <NTag
                  :type="statusColor(task.status)"
                  size="small"
                  :bordered="false"
                >
                  <template #icon>
                    <NIcon size="14">
                      <component :is="statusIcon(task.status)" />
                    </NIcon>
                  </template>
                  {{ statusLabel(task.status) }}
                </NTag>
                <NButton
                  v-if="task.status === 'completed'"
                  size="tiny"
                  quaternary
                  type="primary"
                  @click.stop="startTranscription(task)"
                >
                  <template #icon>
                    <NIcon><DocumentTextOutline /></NIcon>
                  </template>
                  查看
                </NButton>
                <NButton
                  v-if="task.status === 'pending'"
                  size="tiny"
                  type="primary"
                  @click.stop="startTranscription(task)"
                >
                  <template #icon>
                    <NIcon><PlayOutline /></NIcon>
                  </template>
                  开始
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
  gap: 24px;
  max-width: 800px;
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
}

.task-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 300px;
}
</style>
