<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { NCard, NButton, NSpace, NText, NIcon, NTag, NProgress, NList, NListItem, NScrollbar, NPopconfirm, useMessage } from "naive-ui";
import { CloudUploadOutline, TrashOutline, PlayOutline, MusicalNotesOutline, FilmOutline, CheckmarkCircleOutline, CloseCircleOutline, TimeOutline, WarningOutline, FolderOpenOutline } from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { startTranscription, extractAudio, getMediaInfo, checkFfmpeg, downloadFfmpeg } from "../utils/invoke";
import { onExtractProgress } from "../utils/events";
import type { TaskFile } from '../stores/app';
import { onTranscribeProgress } from '../utils/events';
import type { TranscriptionResult } from '../utils/types';
import type { UnlistenFn } from "@tauri-apps/api/event";

const store = useAppStore();
const message = useMessage();
const tasks = computed(() => store.tasks);
const ffmpegReady = ref<boolean | null>(null);
const ffmpegVersion = ref("");
const downloadingFfmpeg = ref(false);
const transcribingTaskId = ref<string|null>(null);
const transcriptionResult = ref<TranscriptionResult|null>(null);
let unlistenProgress: UnlistenFn | null = null;

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024*1024) return `${(bytes/1024).toFixed(1)} KB`;
  if (bytes < 1024*1024*1024) return `${(bytes/(1024*1024)).toFixed(1)} MB`;
  return `${(bytes/(1024*1024*1024)).toFixed(2)} GB`;
}

function isAudio(format: string): boolean {
  return ["mp3","wav","flac","aac","ogg","wma","m4a","opus"].includes(format.toLowerCase());
}

function statusColor(status: TaskFile["status"]): string {
  const c: Record<string,string> = { pending:"default",extracting:"info",transcribing:"info",completed:"success",failed:"error" };
  return c[status]||"default";
}

function statusLabel(status: TaskFile["status"]): string {
  const l: Record<string,string> = { pending:"Waiting",extracting:"Extracting",transcribing:"Transcribing",completed:"Done",failed:"Failed" };
  return l[status]||status;
}

async function handleFileSelect() {
  const selected = await open({ multiple:true, filters:[{ name:"Media", extensions:["mp3","wav","flac","aac","ogg","wma","m4a","opus","mp4","mkv","avi","mov","wmv","flv","webm","m4v"] }] });
  if (!selected) return;
  const files = Array.isArray(selected) ? selected : [selected];
  for (const filePath of files) {
    const name = filePath.split("\\").pop()||filePath;
    const ext = name.split(".").pop()||"";
    store.addTask({ name, path: filePath, size: 0, format: ext });
    getMediaInfo(filePath).then(info => { const t = store.tasks.find(t2 => t2.path===filePath); if (t && info.size>0) t.size = info.size; }).catch(()=>{});
  }
  message.success(`Added ${files.length} file(s)`);
}

async function handleDownloadFfmpeg() {
  downloadingFfmpeg.value = true;
  message.info("Downloading FFmpeg (~80MB)...");
  try { await downloadFfmpeg(); ffmpegReady.value = true; message.success("Done"); }
  catch(e:any) { message.error(`Failed: ${e}`); }
  finally { downloadingFfmpeg.value = false; }
}

async function openFolder(filePath: string) {
  const dir = filePath.replace(/[^\\]+$/, "");
  try { await invoke("open_folder", { path: dir }); } catch { message.info(`Output: ${filePath}`); }
}

async function startExtraction(task: TaskFile) {
  if (!ffmpegReady.value) { message.error("FFmpeg not ready"); return; }
  store.selectTask(task.id);
  const baseName = task.name.replace(/\.[^.]+$/, "");
  const outputPath = task.path.replace(/[^\\]+$/, "") + baseName + "_extracted.wav";
  task.status = "extracting"; task.progress = 0;
  try {
    await extractAudio({ input_path:task.path, output_path:outputPath, denoise:store.settings.denoise });
    task.outputPath = outputPath;
    task.status = "completed"; task.progress = 100;
    message.success(`Done: ${task.name}`);
  } catch(e:any) { task.status="failed"; task.error=String(e); message.error(`Failed: ${e}`); }
}

onMounted(async () => {
  try { ffmpegReady.value = true; ffmpegVersion.value = await checkFfmpeg(); } catch { ffmpegReady.value = false; }
  try { unlistenProgress = await onExtractProgress(p => { const t = tasks.value.find(t2 => t2.id===store.activeTaskId); if (t && t.status==="extracting") t.progress = Math.round(p.progress*100); }); } catch(e) { console.warn(e); }
});

