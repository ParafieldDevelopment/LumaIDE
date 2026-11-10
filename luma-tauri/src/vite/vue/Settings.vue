<script setup lang="ts">
import {useWindow} from "~/vue/composables/useWindow.ts";
import TitleBar from "~/vue/window/TitleBar.vue";
import {onMounted} from "vue";
import {loadTheme, webviewReady} from "~/luma/LumaWindow.ts";
import {LumContextMenu, LumModals, LumToasts} from "~/vue/components";
import {invoke} from "@tauri-apps/api/core";

onMounted(async () => {
  await loadTheme();
  await webviewReady();
})

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
      >
      <p>Settings</p>
    </div>
  </TitleBar>

  <div class="main">
    <LumToasts />
    <LumModals />
    <LumContextMenu />
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

  .version {
    padding: 10px;
    color: var(--lum-gray-200);
    font-size: 12px;
    text-align: center;
  }

  .sidebar {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--lum-border);
    overflow: hidden;

    > img {
      padding: 30px;
      width: 100%;
      height: auto;
      flex-shrink: 0;
    }

    .buttons {
      flex: 1;
      min-height: 0;
      border-top: 1px solid var(--lum-border);
      border-bottom: 1px solid var(--lum-border);

      button {
        height: 50px;
        width: 100%;
        background-color: transparent;
        border: none;
        color: white;
        font-family: var(--lum-font-sans);
        transition: background-color 0.1s ease-in-out;

        &:hover {
          background-color: color-mix(
              in srgb,
              var(--lum-gray-700),
              white 5%
          );
        }

        &:active {
          background-color: color-mix(
              in srgb,
              var(--lum-gray-700),
              black 5%
          );
        }

        &.selected {
          border-left: 5px solid var(--lum-accent);
        }
      }
    }

    footer {
      display: flex;
      height: 50px;
      flex-shrink: 0;
      width: 100%;
      align-items: center;
      padding: 10px;
    }
  }

  .content {
    padding: 10px;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;

    .page {
      height: 100%;
      min-height: 0;
      overflow-y: auto;
      overflow-x: hidden;
    }
  }
}

/* Forward navigation: slide upward */
.slide-up-enter-active,
.slide-up-leave-active {
  transition:
      transform 0.2s ease,
      opacity 0.2s ease;
}

.slide-up-enter-from {
  opacity: 0;
  transform: translateY(25px);
}

.slide-up-leave-to {
  opacity: 0;
  transform: translateY(-25px);
}

/* Backward navigation: slide downward */
.slide-down-enter-active,
.slide-down-leave-active {
  transition:
      transform 0.2s ease,
      opacity 0.2s ease;
}

.slide-down-enter-from {
  opacity: 0;
  transform: translateY(-25px);
}

.slide-down-leave-to {
  opacity: 0;
  transform: translateY(25px);
}

@media (prefers-reduced-motion: reduce) {
  .slide-up-enter-active,
  .slide-up-leave-active,
  .slide-down-enter-active,
  .slide-down-leave-active {
    transition-duration: 0.01ms;
  }
}
</style>
