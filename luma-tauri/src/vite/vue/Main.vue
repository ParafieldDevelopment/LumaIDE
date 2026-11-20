<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, computed, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { LumContextMenu, LumModals, LumFileTabs, LumEditor } from "./components";
import TitleBar from "~/vue/window/TitleBar.vue";
import IdeTitlebarNav from "~/vue/window/IdeTitlebarNav.vue";
import { useEditor } from "~/vue/composables/useEditor.ts";
import { useToasts } from "~/vue/components/toasts";
import LumToolStripe from "./components/LumToolStripe.vue";
import { useToolWindows, registerToolWindow } from "~/vue/composables/useToolWindows";
import { FolderIcon, CommitIcon, TerminalIcon, AnalyzerIcon } from "~/vue/icons/ToolIcons";
import ProjectExplorerPanel from "./components/ProjectExplorer.vue";
import TerminalPanel from "./toolwindows/TerminalPanel.vue";
import AnalyzerPanel from "./toolwindows/AnalyzerPanel.vue";
import GitCommitPanel from "./toolwindows/GitCommitPanel.vue";

interface ActiveWorkspace {
  project_uuid: string;
  title: string;
  path: string;
  git_branch?: string;
  icon_path?: string;
}

const {
  openFiles, activeFileId, activeFile, loadError: editorLoadError,
  closeFile, saveFile, saveAll, setContents, setWorkspacePath,
} = useEditor();
const { toast } = useToasts();

const tools = useToolWindows();
registerToolWindow({ id: "project-explorer", title: "Project", anchor: "left", icon: FolderIcon, component: ProjectExplorerPanel, order: 10, hotkey: "Alt+1", defaultOpen: true });
registerToolWindow({ id: "git-commit", title: "Git Commit", anchor: "right", icon: CommitIcon, component: GitCommitPanel, order: 10, hotkey: "Alt+9" });
registerToolWindow({ id: "terminal", title: "Terminal", anchor: "bottom", icon: TerminalIcon, component: TerminalPanel, order: 10, hotkey: "Alt+F12", defaultOpen: true });
registerToolWindow({ id: "analyzer", title: "Analyzer", anchor: "bottom", icon: AnalyzerIcon, component: AnalyzerPanel, order: 20, hotkey: "Ctrl+Shift+A" });
const { leftTools, rightTools, bottomTools, leftTool, rightTool, bottomTool, activeIds, sizes, toggle, close, setSize, handleHotkey } = tools;

const resizeAnchor = ref<"left" | "right" | "bottom" | null>(null);
let resizeStart = 0;
let resizeInitial = 0;
function startDockResize(anchor: "left" | "right" | "bottom", event: MouseEvent) {
  resizeAnchor.value = anchor;
  resizeStart = anchor === "bottom" ? event.clientY : event.clientX;
  resizeInitial = sizes[anchor];
  window.addEventListener("mousemove", onDockResize);
  window.addEventListener("mouseup", stopDockResize);
  event.preventDefault();
}
function onDockResize(event: MouseEvent) {
  const anchor = resizeAnchor.value;
  if (!anchor) return;
  const current = anchor === "bottom" ? event.clientY : event.clientX;
  const delta = anchor === "left" ? current - resizeStart : resizeStart - current;
  setSize(anchor, resizeInitial + delta);
}
function stopDockResize() {
  resizeAnchor.value = null;
  window.removeEventListener("mousemove", onDockResize);
  window.removeEventListener("mouseup", stopDockResize);
}
function toggleTool(id: string) { toggle(id); }

const activeWorkspace = ref<ActiveWorkspace | null>(null);
const unlistenWorkspace = ref<UnlistenFn | null>(null);
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
function triggerAutosave() {
  if (autosaveTimer) { clearTimeout(autosaveTimer); autosaveTimer = null; }
  if (openFiles.value.some((file) => file.dirty)) void saveAll();
}
watch(openFiles, () => {
  if (autosaveTimer) clearTimeout(autosaveTimer);
  if (!openFiles.value.some((file) => file.dirty)) return;
  autosaveTimer = setTimeout(() => { autosaveTimer = null; void saveAll(); }, 1500);
}, { deep: true });
watch(activeFileId, triggerAutosave);

