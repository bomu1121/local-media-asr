<script setup lang="ts">
import { computed } from "vue";
import { NText, NIcon } from "naive-ui";
import { useAppStore } from "../stores/app";
import FileManager from "./FileManager.vue";
import ResultPanel from "./ResultPanel.vue";
import TranscriptionPanel from "./TranscriptionPanel.vue";
import SettingsPanel from "./SettingsPanel.vue";
import { MicOutline, TimeOutline, SettingsOutline } from "@vicons/ionicons5";

const store = useAppStore();
const showResult = computed(() => store.activeTab === "transcribe" || store.activeTab === "history");
const tabs = [
  { key: "transcribe" as const, label: "转写", icon: MicOutline },
  { key: "history" as const, label: "历史", icon: TimeOutline },
  { key: "settings" as const, label: "设置", icon: SettingsOutline },
];
</script>

<template>
  <div class="app-shell">
    <div class="top-bar">
      <div class="top-left">
        <NText strong style="font-size:15px;color:#2080f0;">本地媒体ASR</NText>
        <NText depth="3" style="font-size:12px;">离线运行 · 隐私安全</NText>
      </div>
      <NText depth="3" style="font-size:12px;">
        {{ store.settings.engine === "fast" ? "快速引擎" : "精准引擎" }}
      </NText>
    </div>
    <div class="body-row">
      <div class="nav-col">
        <div class="nav-items">
          <div v-for="tab in tabs" :key="tab.key"
            class="nav-item" :class="{ active: store.activeTab === tab.key }"
            @click="store.activeTab = tab.key" :title="tab.label">
            <NIcon size="20"><component :is="tab.icon" /></NIcon>
            <span class="nav-label">{{ tab.label }}</span>
          </div>
        </div>
      </div>
      <div class="main-col">
        <FileManager v-if="store.activeTab === 'transcribe'" />
        <TranscriptionPanel v-else-if="store.activeTab === 'history'" />
        <SettingsPanel v-else-if="store.activeTab === 'settings'" />
      </div>
      <div v-if="showResult" class="result-col">
        <ResultPanel />
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-shell { display:flex;flex-direction:column;height:100%; }
.top-bar {
  height:48px;flex-shrink:0;display:flex;align-items:center;justify-content:space-between;
  padding:0 20px;border-bottom:1px solid var(--n-border-color);background:var(--n-color);
}
.top-left { display:flex;align-items:center;gap:12px; }
.body-row { flex:1;display:flex;min-height:0; }
.nav-col { width:56px;flex-shrink:0;display:flex;flex-direction:column;align-items:center;padding-top:12px;border-right:1px solid var(--n-border-color);background:var(--n-color); }
.nav-items { display:flex;flex-direction:column;gap:4px; }
.nav-item { display:flex;flex-direction:column;align-items:center;gap:2px;padding:8px 0;width:48px;border-radius:6px;cursor:pointer;color:var(--n-text-color-3);transition:all 0.15s; }
.nav-item:hover { background:var(--n-color-hover);color:var(--n-text-color); }
.nav-item.active { background:rgba(32,128,240,0.1);color:#2080f0; }
.nav-label { font-size:10px;line-height:1;user-select:none; }
.main-col { flex:1;min-width:0;overflow-y:auto;overflow-x:hidden;padding:16px 20px; }
.result-col { width:360px;max-width:420px;min-width:280px;flex-shrink:0;border-left:1px solid var(--n-border-color);background:var(--n-color); }
</style>
