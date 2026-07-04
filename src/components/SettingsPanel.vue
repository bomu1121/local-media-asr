<script setup lang="ts">
import { ref } from "vue";
import {
  NCard,
  NButton,
  NSpace,
  NText,
  NIcon,
  NSwitch,
  NSelect,
  NSlider,
  NDivider,
  NTag,
  NInput,
  useMessage,
} from "naive-ui";
import {
  FlashOutline,
  GitBranchOutline,
  VolumeHighOutline,
  TextOutline,
  FolderOpenOutline,
  CheckmarkCircleOutline,
  CloudDownloadOutline,
} from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";
import { checkFfmpeg } from "../utils/invoke";
import type { EngineType, ExportFormat } from "../stores/app";

const store = useAppStore();
const message = useMessage();
const ffmpegStatus = ref<string | null>(null);
const checkingFfmpeg = ref(false);

const engineOptions = [
  {
    label: "快速引擎 (SenseVoice-Small)",
    value: "fast",
    description: "速度快, ~230MB, 中文CER 7.81%",
  },
  {
    label: "精准引擎 (Paraformer-Large)",
    value: "precise",
    description: "支持时间戳+字幕, ~227MB, 中文CER 10.18%",
  },
];

const exportOptions = [
  { label: "纯文本 TXT", value: "txt" },
  { label: "字幕 SRT", value: "srt" },
  { label: "网页字幕 VTT", value: "vtt" },
  { label: "歌词 LRC", value: "lrc" },
  { label: "结构化 JSON", value: "json" },
];

async function checkFFmpeg() {
  checkingFfmpeg.value = true;
  try {
    ffmpegStatus.value = await checkFfmpeg();
    message.success("FFmpeg 已就绪");
  } catch (err: any) {
    ffmpegStatus.value = null;
    message.error(String(err));
  } finally {
    checkingFfmpeg.value = false;
  }
}

function downloadModel(modelName: string) {
  message.info(`开始下载: ${modelName}`);
  // Will invoke download command in Phase 3-4
}
</script>

<template>
  <div class="settings-panel">
    <div class="panel-header">
      <NText strong style="font-size: 16px;">设置</NText>
    </div>

    <NSpace vertical :size="16">
      <!-- Engine Selection -->
      <NCard size="small" title="ASR引擎">
        <template #header-extra>
          <NTag size="tiny" type="info" :bordered="false">双引擎</NTag>
        </template>
        <NSpace vertical :size="12">
          <NSelect
            v-model:value="store.settings.engine"
            :options="engineOptions"
            placeholder="选择引擎"
          />
          <NText depth="3" style="font-size: 12px;" v-if="store.settings.engine === 'fast'">
            快速引擎适合日常转写。需要字幕/时间戳时请切换到精准引擎。
          </NText>
          <NText depth="3" style="font-size: 12px;" v-else>
            精准引擎支持字符级时间戳，适合字幕制作和需要精确时间轴的场景。
          </NText>
        </NSpace>
      </NCard>

      <!-- VAD Settings -->
      <NCard size="small" title="智能语音分段 (VAD)">
        <NSpace vertical :size="12">
          <div class="setting-row">
            <NText>启用VAD智能分段</NText>
            <NSwitch v-model:value="store.settings.vad.enabled" />
          </div>
          <div class="setting-row" v-if="store.settings.vad.enabled">
            <NText>VAD灵敏度</NText>
            <div class="setting-control">
              <NSlider
                v-model:value="store.settings.vad.threshold"
                :min="0.1"
                :max="0.9"
                :step="0.05"
                style="width: 200px;"
              />
              <NText depth="3" style="font-size: 12px; width: 40px;">
                {{ store.settings.vad.threshold.toFixed(2) }}
              </NText>
            </div>
          </div>
          <div class="setting-row" v-if="store.settings.vad.enabled">
            <NText>最小语音段 (秒)</NText>
            <NSlider
              v-model:value="store.settings.vad.minSpeechDuration"
              :min="0.1"
              :max="2.0"
              :step="0.1"
              style="width: 200px;"
            />
          </div>
          <div class="setting-row" v-if="store.settings.vad.enabled">
            <NText>最大段时长 (秒)</NText>
            <NSlider
              v-model:value="store.settings.vad.maxSegmentDuration"
              :min="15"
              :max="120"
              :step="5"
              style="width: 200px;"
            />
          </div>
        </NSpace>
      </NCard>

      <!-- Audio Processing -->
      <NCard size="small" title="音频处理">
        <NSpace vertical :size="12">
          <div class="setting-row">
            <NText>启用降噪</NText>
            <NSwitch v-model:value="store.settings.denoise" />
          </div>
        </NSpace>
      </NCard>

      <!-- Text Processing -->
      <NCard size="small" title="文本处理">
        <NSpace vertical :size="12">
          <div class="setting-row">
            <NText>智能标点恢复</NText>
            <NSwitch v-model:value="store.settings.punctuation" />
          </div>
          <NText depth="3" style="font-size: 12px;">
            自动恢复句号、逗号、问号等标点，提升转写结果可读性
          </NText>
        </NSpace>
      </NCard>

      <!-- Export -->
      <NCard size="small" title="导出设置">
        <NSpace vertical :size="12">
          <div class="setting-row">
            <NText>默认导出格式</NText>
            <NSelect
              v-model:value="store.settings.exportFormat"
              :options="exportOptions"
              style="width: 160px;"
              size="small"
            />
          </div>
          <div class="setting-row">
            <NText>导出目录</NText>
            <NInput
              v-model:value="store.settings.outputDir"
              placeholder="默认: 用户文档目录"
              size="small"
              style="width: 240px;"
            />
          </div>
        </NSpace>
      </NCard>

      <!-- System Status -->
      <NCard size="small" title="系统状态">
        <NSpace vertical :size="8">
          <div class="setting-row">
            <NSpace :size="8" align="center">
              <NIcon size="16" color="#18a058">
                <CheckmarkCircleOutline v-if="false" />
              </NIcon>
              <NText>FFmpeg Sidecar</NText>
            </NSpace>
            <NSpace :size="8" align="center">
              <NText depth="3" style="font-size: 12px;" v-if="ffmpegStatus">
                {{ ffmpegStatus }}
              </NText>
              <NButton size="tiny" @click="checkFFmpeg">检测</NButton>
            </NSpace>
          </div>
          <NDivider style="margin: 4px 0;" />
          <div class="setting-row">
            <NSpace :size="8" align="center">
              <NText>模型状态</NText>
            </NSpace>
            <NSpace :size="4">
              <NTag size="tiny" type="warning" :bordered="false">SenseVoice: 未下载</NTag>
              <NTag size="tiny" :bordered="false">Paraformer: 未下载</NTag>
              <NTag size="tiny" :bordered="false">VAD: 未下载</NTag>
            </NSpace>
          </div>
        </NSpace>
      </NCard>
    </NSpace>
  </div>
</template>

<style scoped>
.settings-panel {
  max-width: 700px;
}

.panel-header {
  margin-bottom: 16px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.setting-control {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
