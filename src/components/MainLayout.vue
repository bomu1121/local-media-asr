<script setup lang="ts">
import { computed } from "vue";
import {
  NLayout,
  NLayoutHeader,
  NLayoutSider,
  NLayoutContent,
  NSpace,
  NText,
  NIcon,
} from "naive-ui";
import { useAppStore } from "../stores/app";
import FileManager from "./FileManager.vue";

const store = useAppStore();
</script>

<template>
  <NLayout :position="'absolute'" :has-sider="true">
    <!-- Sidebar -->
    <NLayoutSider
      bordered
      :width="220"
      :show-trigger="false"
      collapse-mode="width"
    >
      <div class="sider-header">
        <NText strong style="font-size: 16px;color:#2080f0;">本地媒体ASR</NText>
      </div>
    </NLayoutSider>

    <!-- Main Content -->
    <NLayout>
      <NLayoutHeader bordered class="main-header">
        <NText depth="3" style="font-size:13px;">离线运行 · 隐私安全</NText>
      </NLayoutHeader>
      <NLayoutContent class="main-content">
        <FileManager />
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
