<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { NButton, NSpace, NText, NIcon, NDivider, NTag, useMessage } from "naive-ui";
import { CopyOutline, DownloadOutline, CheckmarkCircleOutline, SparklesOutline } from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";

const store = useAppStore();
const message = useMessage();
const previewFormat = ref("txt");
const copying = ref(false);
const exporting = ref(false);

// Single source of truth: the final display text (raw ASR, progressively replaced by AI-refined chunks)
const displayText = ref("");
const displaySegments = ref<Array<{ start: number; end: number; text: string }>>([]);
const loadError = ref("");

// Refinement state
const refineProgress = ref({ done: 0, total: 0 }); // chunk-level progress
const refineFailed = ref(false); // true if any chunk failed (falls back to raw)

const activeTask = computed(() => store.tasks.find(t => t.id === store.activeTaskId) ?? null);
const activeResult = computed(() =>
  store.activeTab === "history" ? store.activeHistoryResult : activeTask.value?.result ?? null
);

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
  if (!store.activeTaskId) return "请在左侧选择任务并点击 ▶ 开始转写";
  const t = activeTask.value;
  if (!t) return "任务不存在";
  if (t.status === "processing") return "正在处理中...";
  if (t.status === "pending") return "任务尚未开始处理";
  if (t.status === "failed") return `处理失败: ${t.error ?? "未知错误"}`;
  if (!t.result) return "任务已完成但无转写结果";
  return "";
});

// ---- DeepSeek API (frontend-only, no Rust dep) ----
const SYSTEM_PROMPT = [
  "你是一个顶级的语音转文字校对专家，专门处理中文口播视频的ASR（自动语音识别）后处理。",
  "你的任务是接收一段充满错误的原始转录文本，输出高质量、可直接发布的口播文案。",
  "在开始修正之前，请先快速通读全文，理解视频的主题、说话人的身份和语境，然后用这个整体理解来指导每一个修正决策。",
  "",
  "# 核心原则：分层修正",
  "请按以下优先级逐层处理，每一层的输出作为下一层的输入：",
  "",
  "## 第一层：字面纠错（必须做）",
  "- 纠正同音错字：根据词语搭配和常见用法判断（如赫名昭著→臭名昭著、黑力→红利、攒机→装机）",
  "- 修正数字错误：如上百亿token在中文AI语境下应为上百万token或几百万token，根据常识判断",
  "- 补全标点：句号、逗号、问号、感叹号、引号按语义添加，去除无意义的断句换行",
  "",
  "## 第二层：专有名词恢复（尽力做）",
  "- AI产品/公司：Claude Code、GPT-5.6、GLM-5.2、Cloud Opus/Office 4.6、Codex、Anthropic、OpenAI、智谱、MiniMax、Kimi、DeepSeek",
  "- 技术术语：AI Agent、Web Coding、Vibe Coding、多模态、token、GitHub、Star、VS Code",
  "- 网站/品牌：Reddit、Awards、One Page Love、Mobbin、Reform、Stripe、Linear、Lovable",
  "- 识别线索：ASR经常把英文名识别成读音相近的中文碎片（如SAP=Anthropic、SROP=Anthropic、SYOPIC=Anthropic、飞过=Sonnet）。当看到一串无意义的中文字符出现在英文专有名词应该出现的位置时，根据上下文推断最可能的原名",
  "",
  "## 第三层：语句重塑（谨慎做）",
  "- 将因ASR断句导致的破碎短句合并为完整句子",
  "- 删除因语音卡顿导致的重复词和口语赘余（如那那那个→那个、怎怎怎么→怎么）",
  "- 如果一句话被截断但因上下文可以推断完整意思，补全它（如这是一个设→这是一个设计）",
  "- 如果一句话完全不通且无法推断，直接删除，不要强行编造",
  "",
  "## 第四层：结构优化",
  "- 按语义转折划分段落，每段3-6句话",
  "- 保留说话人的语气词和口语风格（啊、呢、吧、对吧、哼等），这是口播稿的特色",
  "- 不要刻意美化或改写，忠实于原始表达",
  "",
  "# 关键规则",
  "1. 遇到无法确定的专有名词，保留最接近原文的读音形式，不要随意替换",
  "2. 不要添加原文中不存在的信息",
  "3. 不要删除有实际内容的话，只删除无意义的语音碎片",
  "4. 输出时不要添加任何解释、说明或标记，直接输出最终文本",
  "",
  "直接返回修正后的完整文本，不要加任何解释、前言或后缀。",
].join("\n");

