<script setup lang="ts">
import type {Workspace} from "~/luma/Workspace.ts";
import {getProjectInitials} from "~/luma/Workspace.ts";
import MoreVerticalIcon from "~/vue/icons/MoreVerticalIcon.vue";
import { convertFileSrc } from "@tauri-apps/api/core";

const props = defineProps<{
  project: Workspace
}>()

function click() {

}

import { computed } from "vue";

const iconSrc = computed(() => {
  if (!props.project.icon_path) return null;

  return convertFileSrc(props.project.icon_path);
});
</script>

<template>
  <div ref="projectDiv" class="project" @click="click">
    <div
        class="project_icon"
        :style="{
        backgroundColor: project.icon_path ? 'transparent' : project.color
      }"
    >
      <img
          v-if="iconSrc"
          :src="iconSrc"
          alt="Project icon"
      />

      <p v-else>
        {{ getProjectInitials(props.project.title) }}
      </p>
    </div>

    <div class="project_content">
      <h3>{{ props.project.title }}</h3>
      <p>{{ props.project.path }}</p>
      <p>{{ props.project.git_branch }}</p>
    </div>

    <div class="more" @click.stop="">
      <MoreVerticalIcon />
    </div>
  </div>
</template>

<style scoped lang="scss">
  div.project {
    background-color: transparent;
    border-radius: 15px;
    border: 1px solid var(--lum-border);

    padding: 10px;

    width: 100%;

    display: flex;

    &:hover {
      background-color: rgba(255,255,255,0.1);
    }

    &:active {
      background-color: transparent;
    }

    & .project_icon {
      display: flex;

      height: 35px;
      width: 35px;
      border-radius: 10px;

      align-items: center;
      justify-content: center;

      font-size: var(--lum-text-lg);

      img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        border-radius: inherit;
      }
    }

    & .project_content {
      margin-left: 10px;

      display: flex;
      flex-direction: column;
      justify-content: flex-start;
      align-items: flex-start;
    }

    &:hover .more {
      display: flex;
    }

    & .more {
      display: none;

      margin-left: auto;
      align-items: center;
      justify-content: center;

      width: 35px;
    }
  }
</style>