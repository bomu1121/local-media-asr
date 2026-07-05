<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { NButton, NSpace, NText, NIcon, NTag, NProgress, useMessage } from "naive-ui";
import { CloudUploadOutline, PlayOutline, MusicalNotesOutline, FilmOutline, FolderOpenOutline } from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { getMediaInfo, checkFfmpeg, downloadFfmpeg } from "../utils/invoke";
import { stat } from "@tauri-apps/plugin-fs";
import { Command } from "@tauri-apps/plugin-shell";
import { onExtractProgress, onTranscribeProgress } from "../utils/events";

import type { TaskFile, TranscriptionResult } from "../stores/app";
import type { UnlistenFn } from "@tauri-apps/api/event";

const store = useAppStore();
const message = useMessage();
const tasks = computed(() => store.tasks);
const ffmpegReady = ref<boolean | null>(null);
const downloadingFfmpeg = ref(false);
let unlistenExtract: UnlistenFn | null = null;
let unlistenTranscribe: UnlistenFn | null = null;

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024*1024) return `${(bytes/1024).toFixed(1)} KB`;
  if (bytes < 1024*1024*1024) return `${(bytes/(1024*1024)).toFixed(1)} MB`;
  return `${(bytes/(1024*1024*1024)).toFixed(2)} GB`;
}

function isAudio(format: string): boolean { return ["mp3","wav","flac","aac","ogg","wma","m4a","opus"].includes(format.toLowerCase()); }
function statusLabel(status: TaskFile["status"]): string {
  const l: Record<string,string> = { pending:"等待中", processing:"处理中", completed:"已完成", failed:"失败" };
  return l[status]||status;
}
function statusColor(status: TaskFile["status"]): "default"|"info"|"success"|"error" {
  const c: Record<string,string> = { pending:"default", processing:"info", completed:"success", failed:"error" };
  return (c[status]||"default") as "default"|"info"|"success"|"error";
}

async function handleFileSelect() {
  const selected = await open({ multiple:true, filters:[{ name:"Media", extensions:["mp3","wav","flac","aac","ogg","wma","m4a","opus","mp4","mkv","avi","mov","wmv","flv","webm","m4v"] }] });
  if (!selected) return;
  const files = Array.isArray(selected) ? selected : [selected];
  for (const filePath of files) {
    const name = filePath.split("\\").pop()||filePath;
    const ext = name.split(".").pop()||"";
    const task = store.addTask({ name, path: filePath, size: 0, format: ext });
    stat(filePath).then(s => {
      // Find and update via store to ensure reactivity
      const t = store.tasks.find(t2 => t2.id === task.id);
      if (t) t.size = s.size;
    }).catch((e: any) => { console.error("[stat] failed:", e); });
  }
  message.success(`已添加 ${files.length} 个文件`);
}

async function handleDownloadFfmpeg() {
  downloadingFfmpeg.value = true;
  try { await downloadFfmpeg(); ffmpegReady.value = true; message.success("FFmpeg 下载完成"); }
  catch(e:any) { message.error(`下载失败: ${e}`); }
  finally { downloadingFfmpeg.value = false; }
}

async function handleProcess(task: TaskFile) {
  if (!ffmpegReady.value) { message.error("FFmpeg 未就绪"); return; }
  store.selectTask(task.id);
  task.status = "processing"; task.progress = 0;

  try {
    // Step 1: Extract audio via Rust (FFmpeg)
    const ext = task.path.split(".").pop()?.toLowerCase() ?? "";
    const isWav = ext === "wav";
    let wavPath = task.path;

    if (!isWav) {
      const parent = task.path.replace(/[\\/][^\\/]+$/, "");
      const stem = task.path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "output";
      wavPath = parent + "\\" + stem + "_extracted.wav";

      await invoke("extract_audio", {
        args: { input_path: task.path, output_path: wavPath, denoise: false }
      });
    }

    task.progress = 30;
    // Step 2: ASR worker (Python in dev, sidecar exe in production)
    const resourceDir = await invoke("get_resource_path").catch(() => "");
    const useDev = !resourceDir;

    let cmd: ReturnType<typeof Command.create>;
    if (useDev) {
      // Dev: use python + asr_worker.py from src-tauri/
      cmd = Command.create("python", [
        "src-tauri\asr_worker.py",
        "--wav", wavPath,
        "--model", "paraformer",
        "--models-dir", "src-tauri\models",
      ]);
    } else {
      // Prod: use sidecar asr_worker.exe, models from resource dir
      cmd = Command.sidecar("binaries/asr_worker", [
        "--wav", wavPath,
        "--model", "paraformer",
        "--models-dir", resourceDir + "\models",
      ]);
    }

    // Use execute() to collect all output at once
    task.progress = 35; // show "running" while ASR works
    const output = await cmd.execute();

    if (output.code !== 0) {
      const errMsg = output.stderr || "(no stderr)";
      throw new Error("ASR worker exit code: " + output.code + "\n" + errMsg.substring(0, 500));
    }

    // Parse JSON lines from stdout
    let resultText = "";
    let resultSegments: any[] = [];
    const lines = (output.stdout || "").split("\n");
    for (const line of lines) {
      if (!line.trim()) continue;
      try {
    const msg = JSON.parse(line.trim());
    if (msg.type === "result") {
      resultText = msg.text;
      resultSegments = msg.segments;
    }
      } catch (_e: any) {}
    }

    if (!resultText) {
      throw new Error("ASR worker produced no output. stderr: " + (output.stderr || "(empty)").substring(0, 300));
    }

    task.result = {
      text: resultText, segments: resultSegments,
      engine: "paraformer", duration: 0,
    } as any;
    task.status = "completed";
    task.progress = 100;
    message.success("处理完成");
  } catch (e: any) {
    task.status = "failed"; task.error = String(e);
    message.error("处理失败: " + e);
  }
}

