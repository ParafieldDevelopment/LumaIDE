<script setup lang="ts">
import TitleBar from "./window/TitleBar.vue";
import { useWindow } from "~/vue/composables/useWindow.ts";
import Projects from "~/vue/startup/Projects.vue";
import {LumButton, LumModals, LumToasts} from "~/vue/components";
import {getAppVersion, loadTheme, openSettings, webviewReady} from "~/luma/LumaWindow.ts";
import { onMounted, ref, computed } from "vue";
import AboutLuma from "~/vue/startup/AboutLuma.vue";
import SettingsIcon from "~/vue/icons/SettingsIcon.vue";
import { LumContextMenu } from "~/vue/components";

const appVersion = ref("");
const selectedButton = ref(1);
const previousButton = ref(1);

const transitionName = computed(() =>
    selectedButton.value > previousButton.value
        ? "slide-up"
        : "slide-down"
);

function selectMenu(button: number) {
  if (selectedButton.value === button) return;

  previousButton.value = selectedButton.value;
  selectedButton.value = button;
}

onMounted(async () => {
  await loadTheme();

  appVersion.value = await getAppVersion();

  await webviewReady();
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
      <p>Welcome to Luma</p>
    </div>
  </TitleBar>

  <div class="main">
    <div class="sidebar">
      <img src="/public/banner.png" alt="Luma Banner">

      <div class="version lum-selectable">
        <p>{{ appVersion }}</p>
      </div>

      <div class="buttons">
        <button
            :class="{ selected: selectedButton === 1 }"
            @click="selectMenu(1)"
        >
          Projects
        </button>

        <button
            :class="{ selected: selectedButton === 2 }"
            @click="selectMenu(2)"
        >
          Extensions
        </button>

        <button
            :class="{ selected: selectedButton === 3 }"
            @click="selectMenu(3)"
        >
          About Luma
        </button>
      </div>

      <footer>
        <LumButton icon @click="openSettings">
          <SettingsIcon />
        </LumButton>
      </footer>
    </div>

    <div class="content">
      <Transition :name="transitionName" mode="out-in">
        <div :key="selectedButton" class="page">
          <Projects v-if="selectedButton === 1" />

          <div v-else-if="selectedButton === 2">
            <h1>Coming Soon!</h1>
          </div>

          <AboutLuma v-else-if="selectedButton === 3" />
        </div>
      </Transition>
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
