<script setup lang="ts">
import { NMessageProvider } from "naive-ui";
import MainLayout from "./components/MainLayout.vue";
</script>

<template>
  <NMessageProvider>
    <MainLayout />
  </NMessageProvider>
</template>

<style>
html, body, #app { margin:0;padding:0;height:100%;width:100%;overflow:hidden;background:#f5f5f5;color:#333;font-family:system-ui,sans-serif; }
</style>
