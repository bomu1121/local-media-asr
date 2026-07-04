<script setup lang="ts">
import { NConfigProvider, darkTheme, zhCN, dateZhCN } from "naive-ui";
import { useAppStore } from "./stores/app";
import MainLayout from "./components/MainLayout.vue";

const store = useAppStore();
</script>

<template>
  <NConfigProvider :theme="store.isDark ? darkTheme : undefined" :locale="zhCN" :date-locale="dateZhCN">
    <MainLayout />
  </NConfigProvider>
</template>

<style>
html, body, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  background: var(--n-color-body);
  color: var(--n-text-color);
}
</style>
