<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { NButton, NSpace, NText, NIcon, NTag, useMessage } from "naive-ui";
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
  const l: Record<string,string> = { pending:"开始提取", processing:"处理中", completed:"已完成", failed:"失败" };
  return l[status]||status;
}
function statusColor(status: TaskFile["status"]): "default"|"info"|"success"|"error" {
  const c: Record<string,string> = { pending:"info", processing:"info", completed:"success", failed:"error" };
  return (c[status]||"default") as "default"|"info"|"success"|"error";
}

function stageLabel(progress: number): string {
  if (progress < 30) return "提取音频";
  if (progress < 100) return "ASR 转写中";
  return "";
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
    let tempWav = false;

    if (!isWav) {
      const tempDir = await invoke<string>("get_temp_dir");
      const stem = task.path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "output";
      wavPath = tempDir + "\\asr_" + stem + "_" + crypto.randomUUID().slice(0, 8) + ".wav";
      tempWav = true;

      await invoke("extract_audio", {
        args: { input_path: task.path, output_path: wavPath, denoise: false }
      });
    }

    task.progress = 30;
    // Step 2: ASR worker (python in dev, Rust command in prod)
    const resourceDir = await invoke("get_resource_path").catch(() => "");
    const useDev = !resourceDir;

    task.progress = 35; // show "running" while ASR works

    let stdout: string;

    if (useDev) {
      // Dev: use python + asr_worker.py from src-tauri/
      const cmd = Command.create("python", [
        "src-tauri\asr_worker.py",
        "--wav", wavPath,
        "--model", "paraformer",
        "--models-dir", "src-tauri\models",
      ]);
      const output = await cmd.execute();
      if (output.code !== 0) {
        const errMsg = output.stderr || "(no stderr)";
        throw new Error("ASR worker exit code: " + output.code + "\n" + errMsg.substring(0, 500));
      }
      stdout = output.stdout;
    } else {
      // Prod: use Rust command with CREATE_NO_WINDOW (no black box!)
      stdout = await invoke("run_asr", {
        wavPath: wavPath,
        resourceDir: resourceDir,
      });
    }

    // Parse JSON lines from stdout
    let resultText = "";
    let resultSegments: any[] = [];
    const lines = (stdout || "").split("\n");
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
      throw new Error("ASR worker produced no output. The process completed but returned empty text.");
    }

    task.result = {
      text: resultText, segments: resultSegments,
      engine: "paraformer", duration: 0, refined: false,
    } as any;
    task.status = "completed";
    task.progress = 100;

    // Persist to history DB
    try {
      const { saveTranscription } = await import("../utils/invoke");
      await saveTranscription({
        task_id: task.id,
        name: task.name,
        file_path: task.path,
        file_size: task.size,
        file_format: task.format,
        engine: "paraformer",
        text: resultText,
        segments: resultSegments.map((s: any) => ({ start: s.start, end: s.end, text: s.text })),
        duration: 0,
      });
    } catch (e: any) {
      console.error("Failed to save to history:", e);
    }

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

    <div class="task-row-main">
      <NIcon size="16" :color="task.status==='failed'?'#d03050':'#999'" class="task-icon">
        <MusicalNotesOutline v-if="isAudio(task.format)" /><FilmOutline v-else />
      </NIcon>

      <NText class="task-name" :class="{ 'name-clickable': task.status==='completed' }" @click.stop="task.status==='completed' && openFolder(task.path)">{{ task.name }}</NText>

      <NText depth="3" class="task-meta">{{ task.format.toUpperCase() }} · {{ formatSize(task.size) }}</NText>

      <div class="task-spacer"></div>

<NTag v-if="task.status!=='processing'" :type="statusColor(task.status)" size="tiny" :bordered="false" :class="{ 'tag-btn': task.status==='pending' }" @click.stop="task.status==='pending' && handleProcess(task)">{{ statusLabel(task.status) }}</NTag>
<div v-else class="stage-inline">
  <span class="stage-track">
    <span class="stage-dot" :class="{ done: task.progress >= 30, active: task.progress < 30 }"></span>
    <span class="stage-dot" :class="{ done: task.progress >= 100, active: task.progress >= 30 && task.progress < 100 }"></span>
  </span>
  <span class="stage-label">{{ stageLabel(task.progress) }}</span>
</div>


    </div>

    <NText v-if="task.error" type="error" depth="3" style="font-size:11px;">{{ task.error.substring(0,80) }}</NText>
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

.task-row {
  display: flex;
  flex-direction: column;
  border-radius: 6px;
  cursor: pointer;
  padding: 10px 16px;
  border: 1px solid transparent;
  transition: background 0.15s, border-color 0.15s;
}
.task-row:hover { background: #F7F8FA; border-color: #e5e6eb; }
.task-row.selected { border-color: #2080f0; background: rgba(32,128,240,0.04); }

.task-row-main {
  display: flex;
  align-items: center;
  height: 28px;
}

.task-icon { flex-shrink: 0; }

.task-name {
  flex-shrink: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-left: 12px;
  font-size: 16px;
  font-weight: 500;
  color: #1D2129;
}

.task-meta {
  flex-shrink: 0;
  margin-left: 24px;
  font-size: 12px;
}

.task-spacer { flex: 1; min-width: 8px; }
.name-clickable { cursor: pointer; color: #1D2129; }
.name-clickable:hover { color: #2080f0; }

.action-btn { margin-left: 8px; }
.tag-btn { cursor: pointer; }
.tag-btn:hover { filter: brightness(0.9); }
.action-btn:hover { transform: scale(1.05); }

/* stage loading indicator (inline) */
.stage-inline {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.stage-track {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
.stage-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  background: #d8d9db;
  transition: background 0.3s;
}
.stage-dot.done { background: #18a058; }
.stage-dot.active {
  background: #2080f0;
  animation: stg-pulse 1.4s infinite ease-in-out;
}
.stage-label {
  font-size: 12px;
  color: #666;
  font-weight: 500;
  white-space: nowrap;
}
@keyframes stg-pulse {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.35); opacity: 0.65; }
}
.empty-hint { text-align:center;padding:24px; }
</style>