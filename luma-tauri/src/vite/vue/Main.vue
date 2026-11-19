<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { LumContextMenu, LumModals } from "./components";
import TitleBar from "~/vue/window/TitleBar.vue";

interface ActiveWorkspace {
  project_uuid: string;
  title: string;
  path: string;
  git_branch?: string;
  icon_path?: string;
}

const activeWorkspace = ref<ActiveWorkspace | null>(null);
const loadError = ref("");
const unlistenWorkspace = ref<UnlistenFn | null>(null);


onMounted(async () => {
  unlistenWorkspace.value = await listen<ActiveWorkspace>("workspace-opened", (event) => {
    activeWorkspace.value = event.payload;
  });
  try {
    activeWorkspace.value = await invoke<ActiveWorkspace | null>("get_active_workspace");
  } catch (error) {
    loadError.value = `Unable to retrieve the active workspace: ${String(error)}`;
  }
});


onBeforeUnmount(() => {
  unlistenWorkspace.value?.();
});

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
  <TitleBar />
  <main class="workspace-shell">
    <section v-if="activeWorkspace" class="workspace-card">
      <p class="eyebrow">ACTIVE WORKSPACE</p>
      <h1>{{ activeWorkspace.title }}</h1>
      <p class="path">{{ activeWorkspace.path }}</p>
      <p class="status">Workspace loaded successfully. The editor surface will be connected in a later phase.</p>
    </section>
    <p v-else-if="loadError" class="error">{{ loadError }}</p>
    <p v-else class="loading">Loading workspace…</p>
  </main>
  <LumContextMenu />
  <LumModals />
</template>

<style scoped>
.workspace-shell {
  height: calc(100% - 42px);
  display: flex;
  align-items: flex-start;
  padding: 1.5rem;
  overflow: auto;
}

.workspace-card {
  width: 100%;
  padding: 1.25rem;
  border: 1px solid var(--lum-border);
  border-radius: 12px;
}

.eyebrow {
  margin: 0 0 0.5rem;
  opacity: 0.7;
  font-size: 0.75rem;
  letter-spacing: 0.08em;
}

h1 {
  margin: 0;
}

.path {
  opacity: 0.75;
  overflow-wrap: anywhere;
}

.status {
  margin-top: 1.25rem;
}

.error {
  color: var(--lum-danger, #e66);
  overflow-wrap: anywhere;
}
</style>
