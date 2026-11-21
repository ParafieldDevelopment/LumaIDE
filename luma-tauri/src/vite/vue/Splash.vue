<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import SplashScreen from "~/vue/SplashScreen.vue";

const FADE_MS = 400;
const leaving = ref(false);

// Rust emits "splash-finish" once the startup window is shown.
const listening: Promise<UnlistenFn> = listen("splash-finish", () => {
  leaving.value = true;
  window.setTimeout(() => void getCurrentWindow().close(), FADE_MS);
});

async function onReady() {
  // Make sure we can't miss the finish event, then ask Rust to show us.
  await listening;
  await invoke("webview_ready");
}

onBeforeUnmount(async () => (await listening)());
</script>

<template>
  <SplashScreen :leaving="leaving" :fade-out-ms="FADE_MS" @ready="onReady" />
</template>