async function openFolder(filePath: string) {
  try { await invoke("open_folder", { path: filePath.replace(/[^\\]+$/, "") }); } catch (_e: any) {message.info(`路径: ${filePath}`); }
}

onMounted(async () => {
  try { ffmpegReady.value = true; await checkFfmpeg(); } catch (_e: any) {ffmpegReady.value = false; }
  try {
    unlistenExtract = await onExtractProgress(p => {
      const t = tasks.value.find(t2 => t2.id===store.activeTaskId);
      if (t) t.progress = Math.round(p.progress*30);
    });
    unlistenTranscribe = await onTranscribeProgress(p => {
      const t = tasks.value.find(t2 => t2.id===store.activeTaskId);
      if (t) t.progress = Math.round(30 + p.progress*70);
    });
  } catch(e) { console.warn(e); }
});
onUnmounted(() => { unlistenExtract?.(); unlistenTranscribe?.(); });
</script>

<template>
  <div class="file-manager">
    <div v-if="ffmpegReady===false" class="ffmpeg-banner">
      <NText style="font-size:13px;">FFmpeg 未就绪，请先下载</NText>
      <NButton size="tiny" type="primary" @click="handleDownloadFfmpeg" :loading="downloadingFfmpeg" class="banner-btn">
    {{ downloadingFfmpeg ? '下载中...' : '下载 FFmpeg' }}
      </NButton>
    </div>

    <div class="drop-zone" @click="handleFileSelect">
      <NIcon size="40" color="#aaa"><CloudUploadOutline /></NIcon>
      <NText depth="3" style="font-size:14px;margin-top:8px;">点击选择音视频文件</NText>
      <NText depth="3" style="font-size:11px;">MP3, WAV, MP4, MKV, FLAC, AAC...</NText>
    </div>

    <div v-if="tasks.length>0" class="task-section">
      <div v-for="task in tasks" :key="task.id" class="task-row"
    :class="{ selected: store.activeTaskId===task.id }"
    @click="store.selectTask(task.id)">
    <NIcon size="18" :color="task.status==='failed'?'#d03050':'#999'" class="task-icon">
      <MusicalNotesOutline v-if="isAudio(task.format)" /><FilmOutline v-else />
    </NIcon>
    <div class="task-info">
      <div class="task-top">
        <NText class="task-name">{{ task.name }}</NText>
        <NSpace :size="8" align="center" class="task-meta">
          <NText depth="3" style="font-size:11px;">{{ task.format.toUpperCase() }} · {{ formatSize(task.size) }}</NText>
          <NTag :type="statusColor(task.status)" size="tiny" :bordered="false">{{ statusLabel(task.status) }}</NTag>
        </NSpace>
      </div>
      <NProgress v-if="task.status==='processing'" :percentage="task.progress"
        :height="3" :border-radius="2" style="margin-top:4px;" />
      <NText v-if="task.error" type="error" depth="3" style="font-size:11px;">{{ task.error.substring(0,80) }}</NText>
    </div>
    <div class="task-actions">
      <NButton v-if="task.status==='pending'" size="tiny" type="primary"
        @click.stop="handleProcess(task)" class="action-btn">
        <template #icon><NIcon size="14"><PlayOutline /></NIcon></template>
      </NButton>
      <NButton v-if="task.status==='completed'" size="tiny" quaternary
        @click.stop="openFolder(task.path)" class="action-btn">
        <template #icon><NIcon size="14"><FolderOpenOutline /></NIcon></template>
      </NButton>
    </div>
      </div>
    </div>

    <div v-if="tasks.length===0 && ffmpegReady!==false" class="empty-hint">
      <NText depth="3" style="font-size:13px;">选择文件后点击 ? 即可一键提取音频并转写</NText>
    </div>
  </div>
</template>

<style scoped>
.file-manager { display:flex;flex-direction:column;gap:12px; }
.ffmpeg-banner { display:flex;align-items:center;justify-content:space-between;padding:10px 14px;border-radius:6px;background:rgba(208,48,80,0.06); }
.banner-btn:hover { opacity:0.85; }
.drop-zone { display:flex;flex-direction:column;align-items:center;justify-content:center;padding:36px 20px;cursor:pointer;border:2px dashed #d0d0d0;border-radius:8px;background:#fafafa;transition: all 0.15s; }
.drop-zone:hover { border-color:#2080f0;background:#f0f5ff; }
.task-section { display:flex;flex-direction:column;gap:4px; }
.task-row { display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:6px;cursor:pointer;background:var(--n-color);border:1px solid var(--n-border-color);transition: all 0.12s; }
.task-row:hover { border-color:#2080f0; }
.task-row.selected { border-color:#2080f0;background:rgba(32,128,240,0.04); }
.task-icon { flex-shrink:0;align-self:center; }
.task-info { flex:1;min-width:0;display:flex;flex-direction:column;gap:2px; }
.task-top { display:flex;align-items:center;justify-content:space-between;gap:8px; }
.task-name { overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:280px;font-size:13px; }
.task-meta { flex-shrink:0; }
.task-actions { flex-shrink:0;display:flex;align-items:center;gap:4px; }
.action-btn:hover { transform:scale(1.05); }
.empty-hint { text-align:center;padding:24px; }
</style>
