<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, computed, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirmExit } from "~/vue/composables/useConfirm";
import { LumContextMenu, LumModals, LumFileTabs, LumEditor } from "./components";
import TitleBar from "~/vue/window/TitleBar.vue";
import IdeTitlebarNav from "~/vue/window/IdeTitlebarNav.vue";
import { useEditor } from "~/vue/composables/useEditor.ts";
import { useToasts } from "~/vue/components/toasts";
import LumToolStripe from "./components/LumToolStripe.vue";
import { useToolWindows, registerToolWindow, setToolWindowWorkspace, type ToolPosition, type ToolSide } from "~/vue/composables/useToolWindows";
import { FolderIcon, CommitIcon, TerminalIcon, AnalyzerIcon, NotificationIcon } from "~/vue/icons/ToolIcons";
import ProjectExplorerPanel from "./components/ProjectExplorer.vue";
import TerminalPanel from "./toolwindows/TerminalPanel.vue";
import AnalyzerPanel from "./toolwindows/AnalyzerPanel.vue";
import GitCommitPanel from "./toolwindows/GitCommitPanel.vue";
import NotificationsPanel from "./toolwindows/NotificationsPanel.vue";

interface ActiveWorkspace { project_uuid: string; title: string; path: string; git_branch?: string; icon_path?: string }

const { openFiles, activeFileId, activeFile, loadError: editorLoadError, closeFile, saveFile, saveAll, setContents, setWorkspacePath } = useEditor();
const { toast } = useToasts();
const tools = useToolWindows();

registerToolWindow({ id: "project-explorer", title: "Project", position: "left-top", icon: FolderIcon, component: ProjectExplorerPanel, order: 10, hotkey: "Alt+1", defaultOpen: true });
registerToolWindow({ id: "git-commit", title: "Commit", position: "left-top", icon: CommitIcon, component: GitCommitPanel, order: 20, hotkey: "Alt+0" });
registerToolWindow({ id: "terminal", title: "Terminal", position: "left-bottom", icon: TerminalIcon, component: TerminalPanel, order: 20, hotkey: "Alt+F12" });
registerToolWindow({ id: "analyzer", title: "Analyzer", position: "left-bottom", icon: AnalyzerIcon, component: AnalyzerPanel, order: 30, hotkey: "Ctrl+Shift+A" });
registerToolWindow({ id: "notifications", title: "Notifications", position: "right-top", icon: NotificationIcon, component: NotificationsPanel, order: 20 });

const {
  leftTopTools, leftBottomTools, rightTopTools, rightBottomTools,
  leftTool, rightTool, bottomTool, activeIds, sizes, toggle, close, setSize, handleHotkey, move,
} = tools;

const activeWorkspace = ref<ActiveWorkspace | null>(null);
const resizeSide = ref<ToolSide | null>(null);
let resizeStart = 0;
let resizeInitial = 0;

function startDockResize(side: ToolSide, event: MouseEvent) {
  resizeSide.value = side;
  resizeStart = side === "bottom" ? event.clientY : event.clientX;
  resizeInitial = sizes[side];
  window.addEventListener("mousemove", onDockResize);
  window.addEventListener("mouseup", stopDockResize);
  event.preventDefault();
}

function onDockResize(event: MouseEvent) {
  const side = resizeSide.value;
  if (!side) return;
  const delta = side === "left"
    ? event.clientX - resizeStart
    : side === "right"
      ? resizeStart - event.clientX
      : resizeStart - event.clientY;
  setSize(side, resizeInitial + delta);
}

function stopDockResize() {
  resizeSide.value = null;
  window.removeEventListener("mousemove", onDockResize);
  window.removeEventListener("mouseup", stopDockResize);
}

function toggleTool(id: string) {
  toggle(id);
}

// ── Tool button drag (pointer events; works regardless of WebView2 drag handling) ──
const dragTarget = ref<ToolPosition | null>(null);
const dropIndex = ref(0);
const draggingId = ref<string | null>(null);
const DRAG_THRESHOLD = 5;

interface ToolDragState {
  id: string;
  startX: number;
  startY: number;
  started: boolean;
  ghost: HTMLElement | null;
  source: HTMLElement;
}
let toolDrag: ToolDragState | null = null;

