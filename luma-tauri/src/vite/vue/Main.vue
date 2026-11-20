<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, computed } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { LumContextMenu, LumModals, LumFileTabs, LumEditor, LumProjectExplorer } from "./components";
import TitleBar from "~/vue/window/TitleBar.vue";
import { useEditor } from "~/vue/composables/useEditor.ts";
import { useToasts } from "~/vue/components/toasts";

interface ActiveWorkspace {
  project_uuid: string;
  title: string;
  path: string;
  git_branch?: string;
  icon_path?: string;
}

const {
  openFiles,
  activeFileId,
  activeFile,
  loadError: editorLoadError,
  closeFile,
  saveFile,
  saveAll,
  setContents,
  setWorkspacePath,
} = useEditor();

const { toast } = useToasts();

const activeWorkspace = ref<ActiveWorkspace | null>(null);
const unlistenWorkspace = ref<UnlistenFn | null>(null);
const sidebarWidth = ref(260);
const isResizing = ref(false);

const tabFiles = computed(() =>
  openFiles.value.map((file) => ({
    id: file.id,
    name: file.name,
    dirty: file.dirty,
  }))
);

const saveTooltip = computed(() =>
  activeFile.value ? `Save ${activeFile.value.name}` : "Save file"
);

const editorValue = computed({
  get: () => activeFile.value?.contents ?? "",
  set: (value: string) => {
    if (activeFile.value) setContents(activeFile.value.id, value);
  },
});

async function loadActiveWorkspace() {
  try {
    const workspace = await invoke<ActiveWorkspace | null>("get_active_workspace");
    activeWorkspace.value = workspace ?? null;
    if (workspace) {
      setWorkspacePath(workspace.path);
    }
  } catch (error) {
    editorLoadError.value = `Unable to retrieve the active workspace: ${String(error)}`;
  }
}

async function handleCloseTab(id: number) {
  const file = openFiles.value.find((f) => f.id === id);
  if (file && file.dirty) {
    if (
      !window.confirm(`"${file.name}" has unsaved changes. Close without saving?`)
    ) {
      return;
    }
  }
  closeFile(id);
}

async function handleSave() {
  if (!activeFile.value) return;
  const ok = await saveFile(activeFile.value.id);
  if (ok) toast("success", "Saved", activeFile.value.name);
}

async function handleSaveAll() {
  const ok = await saveAll();
  if (ok) toast("success", "Saved", "All files");
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "F12") {
    event.preventDefault();
    return;
  }
  if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "i") {
    event.preventDefault();
    void invoke("open_devtools");
    return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void handleSave();
  }
}

onMounted(async () => {
  unlistenWorkspace.value = await listen<ActiveWorkspace>("workspace-opened", (event) => {
    activeWorkspace.value = event.payload;
    if (event.payload) {
      setWorkspacePath(event.payload.path);
    }
  });
  await loadActiveWorkspace();
  window.addEventListener("keydown", handleKeydown);
});

onBeforeUnmount(() => {
  unlistenWorkspace.value?.();
  window.removeEventListener("keydown", handleKeydown);
});

function startResize(event: MouseEvent) {
  isResizing.value = true;
  window.addEventListener("mousemove", onResize);
  window.addEventListener("mouseup", stopResize);
  event.preventDefault();
}

function onResize(event: MouseEvent) {
  if (!isResizing.value) return;
  sidebarWidth.value = Math.min(Math.max(Math.round(event.clientX), 200), 600);
}

function stopResize() {
  isResizing.value = false;
  window.removeEventListener("mousemove", onResize);
  window.removeEventListener("mouseup", stopResize);
}
</script>

<template>
  <TitleBar>
    <template #default>
      <div class="ide-title" v-if="activeWorkspace" :title="activeWorkspace.path">
        {{ activeWorkspace.title }}
      </div>
    </template>
  </TitleBar>

  <div class="ide-shell" v-if="activeWorkspace">
    <div class="ide-body">
      <div class="lum-split-pane ide-sidebar">
        <LumProjectExplorer :workspace-path="activeWorkspace.path" />
      </div>
      <div
        class="lum-resizer ide-resizer"
        @mousedown="startResize"
        :class="{ 'is-dragging': isResizing }"
      ></div>
      <div class="ide-editor-area">
        <div class="ide-tabs">
          <LumFileTabs
            v-model="activeFileId"
            :files="tabFiles"
            @close="handleCloseTab"
          >
            <template #extra>
              <div class="ide-tab-actions">
                <button class="ide-tab-action" @click="handleSaveAll" title="Save All">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
                    <polyline points="17 21 17 13 7 13 7 21" />
                    <polyline points="7 3 7 8 15 8" />
                  </svg>
                </button>
                <button
                  class="ide-tab-action"
                  @click="handleSave"
                  :title="saveTooltip"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
                    <polyline points="17 21 17 13 7 13 7 21" />
                    <polyline points="7 3 7 8 15 8" />
                  </svg>
                </button>
              </div>
            </template>
          </LumFileTabs>
        </div>

        <div class="ide-editor-surface">
          <LumEditor
            v-if="activeFile"
            v-model="editorValue"
            :language="activeFile.language"
          />
          <div v-else class="ide-empty">
            <p>Open a file from the project explorer to start editing.</p>
          </div>
        </div>
      </div>
    </div>
  </div>

  <div v-else-if="editorLoadError" class="ide-load-error">
    {{ editorLoadError }}
  </div>
  <div v-else class="ide-loading">Loading workspace…</div>

  <LumContextMenu />
  <LumModals />
</template>

<style scoped>
.ide-title {
  font-size: var(--lum-text-md);
  font-weight: var(--lum-font-weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 220px;
}

.ide-shell {
  display: flex;
  flex-direction: column;
  height: calc(100% - 40px);
  min-height: 0;
}

.ide-body {
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
}

.ide-sidebar {
  flex: 0 0 auto;
  width: var(--lum-sidebar-width, 260px);
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.ide-resizer {
  flex-shrink: 0;
  width: 3px;
  cursor: col-resize;
  background: transparent;
  z-index: var(--lum-z-sticky);
  touch-action: none;
}

.ide-resizer:hover {
  background: var(--lum-accent);
}

.ide-resizer.is-dragging {
  background: var(--lum-accent);
  box-shadow: 0 0 0 1px var(--lum-accent-muted);
}

.ide-editor-area {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.ide-tabs {
  flex-shrink: 0;
}

.ide-tab-actions {
  display: flex;
  gap: 4px;
}

.ide-tab-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  color: var(--lum-fg-muted);
  cursor: pointer;
  border-radius: var(--lum-radius-sm);
}

.ide-tab-action:hover {
  background: var(--lum-bg-hover);
  color: var(--lum-fg-strong);
}

.ide-editor-surface {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--lum-bg-app);
  position: relative;
  overflow: hidden;
}

.ide-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--lum-fg-muted);
  padding: var(--lum-space-8);
}

.ide-empty p {
  margin: 0;
}

.ide-load-error,
.ide-loading {
  padding: 1.5rem;
  text-align: center;
  color: var(--lum-fg-muted);
}

.ide-load-error {
  color: var(--lum-danger);
}
</style>
