<script setup lang="ts">
import {LumButton, LumInput} from "~/vue/components";
import SearchIcon from "~/vue/icons/SearchIcon.vue";
import WorkspaceUI from "~/vue/startup/WorkspaceUI.vue";
import {Workspace} from "~/luma/Workspace.ts";

const workspaces: Workspace[] = []
</script>

<template>
  <div class="container">
    <div class="header">
      <LumInput placeholder="Search Workspaces">
        <template #icon>
          <SearchIcon />
        </template>
      </LumInput>
      <div class="buttons">
        <LumButton>New Workspace</LumButton>
        <LumButton>Open</LumButton>
        <LumButton>Clone Repository</LumButton>
      </div>
    </div>
    <div class="projects" v-for="workspace in workspaces" v-if="workspaces.length > 0">
      <WorkspaceUI :project="workspace" />
    </div>
    <div class="projects">
      <p>No Workspaces :(</p>
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
    display: flex;
    flex-direction: column;

    padding: 1rem;
    overflow-y: auto;

    gap: 0.5rem;
  }
</style>