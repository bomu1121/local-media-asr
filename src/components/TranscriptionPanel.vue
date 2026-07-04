<script setup lang="ts">
import { computed } from "vue";
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
  useMessage,
} from "naive-ui";
import {
  CopyOutline,
  DownloadOutline,
  TimeOutline,
  DocumentTextOutline,
} from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import type { ExportFormat } from "../stores/app";

const store = useAppStore();
const message = useMessage();

const tasks = computed(() => store.tasks);
const completedTasks = computed(() => tasks.value.filter((t) => t.status === "completed"));

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

function exportResult(taskId: string, format: ExportFormat) {
  message.info(`导出中: ${format.toUpperCase()}...`);
}
</script>

<template>
  <div class="history-panel">
    <div class="panel-header">
      <NText strong style="font-size: 16px;">历史记录</NText>
      <NText depth="3" style="font-size: 13px;">
        共 {{ completedTasks.length }} 条已完成任务
      </NText>
    </div>

    <NEmpty v-if="completedTasks.length === 0" description="暂无历史记录，先完成一次转写吧" />

    <NScrollbar v-else style="max-height: calc(100vh - 180px);">
      <NSpace vertical :size="16">
        <NCard
          v-for="task in completedTasks"
          :key="task.id"
          size="small"
          :bordered="true"
          :title="task.name"
        >
          <template #header-extra>
            <NSpace :size="8" align="center">
              <NTag size="tiny" :bordered="false" type="success">已完成</NTag>
              <NTag size="tiny" :bordered="false" type="info" v-if="task.result">
                {{ task.result.engine === "fast" ? "快速引擎" : "精准引擎" }}
              </NTag>
              <NText depth="3" style="font-size: 12px;" v-if="task.result">
                {{ formatDuration(task.result.duration) }}
              </NText>
            </NSpace>
          </template>

          <div class="result-content">
            <NScrollbar style="max-height: 200px;">
              <NText class="result-text">
                {{ task.result?.text || "无内容" }}
              </NText>
            </NScrollbar>

            <div class="result-actions">
              <NSpace :size="4">
                <NButton
                  size="tiny"
                  quaternary
                  @click="copyText(task.result?.text || '')"
                >
                  <template #icon>
                    <NIcon><CopyOutline /></NIcon>
                  </template>
                  复制
                </NButton>
                <NSelect
                  size="tiny"
                  :value="store.settings.exportFormat"
                  :options="exportFormatOptions"
                  style="width: 140px;"
                  placeholder="导出格式"
                />
                <NButton
                  size="tiny"
                  quaternary
                  type="primary"
                  @click="exportResult(task.id, store.settings.exportFormat)"
                >
                  <template #icon>
                    <NIcon><DownloadOutline /></NIcon>
                  </template>
                  导出
                </NButton>
              </NSpace>
            </div>
          </div>
        </NCard>
      </NSpace>
    </NScrollbar>
  </div>
</template>

<style scoped>
.history-panel {
  max-width: 800px;
}

.panel-header {
  display: flex;
  align-items: baseline;
  gap: 12px;
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
