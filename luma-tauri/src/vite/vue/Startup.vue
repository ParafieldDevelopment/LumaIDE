<script setup lang="ts">
import TitleBar from "./window/TitleBar.vue";
import {useWindow} from "~/vue/composables/useWindow.ts";
import Projects from "~/vue/startup/Projects.vue";
import {LumButton} from "~/vue/components";
</script>

<template>
  <TitleBar contentClass="startup-title">
    <div class="startup-title">
      <img src="/public/logo.svg" v-if="!useWindow().state.isMacOS" alt="Luma Logo">
      <p>Welcome to Luma</p>
    </div>
  </TitleBar>
  <div class="main">
    <div class="sidebar">
      <img src="/public/banner.png" alt="Luma Banner">
      <div class="buttons">
        <button>
          Projects
        </button>
        <button>
          Extensions
        </button>
        <button>
          About Luma
        </button>
      </div>
      <footer>
        <LumButton icon>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-settings">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z">
            </path>
          </svg>
        </LumButton>
      </footer>
    </div>
    <div class="content">
      <Projects/>
    </div>
  </div>
</template>

<style scoped lang="scss">
  .startup-title {
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
      overflow: auto;
    }
  }

</style>