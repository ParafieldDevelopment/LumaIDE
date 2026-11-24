<script setup lang="ts">
import { useWindow } from "~/vue/composables/useWindow.ts";
import TitleBar from "~/vue/window/TitleBar.vue";
import { onMounted, ref } from "vue";
import { getAppVersion, loadTheme, webviewReady } from "~/luma/LumaWindow.ts";
import { LumContextMenu, LumModals, LumToasts, LumCard } from "~/vue/components";
import { invoke } from "@tauri-apps/api/core";
import DiscordRpcSettings from "~/vue/extensions/DiscordRpcSettings.vue";

const activeTab = ref<"extensions" | "general">("extensions");
const appVersion = ref("");

onMounted(async () => {
  await loadTheme();
  appVersion.value = await getAppVersion();
  await webviewReady();
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
  <LumContextMenu />
  <TitleBar contentClass="startup-title">
    <div class="title">
      <img
        src="/public/logo.svg"
        v-if="!useWindow().state.isMacOS"
        alt="Luma Logo"
      />
      <p>Settings</p>
    </div>
  </TitleBar>

  <div class="main">
    <div class="sidebar">
      <div class="buttons">
        <button
          :class="{ selected: activeTab === 'extensions' }"
          @click="activeTab = 'extensions'"
        >
          Extensions
        </button>
        <button
          :class="{ selected: activeTab === 'general' }"
          @click="activeTab = 'general'"
        >
          General
        </button>
      </div>
      <div class="version lum-selectable">
        <p>{{ appVersion }}</p>
      </div>
    </div>

    <div class="content">
      <!-- Extensions Settings Tab -->
      <div v-if="activeTab === 'extensions'" class="settings-page">
        <DiscordRpcSettings />
      </div>

      <!-- General Settings Tab -->
      <div v-else-if="activeTab === 'general'" class="settings-page">
        <LumCard class="settings-card">
          <template #header>
            <p class="card-title">About LumaIDE</p>
          </template>
          <div class="card-body info-text">
            <p><strong>Version:</strong> {{ appVersion || "0.1.0-alpha" }}</p>
            <p>A modern, lightweight IDE tailored for Lua, Luau, and game scripting.</p>
          </div>
        </LumCard>
      </div>
    </div>

    <LumToasts />
    <LumModals />
  </div>
</template>

<style scoped lang="scss">
.title {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--lum-gray-200);

  img {
    height: auto;
    width: 20px;
  }
}

.main {
  height: 100%;
  min-height: 0;
  min-width: 0;
  width: 100%;
  display: flex;
  flex-direction: row;
  overflow: hidden;

  .sidebar {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--lum-border);
    overflow: hidden;
    background-color: var(--lum-gray-900);

    .buttons {
      flex: 1;
      min-height: 0;
      border-top: 1px solid var(--lum-border);
      border-bottom: 1px solid var(--lum-border);
      display: flex;
      flex-direction: column;

      button {
        height: 48px;
        width: 100%;
        background-color: transparent;
        border: none;
        color: var(--lum-gray-200);
        font-family: var(--lum-font-sans);
        font-size: 14px;
        font-weight: 500;
        text-align: left;
        padding-left: 20px;
        cursor: pointer;
        transition: background-color 0.15s ease-in-out, color 0.15s ease-in-out;

        &:hover {
          background-color: color-mix(in srgb, var(--lum-gray-700), white 5%);
          color: white;
        }

        &:active {
          background-color: color-mix(in srgb, var(--lum-gray-700), black 5%);
        }

        &.selected {
          border-left: 4px solid var(--lum-accent);
          background-color: color-mix(in srgb, var(--lum-gray-800), white 4%);
          color: white;
        }
      }
    }

    .version {
      padding: 12px;
      color: var(--lum-gray-400);
      font-size: 12px;
      text-align: center;
    }
  }

  .content {
    padding: 24px 32px;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
}

.settings-page {
  display: flex;
  flex-direction: column;
  gap: 20px;
  max-width: 640px;
}

.settings-card {
  width: 100%;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--lum-gray-100);
  margin: 0;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-top: 8px;
}

.info-text {
  font-size: 13px;
  line-height: 1.6;
  color: var(--lum-gray-300);

  p {
    margin: 0 0 8px 0;

    &:last-child {
      margin-bottom: 0;
    }
  }
}
</style>