async function callDeepSeekAPI(rawText: string, apiKey: string): Promise<string> {
  const resp = await fetch("https://api.deepseek.com/chat/completions", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "deepseek-chat",
      messages: [
        {
          role: "system",
          content: SYSTEM_PROMPT,
        },
        { role: "user", content: rawText },
      ],
      temperature: 0.3,
      max_tokens: 16000,
    }),
  });
  if (!resp.ok) {
    throw new Error(`DeepSeek API error (${resp.status}): ${await resp.text()}`);
  }
  const data = await resp.json();
  if (!data.choices?.[0]?.message?.content) {
    throw new Error("Unexpected API response format");
  }
  return data.choices[0].message.content;
}

// ---- AI refinement ----
let hasAutoRefined = false;
async function runRefine() {
  const raw = displayText.value;
  if (!raw || !store.settings.aiApiKey) return;
  refineProgress.value = { done: 0, total: 1 };
  refineFailed.value = false;
  try {
    const refined = await callDeepSeekAPI(raw, store.settings.aiApiKey);
    displayText.value = refined;
    refineProgress.value = { done: 1, total: 1 };
    // Persist to store
    const task = activeTask.value;
    if (task?.result) {
      task.result.text = refined;
    }
  } catch (e: any) {
    console.error('Refine failed:', e);
    refineFailed.value = true;
  }
}

// ---- Format conversion helpers ----
function fmtHms(s: number): string { const h=Math.floor(s/3600),m=Math.floor((s%3600)/60),sec=Math.floor(s%60),ms=Math.floor((s%1)*1000); return String(h).padStart(2,'0')+':'+String(m).padStart(2,'0')+':'+String(sec).padStart(2,'0')+','+String(ms).padStart(3,'0'); }
function fmtHmsVtt(s: number): string { const h=Math.floor(s/3600),m=Math.floor((s%3600)/60),sec=Math.floor(s%60),ms=Math.floor((s%1)*1000); return String(h).padStart(2,'0')+':'+String(m).padStart(2,'0')+':'+String(sec).padStart(2,'0')+'.'+String(ms).padStart(3,'0'); }
function fmtLrc(s: number): string { const m=Math.floor(s/60),sec=(s%60).toFixed(2); return '['+String(m).padStart(2,'0')+':'+String(Number(sec)<10?'0':'')+sec+']'; }
const formattedText = computed(()=>{const t=displayText.value,segs=displaySegments.value;switch(previewFormat.value){case'txt':return t;case'srt':return segs.length?segs.map((s,i)=>String(i+1)+'\n'+fmtHms(s.start)+' --> '+fmtHms(s.end)+'\n'+s.text+'\n').join('\n'):t;case'vtt':return segs.length?'WEBVTT\n\n'+segs.map((s,i)=>String(i+1)+'\n'+fmtHmsVtt(s.start)+' --> '+fmtHmsVtt(s.end)+'\n'+s.text+'\n').join('\n'):t;case'lrc':return segs.length?segs.map(s=>fmtLrc(s.start)+s.text).join('\n'):t;case'json':return JSON.stringify({text:t,segments:segs},null,2);default:return t;}});

// ---- Watch: when a new transcription result arrives, start streamed refinement ----
watch(activeResult, (result) => {
  displayText.value = "";
  displaySegments.value = [];
  refineProgress.value = { done: 0, total: 0 };
  refineFailed.value = false;
  loadError.value = "";

  if (!result) return;

  // If AI refine enabled: wait, don't show raw text
  if (store.settings.enableAiRefine && store.settings.aiApiKey) {
    // Don't set displayText yet - streamRefine will populate it as chunks complete
    runRefine();
  } else {
    // No AI: show raw text directly
    displayText.value = result.text;
    displaySegments.value = result.segments ?? [];
  }
}, { immediate: true });

// ---- Actions ----
async function handleCopy() {
  try {
    await navigator.clipboard.writeText(formattedText.value);
    copying.value = true;
    message.success("已复制");
    setTimeout(() => (copying.value = false), 2000);
  } catch (_e: any) {
    message.error("复制失败");
  }
}

