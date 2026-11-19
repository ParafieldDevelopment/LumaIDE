<script setup lang="ts">
import { LumButton, LumInput, useModals } from "~/vue/components";
import SearchIcon from "~/vue/icons/SearchIcon.vue";
import WorkspaceBox from "~/vue/startup/WorkspaceBox.vue";
import { Workspace } from "~/luma/Workspace.ts";
import ModalNewWorkspace from "~/vue/startup/ModalNewWorkspace.vue";
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

interface RecentWorkspace {
  project_uuid: string;
  title: string;
  path: string;
  icon_path?: string;
  git_branch?: string;
  last_opened_at?: number;
}

const workspaces = ref<Workspace[]>([]);
const loadError = ref("");
const actionError = ref("");
const searchQuery = ref("");
const opening = ref(false);

const filteredWorkspaces = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query) return workspaces.value;
  return workspaces.value.filter(workspace =>
    workspace.title.toLowerCase().includes(query) || workspace.path.toLowerCase().includes(query)
  );
});

async function refreshWorkspaces() {
  loadError.value = "";
  try {
    const recent = await invoke<RecentWorkspace[]>("get_recent_workspaces");
    workspaces.value = recent.map(project =>
      new Workspace(project.project_uuid, project.title, project.path, project.icon_path, project.git_branch)
    );
  } catch (error) {
    console.error("Failed to load recent workspaces:", error);
    loadError.value = `Unable to load recent workspaces: ${String(error)}`;
  }
}

onMounted(refreshWorkspaces);

function newWorkspace() {
  const modals = useModals();
  const item = modals.modal("New Workspace", ModalNewWorkspace, { size: "md" });
  item.componentProps = {
    onCreated: refreshWorkspaces,
    onClose: () => modals.dismiss(item.id),
  };
}

async function openWorkspace() {
  if (opening.value) return;
  actionError.value = "";
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Open Workspace Folder",
    });
    if (typeof selected !== "string") return;
    opening.value = true;
    await invoke("open_workspace", { path: selected });
  } catch (error) {
    actionError.value = `Unable to open workspace: ${String(error)}`;
  } finally {
    opening.value = false;
  }
}

async function openRecentWorkspace(workspace: Workspace) {
  if (opening.value) return;
  actionError.value = "";
  opening.value = true;
  try {
    await invoke("open_workspace", { path: workspace.path });
  } catch (error) {
    actionError.value = `Unable to open workspace: ${String(error)}`;
  } finally {
    opening.value = false;
  }
}
</script>

<template>
  <div class="container">
    <div class="header">
      <LumInput v-model="searchQuery" placeholder="Search Workspaces">
        <template #icon>
          <SearchIcon />
        </template>
      </LumInput>
      <div class="buttons">
        <LumButton @click="newWorkspace">New Workspace</LumButton>
        <LumButton @click="openWorkspace" :disabled="opening" :loading="opening">Open</LumButton>
        <LumButton disabled title="Repository cloning will be added in a later pass">Clone Repository</LumButton>
      </div>
    </div>

    <div class="projects" v-if="filteredWorkspaces.length > 0">
      <WorkspaceBox
          v-for="workspace in filteredWorkspaces"
          :key="workspace.project_uuid"
          :project="workspace"
          @open="openRecentWorkspace"
      />
    </div>
    <div class="projects" v-else-if="loadError">
      <p>{{ loadError }}</p>
    </div>
    <div class="projects empty" v-else>
      <p>{{ searchQuery ? "No matching workspaces." : "No Workspaces yet." }}</p>
    </div>

    <p v-if="actionError" class="action-error" role="alert">{{ actionError }}</p>
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

  .buttons {
    display: flex;
    gap: 0.8rem;
    flex-wrap: wrap;
  }
}

.projects {
  display: flex;
  flex-direction: column;
  padding: 1rem;
  overflow-y: auto;
  gap: 0.5rem;
}

.empty {
  opacity: 0.75;
}

.action-error {
  margin: 0 1rem 1rem;
  color: var(--lum-danger, #e66);
  overflow-wrap: anywhere;
}
</style>