const tabFiles = computed(() => openFiles.value.map((file) => ({ id: file.id, name: file.name, dirty: file.dirty })));
const saveTooltip = computed(() => activeFile.value ? `Save ${activeFile.value.name}` : "Save file");
const editorValue = computed({
  get: () => activeFile.value?.contents ?? "",
  set: (value: string) => { if (activeFile.value) setContents(activeFile.value.id, value); },
});

async function loadActiveWorkspace() {
  try {
    const workspace = await invoke<ActiveWorkspace | null>("get_active_workspace");
    activeWorkspace.value = workspace ?? null;
    if (workspace) setWorkspacePath(workspace.path);
  } catch (error) {
    editorLoadError.value = `Unable to retrieve the active workspace: ${String(error)}`;
  }
}
async function handleCloseTab(id: number) {
  const file = openFiles.value.find((f) => f.id === id);
  if (file?.dirty && !window.confirm(`"${file.name}" has unsaved changes. Close without saving?`)) return;
  closeFile(id);
}
async function handleSave() {
  if (!activeFile.value) return;
  if (await saveFile(activeFile.value.id)) toast("success", "Saved", activeFile.value.name);
}
async function handleSaveAll() {
  if (await saveAll()) toast("success", "Saved", "All files");
}
function handleKeydown(event: KeyboardEvent) {
  if (handleHotkey(event)) { event.preventDefault(); return; }
  if (event.key === "F12") { event.preventDefault(); return; }
  if (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "i") {
    event.preventDefault(); void invoke("open_devtools"); return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    event.preventDefault(); void handleSave();
  }
}
onMounted(async () => {
  unlistenWorkspace.value = await listen<ActiveWorkspace>("workspace-opened", (event) => {
    activeWorkspace.value = event.payload;
    if (event.payload) setWorkspacePath(event.payload.path);
  });
  await loadActiveWorkspace();
  window.addEventListener("keydown", handleKeydown);
  window.addEventListener("blur", triggerAutosave);
  document.addEventListener("visibilitychange", onVisibilityChange);
});
function onVisibilityChange() { if (document.hidden) triggerAutosave(); }
onBeforeUnmount(() => {
  stopDockResize();
  if (autosaveTimer) clearTimeout(autosaveTimer);
  autosaveTimer = null;
  unlistenWorkspace.value?.();
  window.removeEventListener("keydown", handleKeydown);
  window.removeEventListener("blur", triggerAutosave);
  document.removeEventListener("visibilitychange", onVisibilityChange);
});
</script>