function beginToolDrag(id: string, event: PointerEvent) {
  if (event.button !== 0) return;
  cancelToolDrag();
  toolDrag = {
    id,
    startX: event.clientX,
    startY: event.clientY,
    started: false,
    ghost: null,
    source: event.currentTarget as HTMLElement,
  };
  window.addEventListener("pointermove", onToolDragMove);
  window.addEventListener("pointerup", onToolDragEnd);
  window.addEventListener("pointercancel", cancelToolDrag);
  window.addEventListener("keydown", onToolDragKey, true);
}

function onToolDragMove(event: PointerEvent) {
  const drag = toolDrag;
  if (!drag) return;
  if (!drag.started) {
    if (Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) < DRAG_THRESHOLD) return;
    drag.started = true;
    draggingId.value = drag.id;
    const ghost = drag.source.cloneNode(true) as HTMLElement;
    ghost.removeAttribute("title");
    Object.assign(ghost.style, {
      position: "fixed",
      zIndex: "100000",
      pointerEvents: "none",
      opacity: "0.9",
      background: "var(--lum-accent-muted)",
      boxShadow: "0 4px 14px rgba(0,0,0,.4)",
    });
    document.body.appendChild(ghost);
    drag.ghost = ghost;
    document.body.style.cursor = "grabbing";
    document.body.style.userSelect = "none";
  }
  if (drag.ghost) {
    drag.ghost.style.left = `${event.clientX - 16}px`;
    drag.ghost.style.top = `${event.clientY - 16}px`;
  }
  updateDropTarget(event.clientX, event.clientY);
}

function updateDropTarget(x: number, y: number) {
  const drag = toolDrag;
  if (!drag) return;
  const element = document.elementFromPoint(x, y);
  const stripe = element?.closest<HTMLElement>("[data-stripe-side]");
  if (!stripe) {
    dragTarget.value = null;
    return;
  }
  const side = stripe.dataset.stripeSide === "right" ? "right" : "left";
  const section = element?.closest<HTMLElement>("[data-drop-position]");
  let position = section?.dataset.dropPosition as ToolPosition | undefined;
  if (!position) {
    const rect = stripe.getBoundingClientRect();
    position = `${side}-${y < rect.top + rect.height / 2 ? "top" : "bottom"}` as ToolPosition;
  }
  const group = stripe.querySelector<HTMLElement>(`[data-drop-position="${position}"]`);
  const buttons = [...(group?.querySelectorAll<HTMLElement>("[data-tool-id]") ?? [])]
    .filter((button) => button.dataset.toolId !== drag.id);
  let index = 0;
  for (const button of buttons) {
    const rect = button.getBoundingClientRect();
    if (y > rect.top + rect.height / 2) index++;
  }
  dragTarget.value = position;
  dropIndex.value = index;
}

function onToolDragEnd() {
  const drag = toolDrag;
  const target = dragTarget.value;
  const index = dropIndex.value;
  const moved = !!drag?.started;
  const id = drag?.id;
  cleanupToolDrag();
  if (!moved) return;
  // The pointerup is followed by a click on the button; swallow it.
  const swallow = (event: Event) => { event.stopPropagation(); event.preventDefault(); };
  window.addEventListener("click", swallow, { capture: true, once: true });
  setTimeout(() => window.removeEventListener("click", swallow, true), 0);
  if (id && target) move(id, target, index);
}

function onToolDragKey(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    cancelToolDrag();
  }
}

function cancelToolDrag() {
  cleanupToolDrag();
}

function cleanupToolDrag() {
  window.removeEventListener("pointermove", onToolDragMove);
  window.removeEventListener("pointerup", onToolDragEnd);
  window.removeEventListener("pointercancel", cancelToolDrag);
  window.removeEventListener("keydown", onToolDragKey, true);
  toolDrag?.ghost?.remove();
  toolDrag = null;
  draggingId.value = null;
  dragTarget.value = null;
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
}

const unlistenWorkspace = ref<UnlistenFn | null>(null);
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

function triggerAutosave() {
  if (autosaveTimer) {
    clearTimeout(autosaveTimer);
    autosaveTimer = null;
  }
  if (openFiles.value.some((file) => file.dirty)) void saveAll();
}

watch(openFiles, () => {
  if (autosaveTimer) clearTimeout(autosaveTimer);
  if (!openFiles.value.some((file) => file.dirty)) return;
  autosaveTimer = setTimeout(() => {
    autosaveTimer = null;
    void saveAll();
  }, 1500);
}, { deep: true });

