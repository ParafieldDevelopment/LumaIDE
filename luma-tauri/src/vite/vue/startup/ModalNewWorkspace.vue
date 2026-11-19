<script setup lang="ts">
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { LumButton, LumInput } from "~/vue/components";

interface WorkspaceCreated {
  project_uuid: string;
  title: string;
  path: string;
  git_branch?: string;
  icon_path?: string;
  last_opened_at?: number;
}

const props = defineProps<{
  onCreated?: (workspace: WorkspaceCreated) => void | Promise<void>;
  onClose?: () => void;
}>();

const title = ref("");
const parentPath = ref("");
const error = ref("");
const busy = ref(false);

const targetPath = computed(() => {
  const parent = parentPath.value.trim().replace(/[\\/]+$/, "");
  const name = title.value.trim();
  if (!parent || !name) return "";
  const separator = parent.includes("\\") ? "\\" : "/";
  return parent + separator + name;
});

async function chooseLocation() {
  error.value = "";
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Choose Workspace Parent Folder",
    });
    if (typeof selected === "string") parentPath.value = selected;
  } catch (cause) {
    error.value = `Unable to open folder picker: ${String(cause)}`;
  }
}

async function createWorkspace() {
  if (busy.value) return;
  const workspaceTitle = title.value.trim();
  const location = parentPath.value.trim();
  if (!workspaceTitle) {
    error.value = "Enter a workspace name.";
    return;
  }
  if (!location) {
    error.value = "Choose a parent folder.";
    return;
  }

  busy.value = true;
  error.value = "";
  try {
    const workspace = await invoke<WorkspaceCreated>("create_workspace", {
      title: workspaceTitle,
      parentPath: location,
    });
    await invoke("open_workspace", { path: workspace.path });
    await props.onCreated?.(workspace);
    props.onClose?.();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="new-workspace">
    <div class="field">
      <label for="workspace-name">Workspace name</label>
      <LumInput id="workspace-name" v-model="title" placeholder="My Workspace" :disabled="busy" />
    </div>

    <div class="field">
      <label>Parent folder</label>
      <div class="location-row">
        <LumInput v-model="parentPath" placeholder="Choose where to create it" readonly :disabled="busy" />
        <LumButton @click="chooseLocation" :disabled="busy">Browse</LumButton>
      </div>
    </div>

    <div v-if="targetPath" class="target">
      <span>Workspace folder</span>
      <p>{{ targetPath }}</p>
    </div>

    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <div class="actions">
      <LumButton @click="props.onClose?.()" variant="ghost" :disabled="busy">Cancel</LumButton>
      <LumButton @click="createWorkspace" variant="primary" :loading="busy" :disabled="busy">Create Workspace</LumButton>
    </div>
  </div>
</template>

<style scoped>
.new-workspace {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.field label,
.target span {
  font-size: 0.85rem;
  color: var(--lum-gray-300);
}

.location-row {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.location-row :deep(input) {
  min-width: 0;
  flex: 1;
}

.target {
  padding: 0.75rem;
  border: 1px solid var(--lum-border);
  border-radius: 8px;
  overflow-wrap: anywhere;
}

.target p {
  margin: 0.4rem 0 0;
  font-size: 0.85rem;
}

.error {
  margin: 0;
  color: var(--lum-danger, #e66);
  overflow-wrap: anywhere;
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
}
</style>