async function handleExport() {
  const result = activeResult.value;
  if (!result) return;
  exporting.value = true;
  try {
    const { exportResultString, saveExportFile } = await import("../utils/invoke");
    const ext = previewFormat.value;
    const updatedResult = { ...result, text: displayText.value };
    const outPath = `${store.settings.outputDir || "."}\\transcription_${Date.now()}.${ext}`;
    await saveExportFile(ext, outPath, updatedResult);
    message.success("已导出");
  } catch (e: any) {
    message.error(`导出失败: ${e}`);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div class="result-panel">
    <div class="result-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="tab-btn"
        :class="{ active: previewFormat === tab.key }"
        @click="previewFormat = tab.key"
      >
        {{ tab.label }}
      </button>
      <NSpace :size="4" class="tab-actions">
        <NButton v-if="displayText && store.settings.aiApiKey && !(refineProgress.done >= refineProgress.total && !refineFailed)" size="tiny" quaternary @click="runRefine" class="icon-btn" title="AI ??"><template #icon><NIcon color="#7c3aed"><SparklesOutline /></NIcon></template></NButton>
        <NButton size="tiny" quaternary @click="handleCopy" :disabled="!displayText" class="icon-btn" title="复制">
          <template #icon>
            <NIcon><CheckmarkCircleOutline v-if="copying" color="#18a058" /><CopyOutline v-else /></NIcon>
          </template>
        </NButton>
        <NButton size="tiny" quaternary @click="handleExport" :loading="exporting" :disabled="!displayText" class="icon-btn" title="导出">
          <template #icon><NIcon><DownloadOutline /></NIcon></template>
        </NButton>
      </NSpace>
    </div>

    <div class="result-body">
      <!-- Empty / Error state -->
      <div v-if="emptyReason" class="result-empty">
        <NText depth="3" style="font-size: 13px">{{ emptyReason }}</NText>
      </div>
      <div v-else-if="loadError" class="result-empty">
        <NText type="error" depth="3" style="font-size: 13px">{{ loadError }}</NText>
      </div>

      <!-- Content -->
      <div v-else class="result-scroll">
                <!-- AI refine progress (animated dots) -->
        <div v-if="refineProgress.total > 0 && refineProgress.done < refineProgress.total" class="refine-status">
          <div class="refine-dots"><span class="rdot"></span><span class="rdot"></span><span class="rdot"></span></div>
          <NText style="font-size:13px;color:#7c3aed;">AI ???...</NText>
        </div>
        <div v-else-if="refineFailed" style="padding:8px 14px;">
          <NTag type="warning" size="small" :bordered="false">AI ???????????</NTag>
        </div>
        <div v-else-if="refineProgress.done >= refineProgress.total && !refineFailed" style="padding:6px 14px 0;">
          <NTag type="success" size="tiny" :bordered="false"><template #icon><NIcon size="12"><SparklesOutline /></NIcon></template>AI ???</NTag>
        </div>

<!-- Main text display -->
        <div class="result-text">{{ formattedText }}</div>

        <!-- Segments (time-stamped) -->
        <template v-if="displaySegments.length > 0">
          <NDivider style="margin: 12px 0" />
          <NText depth="3" style="font-size: 12px; display: block; margin-bottom: 6px">
            分段 ({{ displaySegments.length }})
          </NText>
          <div v-for="(seg, i) in displaySegments" :key="i" class="segment-row">
            <span class="seg-time">{{ seg.start.toFixed(1) }}s</span>
            <span class="seg-text">{{ seg.text }}</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.result-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #f0f1f3;
}
.result-tabs {
  display: flex;
  align-items: center;
  gap: 0;
  padding: 0 14px;
  flex-shrink: 0;
  background: #f0f1f3;
  border-bottom: 1px solid #d8d9db;
}
.tab-btn {
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 500;
  border: none;
  background: none;
  color: #888;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  font-family: monospace;
  letter-spacing: 0.5px;
  transition: all 0.12s;
}
.tab-btn:hover:not(:disabled) {
  color: #555;
}
.tab-btn.active {
  color: #2080f0;
  border-bottom-color: #2080f0;
}
.tab-actions {
  margin-left: auto;
  flex-shrink: 0;
}
.icon-btn:hover {
  color: #2080f0;
  background: rgba(32, 128, 240, 0.06);
}
.result-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.result-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}
.result-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 16px 18px;
}
.result-text {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 13px;
  line-height: 2;
  color: #333;
}
.segment-row {
  display: flex;
  gap: 8px;
  padding: 3px 0;
}
.seg-time {
  flex-shrink: 0;
  width: 46px;
  font-size: 11px;
  color: #999;
  font-variant-numeric: tabular-nums;
}
.seg-text {
  font-size: 12px;
  line-height: 1.7;
  color: #555;
}

/* AI refine dots */
.refine-status {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}
.refine-dots { display: flex; gap: 6px; margin-bottom: 12px; }
.rdot {
  width: 8px; height: 8px; border-radius: 50%;
  background: #7c3aed;
  animation: bounce 1.4s infinite ease-in-out both;
}
.rdot:nth-child(1) { animation-delay: -0.32s; }
.rdot:nth-child(2) { animation-delay: -0.16s; }
@keyframes bounce {
  0%, 80%, 100% { transform: scale(0.6); opacity: 0.4; }
  40% { transform: scale(1); opacity: 1; }
}
</style>