watch(activeFileId, triggerAutosave);
const tabFiles = computed(() => openFiles.value.map((file) => ({ id: file.id, name: file.name, dirty: file.dirty })));
const saveTooltip = computed(() => activeFile.value ? `Save ${activeFile.value.name}` : "Save file");
const editorValue = computed({
  get: () => activeFile.value?.contents ?? "",
  set: (value: string) => { if (activeFile.value) setContents(activeFile.value.id, value); },
});

async function syncWorkspace(workspace: ActiveWorkspace | null) {
  activeWorkspace.value = workspace;
  if (workspace) {
    setWorkspacePath(workspace.path);
    await setToolWindowWorkspace(workspace.path);
  } else {
    await setToolWindowWorkspace("");
  }
}

async function loadActiveWorkspace() {
  try {
    const workspace = await invoke<ActiveWorkspace | null>("get_active_workspace");
    await syncWorkspace(workspace);
  } catch (error) {
    editorLoadError.value = `Unable to retrieve the active workspace: ${String(error)}`;
  }
}

async function handleCloseTab(id: number) {
  const file = openFiles.value.find((item) => item.id === id);
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
  if (handleHotkey(event)) {
    event.preventDefault();
    return;
  }
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

// X button and Alt+F4 both arrive here. Ask first, then really close the window.
let confirmingExit = false;
let unlistenClose: UnlistenFn | null = null;

async function handleCloseRequested(event: { preventDefault: () => void }) {
  event.preventDefault();
  if (confirmingExit) return;
  confirmingExit = true;
  try {
    if (!(await confirmExit())) return;
    if (openFiles.value.some((file) => file.dirty)) await saveAll();
    await getCurrentWindow().destroy();
  } finally {
    confirmingExit = false;
  }
}

onMounted(async () => {
  unlistenWorkspace.value = await listen<ActiveWorkspace>("workspace-opened", (event) => { void syncWorkspace(event.payload); });
  unlistenClose = await getCurrentWindow().onCloseRequested(handleCloseRequested);
  await loadActiveWorkspace();
  window.addEventListener("keydown", handleKeydown);
  window.addEventListener("blur", triggerAutosave);
  document.addEventListener("visibilitychange", onVisibilityChange);
});

function onVisibilityChange() {
  if (document.hidden) triggerAutosave();
}

onBeforeUnmount(() => {
  stopDockResize();
  cleanupToolDrag();
  if (autosaveTimer) clearTimeout(autosaveTimer);
  autosaveTimer = null;
  unlistenWorkspace.value?.();
  unlistenClose?.();
  window.removeEventListener("keydown", handleKeydown);
  window.removeEventListener("blur", triggerAutosave);
  document.removeEventListener("visibilitychange", onVisibilityChange);
});
</script>

<template>
  <TitleBar>
    <template #default>
      <IdeTitlebarNav :active-workspace="activeWorkspace" @workspace-changed="(workspace) => { void syncWorkspace(workspace); }" />
    </template>
  </TitleBar>

  <div v-if="activeWorkspace" class="ide-shell">
    <div class="ide-main-row">
      <LumToolStripe
        class="ide-stripe-left"
        side="left"
        :top="leftTopTools"
        :bottom="leftBottomTools"
        :active-ids="activeIds"
        :drop-target="dragTarget"
        :drop-index="dropIndex"
        :dragging-id="draggingId"
        @toggle="toggleTool"
        @tool-pointer-down="beginToolDrag"
      />

      <div class="ide-center">
      <div class="ide-upper-row">
        <aside v-if="leftTool" class="ide-dock ide-dock-left" :style="{ width: sizes.left + 'px' }">
          <keep-alive>
            <component :is="leftTool.component" :workspace-path="activeWorkspace.path" @hide="close('left')" />
          </keep-alive>
        </aside>
        <div
          v-if="leftTool"
          class="lum-resizer ide-dock-resizer"
          :class="{ 'is-dragging': resizeSide === 'left' }"
          @mousedown="startDockResize('left', $event)"
        />

      <div class="ide-editor-area">
        <div class="ide-tabs">
          <LumFileTabs v-model="activeFileId" :files="tabFiles" @close="handleCloseTab">
            <template #extra>
              <div class="ide-tab-actions">
                <button class="ide-tab-action" @click="handleSaveAll" title="Save All" aria-label="Save All">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                    <path d="M15 17H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8l4 4v9a2 2 0 0 1-2 2z" />
                    <polyline points="13 17 13 10 5 10 5 17" />
                    <polyline points="6 2 6 5 12 5" />
                  </svg>
                </button>
                <button class="ide-tab-action" @click="handleSave" :title="saveTooltip" aria-label="Save">
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
          <LumEditor v-if="activeFile" v-model="editorValue" :model-key="activeFile.id" :language="activeFile.language" />
          <div v-else class="ide-empty"><p>Open a file from the project explorer to start editing.</p></div>
        </div>
      </div>

        <div
          v-if="rightTool"
          class="lum-resizer ide-dock-resizer"
          :class="{ 'is-dragging': resizeSide === 'right' }"
          @mousedown="startDockResize('right', $event)"
        />
        <aside v-if="rightTool" class="ide-dock ide-dock-right" :style="{ width: sizes.right + 'px' }">
          <keep-alive>
            <component :is="rightTool.component" :workspace-path="activeWorkspace.path" @hide="close('right')" />
          </keep-alive>
        </aside>
      </div>

        <div
          v-if="bottomTool"
          class="lum-resizer ide-dock-resizer-h"
          :class="{ 'is-dragging': resizeSide === 'bottom' }"
          @mousedown="startDockResize('bottom', $event)"
        />
        <aside v-if="bottomTool" class="ide-dock ide-dock-bottom" :style="{ height: sizes.bottom + 'px' }">
          <keep-alive>
            <component :is="bottomTool.component" :workspace-path="activeWorkspace.path" @hide="close('bottom')" />
          </keep-alive>
        </aside>
      </div>

      <LumToolStripe
        class="ide-stripe-right"
        side="right"
        :top="rightTopTools"
        :bottom="rightBottomTools"
        :active-ids="activeIds"
        :drop-target="dragTarget"
        :drop-index="dropIndex"
        :dragging-id="draggingId"
        @toggle="toggleTool"
        @tool-pointer-down="beginToolDrag"
      />
    </div>
  </div>

  <div v-else-if="editorLoadError" class="ide-load-error">{{ editorLoadError }}</div>
  <div v-else class="ide-loading">Loading workspace…</div>

  <LumContextMenu />
  <LumModals />
</template>

<style scoped>
.ide-shell {
  display: flex;
  flex-direction: column;
  height: calc(100% - 40px);
  min-height: 0;
  overflow: hidden;
}

.ide-main-row {
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
}

.ide-stripe-left {
  border-right: 1px solid var(--lum-border);
}

.ide-stripe-right {
  border-left: 1px solid var(--lum-border);
}

/* Side docks + editor on top, bottom dock underneath, all between the stripes. */
.ide-center {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.ide-upper-row {
  display: flex;
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.ide-dock-bottom {
  width: 100%;
  border-top: 1px solid var(--lum-border);
}

.ide-dock-resizer-h {
  flex: 0 0 3px;
  height: 3px;
  width: 100%;
  cursor: row-resize;
  touch-action: none;
}

.ide-dock-resizer-h:hover,
.ide-dock-resizer-h.is-dragging {
  background: var(--lum-accent);
}

.ide-dock {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.ide-dock-left,
.ide-dock-right {
  border: 0;
}

.ide-dock-resizer {
  flex: 0 0 3px;
  width: 3px;
  cursor: col-resize;
  touch-action: none;
}

.ide-dock-resizer:hover,
.ide-dock-resizer.is-dragging {
  background: var(--lum-accent);
}

.ide-editor-area {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.ide-tabs {
  flex: 0 0 auto;
}

.ide-tab-actions {
  display: flex;
  gap: var(--lum-space-1);
}

.ide-tab-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: var(--lum-radius-sm);
  background: transparent;
  color: var(--lum-fg-muted);
  cursor: pointer;
}

.ide-tab-action:hover {
  background: var(--lum-bg-hover);
  color: var(--lum-fg-strong);
}

.ide-editor-surface {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
  position: relative;
  overflow: hidden;
  background: var(--lum-bg-app);
}

.ide-empty {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--lum-space-8);
  color: var(--lum-fg-muted);
}

.ide-empty p {
  margin: 0;
}

.ide-load-error,
.ide-loading {
  padding: var(--lum-space-6);
  text-align: center;
  color: var(--lum-fg-muted);
}

.ide-load-error {
  color: var(--lum-danger);
}
</style>
