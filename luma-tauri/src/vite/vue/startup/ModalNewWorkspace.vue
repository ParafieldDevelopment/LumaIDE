<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { LumButton, LumField, LumInput, LumList, LumListItem, LumSelect } from "~/vue/components";
import { getProjectColor } from "~/luma/Workspace.ts";
import { getProjectType, groupedProjectTypes } from "~/luma/ProjectTypes.ts";

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

const groups = groupedProjectTypes();
const selectedTypeId = ref(groups[0].types[0].id);
const selectedType = computed(() => getProjectType(selectedTypeId.value));

const runtimeId = ref("");
const templateId = ref("");

// Reset runtime/template to the first option whenever the project type changes.
watch(
  selectedType,
  type => {
    runtimeId.value = type.runtimes[0]?.id ?? "";
    templateId.value = type.templates[0]?.id ?? "";
  },
  { immediate: true },
);

const runtimeOptions = computed(() =>
  selectedType.value.runtimes.map(runtime => ({ value: runtime.id, label: runtime.label })),
);

const title = ref("");
const parentPath = ref("");
const error = ref("");
const busy = ref(false);

const INVALID_NAME_CHARS = /[<>:"/\\|?*]/;

const nameError = computed(() => {
  const name = title.value.trim();
  if (!name) return "";
  if (INVALID_NAME_CHARS.test(name)) return 'Name cannot contain < > : " / \\ | ? *';
  if (/[. ]$/.test(name)) return "Name cannot end with a dot or space.";
  return "";
});

const targetPath = computed(() => {
  const parent = parentPath.value.trim().replace(/[\\/]+$/, "");
  const name = title.value.trim();
  if (!parent || !name || nameError.value) return "";
  const separator = parent.includes("\\") ? "\\" : "/";
  return parent + separator + name;
});

function typeColor(label: string) {
  return getProjectColor(label);
}

function typeInitials(label: string) {
  return label.slice(0, 2).toUpperCase();
}

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
  if (nameError.value) {
    error.value = nameError.value;
    return;
  }
  if (!location) {
    error.value = "Choose a parent folder.";
    return;
  }

  busy.value = true;
  error.value = "";
  try {
    // Backend currently only takes a title and parent path. Project type,
    // runtime and template are not sent until the Rust model supports them.
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
  <div class="wizard">
    <div class="body">
      <nav class="types" aria-label="Project types">
        <div v-for="group in groups" :key="group.label" class="group">
          <p class="group-label">{{ group.label }}</p>
          <LumList>
            <LumListItem
                v-for="type in group.types"
                :key="type.id"
                :selected="type.id === selectedTypeId"
                :disabled="busy"
                @select="selectedTypeId = type.id"
            >
              <span class="badge" :style="{ backgroundColor: typeColor(type.label) }">
                {{ typeInitials(type.label) }}
              </span>
              {{ type.label }}
            </LumListItem>
          </LumList>
        </div>
      </nav>

      <section class="details">
        <header class="type-header">
          <h3>{{ selectedType.label }}</h3>
          <p>{{ selectedType.description }}</p>
        </header>

        <LumField label="Name" :error="nameError || undefined">
          <LumInput
              v-model="title"
              placeholder="My Workspace"
              :invalid="!!nameError"
              :disabled="busy"
          />
        </LumField>

        <LumField label="Location">
          <div class="location-row">
            <LumInput v-model="parentPath" placeholder="Choose where to create it" readonly :disabled="busy" />
            <LumButton @click="chooseLocation" :disabled="busy">Browse</LumButton>
          </div>
          <template #help>
            <span v-if="targetPath">Workspace will be created in {{ targetPath }}</span>
          </template>
        </LumField>

        <LumField v-if="runtimeOptions.length > 0" label="Runtime">
          <LumSelect v-model="runtimeId" :options="runtimeOptions" :disabled="busy" />
        </LumField>

        <LumField v-if="selectedType.templates.length > 0" label="Template">
          <LumList>
            <LumListItem
                v-for="template in selectedType.templates"
                :key="template.id"
                :selected="template.id === templateId"
                :disabled="busy"
                @select="templateId = template.id"
            >
              {{ template.label }}
              <template #meta>{{ template.description }}</template>
            </LumListItem>
          </LumList>
        </LumField>

        <p class="note">Runtime and template are saved with the workspace in a later update.</p>

        <p v-if="error" class="error" role="alert">{{ error }}</p>
      </section>
    </div>

    <div class="actions">
      <LumButton @click="props.onClose?.()" variant="ghost" :disabled="busy">Cancel</LumButton>
      <LumButton @click="createWorkspace" variant="primary" :loading="busy" :disabled="busy">Create</LumButton>
    </div>
  </div>
</template>

<style scoped>
.wizard {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
}

.body {
  display: flex;
  min-height: 420px;
  max-height: 60vh;
  border: 1px solid var(--lum-border);
  border-radius: 8px;
  overflow: hidden;
}

.types {
  width: 220px;
  flex-shrink: 0;
  padding: 0.75rem;
  overflow-y: auto;
  border-right: 1px solid var(--lum-border);
}

.group + .group {
  margin-top: 1rem;
}

.group-label {
  margin: 0 0 0.4rem;
  padding: 0 0.5rem;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--lum-gray-300);
}

.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-right: 0.5rem;
  border-radius: 6px;
  font-size: 0.65rem;
  font-weight: 600;
  color: white;
  vertical-align: middle;
}

.details {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  overflow-y: auto;
}

.type-header h3 {
  margin: 0;
}

.type-header p {
  margin: 0.25rem 0 0;
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

.note {
  margin: 0;
  font-size: 0.8rem;
  color: var(--lum-gray-300);
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
}
</style>
