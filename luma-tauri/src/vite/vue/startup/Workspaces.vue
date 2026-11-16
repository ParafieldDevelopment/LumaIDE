<script setup lang="ts">
import {LumButton, LumInput, useModals} from "~/vue/components";
import SearchIcon from "~/vue/icons/SearchIcon.vue";
import WorkspaceBox from "~/vue/startup/WorkspaceBox.vue";
import {Workspace} from "~/luma/Workspace.ts";
import ModalNewWorkspace from "~/vue/startup/ModalNewWorkspace.vue";
import {onMounted, ref} from "vue";
import {invoke} from "@tauri-apps/api/core";

const workspaces = ref<Workspace[]>([]);
const loadError = ref("");

interface RecentWorkspace {
  project_uuid: string;
  title: string;
  path: string;
  icon_path?: string;
  git_branch?: string;
}

onMounted(async () => {
  try {
    const recentWorkspaces = await invoke<RecentWorkspace[]>("get_recent_workspaces");
    workspaces.value = recentWorkspaces.map(workspace =>
        new Workspace(
            workspace.project_uuid,
            workspace.title,
            workspace.path,
            workspace.icon_path,
            workspace.git_branch
        )
    );
  } catch (error) {
    console.error("Failed to load recent workspaces:", error);
    loadError.value = `Unable to load recent workspaces: ${String(error)}`;
  }
});

async function newWorkspace() {
  useModals().modal("New Workspace", ModalNewWorkspace, {});
}

async function openWorkspace() {

}

async function cloneRepository() {

}
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
        <LumButton @click="newWorkspace">New Workspace</LumButton>
        <LumButton @click="openWorkspace">Open</LumButton>
        <LumButton @click="cloneRepository">Clone Repository</LumButton>
      </div>
    </div>
    <div class="projects" v-if="workspaces.length > 0">
      <WorkspaceBox v-for="workspace in workspaces" :key="workspace.project_uuid" :project="workspace" />
    </div>
    <div class="projects" v-else-if="loadError">
      <p>{{ loadError }}</p>
    </div>
    <div class="projects" v-else>
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