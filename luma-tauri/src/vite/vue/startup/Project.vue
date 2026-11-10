<script setup lang="ts">
  import type {ProjectObject} from "~/luma/ProjectObject.ts";
  import {getProjectInitials} from "~/luma/ProjectObject.ts";
  import MoreVerticalIcon from "~/vue/icons/MoreVerticalIcon.vue";

  const props = defineProps<{
    project: ProjectObject
  }>()

  function click() {

  }

  function moreClick() {
    
  }
</script>

<template>
  <div ref="projectDiv" class="project" @click="click">
    <div
        class="project_icon"
        :style="{
        backgroundColor: project.icon ? 'transparent' : project.color
      }"
    >
      <component
          v-if="props.project.icon"
          :is="props.project.icon"
      />

      <p v-else>
        {{ getProjectInitials(props.project.title) }}
      </p>
    </div>

    <div class="project_content">
      <h3>{{ props.project.title }}</h3>
      <p>{{ props.project.path }}</p>
      <p>{{ props.project.project_uuid }}</p>
    </div>

    <div class="more" @click.stop="moreClick">
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