<template>
  <TitleBar>
    <template #default>
      <IdeTitlebarNav :active-workspace="activeWorkspace" @workspace-changed="(ws) => { activeWorkspace = ws; }" />
    </template>
  </TitleBar>

  <div class="ide-shell" v-if="activeWorkspace">
    <div class="ide-body">
      <LumToolStripe side="left" :top="leftTools" :active-ids="activeIds" @toggle="toggleTool" />
      <aside v-if="leftTool" class="ide-dock ide-dock-left" :style="{ width: sizes.left + 'px' }">
        <keep-alive><component :is="leftTool.component" :workspace-path="activeWorkspace.path" @hide="close('left')" /></keep-alive>
      </aside>
      <div v-if="leftTool" class="lum-resizer ide-dock-resizer" @mousedown="startDockResize('left', $event)" :class="{ 'is-dragging': resizeAnchor === 'left' }"></div>

      <div class="ide-editor-area">
        <div class="ide-tabs">
          <LumFileTabs v-model="activeFileId" :files="tabFiles" @close="handleCloseTab">
            <template #extra>
              <div class="ide-tab-actions">
                <button class="ide-tab-action" @click="handleSaveAll" title="Save All" aria-label="Save All">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><path d="M15 17H4a2 2 0 0 1-2-2V4a2 2 0 0 a2-2h8l4 4v9a2 2 0 0 1-2 2z" /><polyline points="13 17 13 10 5 10 5 17" /><polyline points="6 2 6 5 12 5" /></svg>
                </button>
                <button class="ide-tab-action" @click="handleSave" :title="saveTooltip" aria-label="Save">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" /><polyline points="17 21 17 13 7 13 7 21" /><polyline points="7 3 7 8 15 8" /></svg>
                </button>
              </div>
            </template>
          </LumFileTabs>
        </div>
        <div class="ide-editor-surface">
          <LumEditor v-if="activeFile" v-model="editorValue" :model-key="activeFile.id" :language="activeFile.language" />
          <div v-else class="ide-empty"><p>Open a file from the project explorer to start editing.</p></div>
        </div>
      </div>

      <div v-if="rightTool" class="lum-resizer ide-dock-resizer" @mousedown="startDockResize('right', $event)" :class="{ 'is-dragging': resizeAnchor === 'right' }"></div>
      <aside v-if="rightTool" class="ide-dock ide-dock-right" :style="{ width: sizes.right + 'px' }">
        <keep-alive><component :is="rightTool.component" :workspace-path="activeWorkspace.path" @hide="close('right')" /></keep-alive>
      </aside>
      <LumToolStripe side="right" :top="rightTools" :bottom="bottomTools" :active-ids="activeIds" @toggle="toggleTool" />
    </div>

    <div v-if="bottomTool" class="lum-resizer ide-bottom-resizer" @mousedown="startDockResize('bottom', $event)" :class="{ 'is-dragging': resizeAnchor === 'bottom' }"></div>
    <section v-if="bottomTool" class="ide-bottom-dock" :style="{ height: sizes.bottom + 'px' }">
      <keep-alive><component :is="bottomTool.component" :workspace-path="activeWorkspace.path" @hide="close('bottom')" /></keep-alive>
    </section>
  </div>
  <div v-else-if="editorLoadError" class="ide-load-error">{{ editorLoadError }}</div>
  <div v-else class="ide-loading">Loading workspace…</div>

  <LumContextMenu />
  <LumModals />
</template>

<style scoped>
.ide-shell { display:flex; flex-direction:column; height:calc(100% - 40px); min-height:0; }
.ide-body { display:flex; flex:1 1 auto; min-height:0; overflow:hidden; }
.ide-dock { flex:0 0 auto; min-width:0; display:flex; flex-direction:column; overflow:hidden; }
.ide-dock-left { border-right:1px solid var(--lum-border); }
.ide-dock-right { border-left:1px solid var(--lum-border); }
.ide-dock-resizer { flex-shrink:0; width:4px; cursor:col-resize; z-index:var(--lum-z-sticky); touch-action:none; }
.ide-dock-resizer:hover,.ide-dock-resizer.is-dragging,.ide-bottom-resizer:hover,.ide-bottom-resizer.is-dragging { background:var(--lum-accent); }
.ide-bottom-resizer { height:4px; flex:0 0 auto; cursor:row-resize; }
.ide-bottom-dock { flex:0 0 auto; min-height:0; overflow:hidden; border-top:1px solid var(--lum-border); }
.ide-editor-area { flex:1 1 auto; min-width:0; display:flex; flex-direction:column; min-height:0; }
.ide-tabs { flex-shrink:0; }
.ide-tab-actions { display:flex; gap:4px; }
.ide-tab-action { display:inline-flex; align-items:center; justify-content:center; width:30px; height:30px; border:none; background:transparent; color:var(--lum-fg-muted); cursor:pointer; border-radius:var(--lum-radius-sm); }
.ide-tab-action:hover { background:var(--lum-bg-hover); color:var(--lum-fg-strong); }
.ide-editor-surface { flex:1 1 auto; min-height:0; display:flex; flex-direction:column; background:var(--lum-bg-app); position:relative; overflow:hidden; }
.ide-empty { flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; color:var(--lum-fg-muted); padding:var(--lum-space-8); }
.ide-empty p { margin:0; }
.ide-load-error,.ide-loading { padding:1.5rem; text-align:center; color:var(--lum-fg-muted); }
.ide-load-error { color:var(--lum-danger); }
</style>