async function startTranscribing(task: TaskFile) {
  if (!task.outputPath) { message.error("No extracted audio found"); return; }
  transcribingTaskId.value = task.id;
  task.status = "transcribing"; task.progress = 0;
  try {
    transcriptionResult.value = await startTranscription(task.outputPath, store.settings.engine);
    task.result = transcriptionResult.value;
    task.status = "completed"; task.progress = 100;
    message.success("Transcription done");
  } catch(e:any) { task.status="failed"; task.error=String(e); message.error(`Transcription failed: ${e}`); }
  finally { transcribingTaskId.value = null; }
}

onUnmounted(() => { unlistenProgress?.(); });
</script>

<template>
  <div class="file-manager">
    <div v-if="ffmpegReady===false" class="ffmpeg-banner error">
      <NIcon size="18"><WarningOutline /></NIcon>
      <NText>FFmpeg not ready</NText>
      <NButton size="tiny" type="primary" @click="handleDownloadFfmpeg" :loading="downloadingFfmpeg">{{ downloadingFfmpeg?'Downloading...':'Auto Download' }}</NButton>
    </div>
    <div class="drop-zone-content" @click="handleFileSelect">
      <NIcon size="48" color="#999"><CloudUploadOutline /></NIcon>
      <NText depth="3" style="font-size:16px;margin-top:12px;">Click to select audio/video files</NText>
      <NText depth="3" style="font-size:13px;margin-top:4px;">MP3, WAV, MP4, MKV, FLAC, AAC...</NText>
    </div>
    <div v-if="tasks.length>0" class="task-section">
      <NText strong style="font-size:14px;margin-bottom:12px;display:block;">Tasks ({{ tasks.length }})</NText>
      <NScrollbar style="max-height:460px;">
        <NList hoverable clickable>
          <NListItem v-for="task in tasks" :key="task.id" @click="store.selectTask(task.id)">
            <template #prefix>
              <NIcon size="20" :color="task.status==='failed'?'#d03050':undefined"><MusicalNotesOutline v-if="isAudio(task.format)" /><FilmOutline v-else /></NIcon>
            </template>
            <div class="task-item">
              <div class="task-info">
                <NText class="task-name">{{ task.name }}</NText>
                <NSpace :size="8" style="margin-top:2px;">
                  <NText depth="3" style="font-size:12px;">{{ task.format.toUpperCase() }}</NText>
                  <NText depth="3" style="font-size:12px;">{{ formatSize(task.size) }}</NText>
                  <NText v-if="task.outputPath" depth="3" style="font-size:11px;max-width:200px;overflow:hidden;text-overflow:ellipsis;">{{ task.outputPath }}</NText>
                </NSpace>
                <NText v-if="task.result" depth="3" style="font-size:12px;white-space:pre-wrap;max-width:250px;overflow:hidden;text-overflow:ellipsis;">{{ (task.result.text||"").substring(0,100) }}{{ task.result.text?.length>100?"...":"" }}</NText>`n                <NProgress v-if="task.status==='extracting'||task.status==='transcribing'" :percentage="task.progress" :height="4" :border-radius="2" style="margin-top:6px;max-width:200px;" />
                <NText v-if="task.error" type="error" depth="3" style="font-size:12px;">{{ task.error }}</NText>
              </div>
              <NSpace :size="8" align="center">
                <NTag :type="statusColor(task.status)" size="small" :bordered="false">{{ statusLabel(task.status) }}</NTag>
                <NButton v-if="task.status==='pending'" size="tiny" type="primary" @click.stop="startExtraction(task)"><template #icon><NIcon><PlayOutline /></NIcon></template>Extract</NButton>`n                <NButton v-if="task.status==='completed'" size="tiny" type="primary" @click.stop="startTranscribing(task)"><template #icon><NIcon><PlayOutline /></NIcon></template>Transcribe</NButton>
                <NButton v-if="task.status==='completed'" size="tiny" type="primary" @click.stop="openFolder(task.outputPath||task.path)"><template #icon><NIcon><FolderOpenOutline /></NIcon></template>Open Folder</NButton>
                <NPopconfirm @positive-click="store.removeTask(task.id)">
                  <template #trigger><NButton size="tiny" quaternary type="error"><template #icon><NIcon><TrashOutline /></NIcon></template></NButton></template>
                  Delete?
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
.file-manager { display:flex;flex-direction:column;gap:16px;max-width:800px; }
.ffmpeg-banner { display:flex;align-items:center;gap:8px;padding:10px 16px;border-radius:6px;font-size:13px; }
.ffmpeg-banner.error { background:rgba(208,48,80,0.08);color:#d03050; }
.drop-zone-content { display:flex;flex-direction:column;align-items:center;justify-content:center;padding:48px 24px;cursor:pointer;border:2px dashed #d0d0d0;border-radius:8px;background:#fafafa; }
.drop-zone-content:hover { border-color:#2080f0;background:#f0f5ff; }
.task-section { margin-top:8px; }
.task-item { display:flex;align-items:center;justify-content:space-between;width:100%; }
.task-info { display:flex;flex-direction:column;min-width:0;flex:1; }
.task-name { overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:300px; }
</style>
