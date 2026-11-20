<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { LumButton, LumField, LumInput, LumList, LumListItem } from "~/vue/components";
import { getProjectType, getTemplateFiles, groupedProjectTypes } from "~/luma/ProjectTypes.ts";

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

const groups = computed(() => groupedProjectTypes());
const selectedTypeId = ref(groups.value[0].types[0].id);
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

const templateFiles = computed(() => {
  if (!templateId.value) return [];
  return getTemplateFiles(selectedTypeId.value, templateId.value, {
    title: title.value.trim() || "Workspace",
    runtime: runtimeId.value || undefined,
  });
});

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

const FALLBACK_TYPE_ICON = "/project_type_icons/package.svg";

function typeIcon(icon?: string) {
  return icon ?? FALLBACK_TYPE_ICON;
}

function runtimeIcon(icon?: string) {
  return icon ?? FALLBACK_TYPE_ICON;
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
    const files = getTemplateFiles(selectedTypeId.value, templateId.value, {
      title: workspaceTitle,
      runtime: runtimeId.value || undefined,
    });
    const workspace = await invoke<WorkspaceCreated>("create_workspace", {
      title: workspaceTitle,
      parentPath: location,
      projectType: selectedTypeId.value,
      runtime: runtimeId.value || null,
      template: templateId.value || null,
      files: files.length > 0 ? files : null,
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
              <span class="badge">
                <img :src="typeIcon(type.icon)" :alt="`${type.label} icon`" />
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

        <LumField v-if="selectedType.runtimes.length > 0" label="Runtime">
          <LumList>
            <LumListItem
                v-for="runtime in selectedType.runtimes"
                :key="runtime.id"
                :selected="runtime.id === runtimeId"
                :disabled="busy"
                @select="runtimeId = runtime.id"
            >
              <span class="badge">
                <img :src="runtimeIcon(runtime.icon)" :alt="`${runtime.label} icon`" />
              </span>
              {{ runtime.label }}
            </LumListItem>
          </LumList>
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

        <p v-if="templateFiles.length > 0" class="note">
          Generates starter files:
          <span class="file-badges">
            <code v-for="file in templateFiles" :key="file.path">{{ file.path }}</code>
          </span>
        </p>
        <p v-else class="note">Empty workspace with no initial files.</p>

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

/* Fixed height so both panes scroll independently instead of the modal body. */
.body {
  display: flex;
  height: min(520px, 62vh);
  min-height: 0;
  border: 1px solid var(--lum-border);
  border-radius: 8px;
  overflow: hidden;
}

.types {
  width: 220px;
  flex-shrink: 0;
  min-height: 0;
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
  width: 30px;
  height: 30px;
  padding: 1px;
  box-sizing: border-box;
  margin-right: 0.5rem;
  vertical-align: middle;
}

.badge img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

/* These feather-style icons use currentColor, which is black inside an <img>. */
.badge img[src*="box.svg"],
.badge img[src*="package.svg"] {
  filter: invert(1);
}

.details {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
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

.file-badges {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-left: 0.35rem;
  vertical-align: middle;

  code {
    font-size: 0.75rem;
    padding: 0.1rem 0.35rem;
    background: var(--lum-bg-hover, rgba(255, 255, 255, 0.06));
    border: 1px solid var(--lum-border);
    border-radius: 4px;
    color: var(--lum-text);
  }
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
