<script setup lang="ts">
import { LumContextMenu, LumModals } from "./components";
import { invoke } from "@tauri-apps/api/core";

window.addEventListener("keydown", async (event) => {
  if (event.key === "F12") {
    event.preventDefault();
    return;
  }

  if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "i") {
    event.preventDefault();
    await invoke("open_devtools");
  }
});
</script>

<template>
  <LumContextMenu />
  <LumModals />
</template>

<style scoped>

</style>
