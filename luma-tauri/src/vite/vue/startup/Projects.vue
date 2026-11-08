<script setup lang="ts">
import {LumButton, LumInput} from "~/vue/components";
import SearchIcon from "~/vue/icons/SearchIcon.vue";
import Project from "~/vue/startup/Project.vue";
</script>

<template>
  <div class="container">
    <div class="header">
      <LumInput placeholder="Search Projects">
        <template #icon>
          <SearchIcon />
        </template>
      </LumInput>
      <div class="buttons">
        <LumButton>New Project</LumButton>
        <LumButton>Open</LumButton>
        <LumButton>Clone Repository</LumButton>
      </div>
    </div>
    <div class="projects">
      <Project />
    </div>
  </div>
</template>

<style scoped>
  .container {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: 1rem;

    border-bottom: 1px solid var(--lum-border);
    padding: 1rem;

    div.buttons {
      display: flex;
      gap: 0.8rem;
    }
  }

  .projects {
    padding: 1rem;
    overflow-y: auto;
  }
</style>