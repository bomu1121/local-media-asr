<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  NCard,
  NButton,
  NSpace,
  NText,
  NIcon,
  NTag,
  NEmpty,
  NScrollbar,
  NSelect,
  NSpin,
  useMessage,
} from "naive-ui";
import {
  CopyOutline,
  DownloadOutline,
  TimeOutline,
  DocumentTextOutline,
  RefreshOutline,
} from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import { listHistory, deleteTask } from "../utils/invoke";
import type { TaskRecord } from "../utils/types";

const store = useAppStore();
const message = useMessage();
const historyTasks = ref<TaskRecord[]>([]);
const loading = ref(false);

const exportFormatOptions = [
  { label: "纯文本 (TXT)", value: "txt" },
  { label: "字幕文件 (SRT)", value: "srt" },
  { label: "网页字幕 (VTT)", value: "vtt" },
  { label: "歌词 (LRC)", value: "lrc" },
  { label: "结构化 (JSON)", value: "json" },
];

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function copyText(text: string) {
  navigator.clipboard.writeText(text).then(() => {
    message.success("已复制到剪贴板");
  });
}

async function loadHistory() {
  loading.value = true;
  try {
    historyTasks.value = await listHistory(50, 0);
  } catch (err: any) {
    message.error(`加载历史失败: ${err}`);
  } finally {
    loading.value = false;
  }
}

async function removeTask(taskId: string) {
  try {
    await deleteTask(taskId);
    historyTasks.value = historyTasks.value.filter((t) => t.id !== taskId);
    message.success("已删除");
  } catch (err: any) {
    message.error(`删除失败: ${err}`);
  }
}

function exportResult(taskId: string, format: string) {
  message.info(`导出中: ${format.toUpperCase()}...`);
  // Will invoke export Tauri command when transcription result is available
}

onMounted(() => {
  loadHistory();
});
</script>

<template>
  <div class="history-panel">
    <div class="panel-header">
      <NText strong style="font-size: 16px;">历史记录</NText>
      <NSpace :size="8">
        <NText depth="3" style="font-size: 13px;">
          共 {{ historyTasks.length }} 条
        </NText>
        <NButton size="tiny" quaternary @click="loadHistory">
          <template #icon><NIcon><RefreshOutline /></NIcon></template>
        </NButton>
      </NSpace>
    </div>

    <NSpin :show="loading">
      <NEmpty v-if="!loading && historyTasks.length === 0" description="暂无历史记录" />

      <NScrollbar v-else style="max-height: calc(100vh - 180px);">
        <NSpace vertical :size="16">
          <NCard
            v-for="task in historyTasks"
            :key="task.id"
            size="small"
            :bordered="true"
            :title="task.name"
          >
            <template #header-extra>
              <NSpace :size="8" align="center">
                <NTag size="tiny" :bordered="false" :type="task.status === 'completed' ? 'success' : 'default'">
                  {{ task.status === 'completed' ? '已完成' : task.status }}
                </NTag>
                <NTag size="tiny" :bordered="false" type="info" v-if="task.engine">
                  {{ task.engine === "fast" ? "快速引擎" : "精准引擎" }}
                </NTag>
                <NText depth="3" style="font-size: 12px;" v-if="task.result">
                  {{ formatDuration(task.result.duration_secs) }}
                </NText>
              </NSpace>
            </template>

            <div class="result-content">
              <NScrollbar style="max-height: 200px;">
                <NText class="result-text">
                  {{ task.result?.full_text || "无转写内容" }}
                </NText>
              </NScrollbar>

              <div class="result-actions">
                <NSpace :size="4">
                  <NButton
                    size="tiny"
                    quaternary
                    @click="copyText(task.result?.full_text || '')"
                  >
                    <template #icon>
                      <NIcon><CopyOutline /></NIcon>
                    </template>
                    复制
                  </NButton>
                  <NButton
                    size="tiny"
                    quaternary
                    type="error"
                    @click="removeTask(task.id)"
                  >
                    删除
                  </NButton>
                </NSpace>
              </div>
            </div>
          </NCard>
        </NSpace>
      </NScrollbar>
    </NSpin>
  </div>
</template>

<style scoped>
.history-panel {
  max-width: 800px;
}

.panel-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 16px;
}

.result-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.result-text {
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.8;
  font-size: 14px;
  color: var(--n-text-color-2);
}

.result-actions {
  border-top: 1px solid var(--n-border-color);
  padding-top: 8px;
}
</style>
