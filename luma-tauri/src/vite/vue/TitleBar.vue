<script setup lang="ts">
import { useWindow } from "~/vue/composables/useWindow.ts";
import type {HTMLAttributes} from "vue";

const props = defineProps<{
  contentClass?: HTMLAttributes["class"];
}>();

const { state, minimize, toggleMaximize, close } = useWindow();
</script>

<template>
  <header
      data-tauri-drag-region="deep"
      class="titlebar"
      :class="{
      macos: state.isMacOS,
      unfocused: !state.isFocused
    }"
  >
    <div class="titlebar-content" :class="props.contentClass">
      <slot />
    </div>

    <div class="titlebar-buttons" v-if="!state.isMacOS">
      <!-- Minimize -->
      <button @click="minimize()">
        <svg xmlns="http://www.w3.org/2000/svg"
             width="20" height="20" viewBox="0 0 24 24"
             fill="none" stroke="white" stroke-width="1.5"
             stroke-linecap="round" stroke-linejoin="round">
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>

      <!-- Maximize / Restore -->
      <button @click="toggleMaximize()">
        <svg v-if="!state.isMaximized"
             xmlns="http://www.w3.org/2000/svg"
             width="20" height="20" viewBox="0 0 24 24"
             fill="none" stroke="white" stroke-width="1.5">
          <rect x="5" y="5" width="14" height="14" />
        </svg>

        <svg v-else
             xmlns="http://www.w3.org/2000/svg"
             width="20" height="20" viewBox="0 0 24 24"
             fill="none" stroke="white" stroke-width="1.5">
          <path d="M8 8V5H19V16H16" />
          <rect x="5" y="8" width="11" height="11" />
        </svg>
      </button>

      <!-- Close -->
      <button class="close" @click="close()">
        <svg xmlns="http://www.w3.org/2000/svg"
             width="20" height="20" viewBox="0 0 24 24"
             fill="none" stroke="white" stroke-width="1.5"
             stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  </header>
</template>

<style scoped lang="scss">
header.titlebar {
  -webkit-user-select: none;
  user-select: none;

  display: flex;
  align-items: center;

  background-color: var(--lum-bg-sunken);
  width: 100%;
  height: 40px;

  .titlebar-content {
    display: flex;
    align-items: center;
    height: 100%;
    flex: 1;
    padding: 0 10px;
    min-width: 0;
  }

  .titlebar-buttons {
    display: flex;
    height: 100%;
    margin-left: auto;

    button {
      height: 100%;
      width: 50px;
      border: none;

      display: flex;
      align-items: center;
      justify-content: center;

      background-color: transparent;

      &:hover {
        background-color: rgb(255 255 255 / 20%);
      }

      &.close:hover {
        background-color: rgb(232 17 35);
      }
    }
  }

  &.macos { // traffic light buttons
    padding-left: 70px;
  }

  &.unfocused {
    .titlebar-buttons button {
      opacity: 0.5;
    }

    .titlebar-content * {
      opacity: 0.5;
    }

    background-color: color-mix(in srgb, var(--lum-bg-sunken), black 10%);
  }
}
</style>