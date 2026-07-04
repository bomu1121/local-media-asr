<script setup lang="ts">
import { computed } from "vue";
import {
  NLayout,
  NLayoutHeader,
  NLayoutSider,
  NLayoutContent,
  NMenu,
  NButton,
  NSpace,
  NText,
  NIcon,
  useMessage,
  type MenuOption,
} from "naive-ui";
import {
  FileTrayFullOutline,
  TimeOutline,
  SettingsOutline,
  MoonOutline,
  SunnyOutline,
  MicCircleOutline,
} from "@vicons/ionicons5";
import { h, type Component } from "vue";
import { useAppStore } from "../stores/app";
import FileManager from "./FileManager.vue";
import TranscriptionPanel from "./TranscriptionPanel.vue";
import SettingsPanel from "./SettingsPanel.vue";

const store = useAppStore();
const message = useMessage();

function renderIcon(icon: Component) {
  return () => h(NIcon, null, { default: () => h(icon) });
}

const menuOptions: MenuOption[] = [
  {
    label: "转写任务",
    key: "transcribe",
    icon: renderIcon(MicCircleOutline),
  },
  {
    label: "历史记录",
    key: "history",
    icon: renderIcon(TimeOutline),
  },
  {
    label: "设置",
    key: "settings",
    icon: renderIcon(SettingsOutline),
  },
];

const activeTab = computed(() => store.activeTab);
const isDark = computed(() => store.isDark);
</script>

<template>
  <NLayout :position="'absolute'" :has-sider="true">
    <!-- Sidebar -->
    <NLayoutSider
      bordered
      :width="220"
      :native-scrollbar="false"
      :show-trigger="false"
      collapse-mode="width"
    >
      <div class="sider-header">
        <NSpace :align="'center'" :size="8">
          <NIcon size="24" color="#2080f0">
            <FileTrayFullOutline />
          </NIcon>
          <NText strong style="font-size: 16px;">本地媒体ASR</NText>
        </NSpace>
      </div>
      <NMenu
        v-model:value="store.activeTab"
        :options="menuOptions"
        :default-value="'transcribe'"
      />
    </NLayoutSider>

    <!-- Main Content -->
    <NLayout>
      <NLayoutHeader bordered class="main-header">
        <div class="header-left">
          <NText depth="3" style="font-size: 13px;">
            离线运行 · 隐私安全
          </NText>
        </div>
        <div class="header-right">
          <NSpace :size="4">
            <NButton
              quaternary
              circle
              size="small"
              @click="store.toggleDark()"
            >
              <template #icon>
                <NIcon>
                  <SunnyOutline v-if="isDark" />
                  <MoonOutline v-else />
                </NIcon>
              </template>
            </NButton>
          </NSpace>
        </div>
      </NLayoutHeader>
      <NLayoutContent class="main-content">
        <FileManager v-if="activeTab === 'transcribe'" />
        <TranscriptionPanel v-else-if="activeTab === 'history'" />
        <SettingsPanel v-else-if="activeTab === 'settings'" />
      </NLayoutContent>
    </NLayout>
  </NLayout>
</template>

<style scoped>
.sider-header {
  padding: 16px 20px;
  border-bottom: 1px solid var(--n-border-color);
}

.main-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  height: 48px;
}

.main-content {
  padding: 20px;
  overflow-y: auto;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-right {
  display: flex;
  align-items: center;
}
</style>
