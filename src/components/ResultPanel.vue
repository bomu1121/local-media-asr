<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { NButton, NSpace, NText, NIcon, NDivider, useMessage } from "naive-ui";
import { CopyOutline, DownloadOutline, CheckmarkCircleOutline } from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import { exportResultString, saveExportFile } from "../utils/invoke";

const store = useAppStore();
const message = useMessage();
const previewFormat = ref("txt");
const previewText = ref("");
const exporting = ref(false);
const copied = ref(false);
const loadError = ref("");

const activeTask = computed(() => store.tasks.find(t => t.id === store.activeTaskId) ?? null);
const activeResult = computed(() => store.activeTab === "history" ? store.activeHistoryResult : activeTask.value?.result ?? null);

const tabs = [
  { key: "txt", label: "TXT" },
  { key: "srt", label: "SRT" },
  { key: "vtt", label: "VTT" },
  { key: "lrc", label: "LRC" },
  { key: "json", label: "JSON" },
];



const emptyReason = computed(() => {
  if (store.activeTab === "history") {
    if (!store.activeHistoryId) return "请选择一条历史记录查看详情";
    if (!store.activeHistoryResult) return "该历史记录无转写内容";
    return "";
  }
  if (!store.activeTaskId) return "请在左侧选择任务并点击 \u25b6 开始转写";
  const t = activeTask.value;
  if (!t) return "任务不存在";
  if (t.status === "processing") return "正在处理中...";
  if (t.status === "pending") return "任务尚未开始处理";
  if (t.status === "failed") return `处理失败: ${t.error ?? "未知错误"}`;
  if (!t.result) return "任务已完成但无转写结果";
  return "";
});

async function loadPreview() {
  if (!activeResult.value) { previewText.value = ""; loadError.value = ""; return; }
  loadError.value = "";
  try { previewText.value = await exportResultString(previewFormat.value, activeResult.value); }
  catch(e: any) { previewText.value = ""; loadError.value = `格式 "${previewFormat.value.toUpperCase()}" 暂不可用: ${e}`; }
}
watch(previewFormat, loadPreview);
watch(activeResult, loadPreview, { immediate: true });

async function handleCopy() {
  try {
    const text = await exportResultString(previewFormat.value, activeResult.value!);
    await navigator.clipboard.writeText(text);
    copied.value = true; message.success("已复制");
    setTimeout(() => copied.value = false, 2000);
  } catch { message.error("复制失败"); }
}
async function handleExport() {
  if (!activeResult.value) return;
  exporting.value = true;
  try {
    const ext = previewFormat.value;
    const outPath = `${store.settings.outputDir || "."}\\transcription_${Date.now()}.${ext === "txt" ? "txt" : ext}`;
    await saveExportFile(ext, outPath, activeResult.value);
    message.success("已导出");
  } catch(e: any) { message.error(`导出失败: ${e}`); }
  finally { exporting.value = false; }
}
</script>

<template>
  <div class="result-panel">
    <div class="result-tabs">
      <button v-for="tab in tabs" :key="tab.key"
        class="tab-btn" :class="{ active: previewFormat === tab.key }"
        @click="previewFormat = tab.key">
        {{ tab.label }}
      </button>
      <NSpace :size="4" class="tab-actions">
        <NButton size="tiny" quaternary @click="handleCopy" class="icon-btn">
          <template #icon><NIcon><CheckmarkCircleOutline v-if="copied" color="#18a058" /><CopyOutline v-else /></NIcon></template>
        </NButton>
        <NButton size="tiny" quaternary @click="handleExport" :loading="exporting" class="icon-btn">
          <template #icon><NIcon><DownloadOutline /></NIcon></template>
        </NButton>
      </NSpace>
    </div>

    <div class="result-body">
      <div v-if="emptyReason" class="result-empty">
        <NText depth="3" style="font-size:13px;">{{ emptyReason }}</NText>
      </div>
      <div v-else-if="loadError" class="result-empty">
        <NText type="error" depth="3" style="font-size:13px;">{{ loadError }}</NText>
      </div>
      <div v-else class="result-scroll">
        <div class="result-text">{{ previewText }}</div>
        <template v-if="activeResult?.segments?.length">
          <NDivider style="margin:12px 0;" />
          <NText depth="3" style="font-size:12px;display:block;margin-bottom:6px;">分段 ({{ activeResult.segments.length }})</NText>
          <div v-for="(seg,i) in activeResult.segments" :key="i" class="segment-row">
            <span class="seg-time">{{ seg.start.toFixed(1) }}s</span>
            <span class="seg-text">{{ seg.text }}</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.result-panel { display:flex;flex-direction:column;height:100%;background:#f0f1f3; }
.result-tabs {
  display:flex;align-items:center;gap:0;padding:0 14px;flex-shrink:0;
  background:#f0f1f3;border-bottom:1px solid #d8d9db;
}
.tab-btn {
  padding:8px 12px;font-size:12px;font-weight:500;border:none;background:none;
  color:#888;cursor:pointer;border-bottom:2px solid transparent;
  font-family:monospace;letter-spacing:0.5px;transition:all 0.12s;
}
.tab-btn:hover:not(:disabled) { color:#555; }
.tab-btn.active { color:#2080f0;border-bottom-color:#2080f0; }

.tab-actions { margin-left:auto;flex-shrink:0; }
.icon-btn:hover { color:#2080f0;background:rgba(32,128,240,0.06); }
.result-body { flex:1;display:flex;flex-direction:column;overflow:hidden; }
.result-empty { flex:1;display:flex;align-items:center;justify-content:center;padding:20px; }
.result-scroll { flex:1;overflow-y:auto;padding:16px 18px; }
.result-text { white-space:pre-wrap;word-break:break-word;font-size:13px;line-height:2;color:#333; }
.segment-row { display:flex;gap:8px;padding:3px 0; }
.seg-time { flex-shrink:0;width:46px;font-size:11px;color:#999;font-variant-numeric:tabular-nums; }
.seg-text { font-size:12px;line-height:1.7;color:#555; }
</style>
