<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, computed, watch } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirmExit } from "~/vue/composables/useConfirm";
import { LumContextMenu, LumModals, LumFileTabs, LumEditor, LumCommandPalette } from "./components";
import TitleBar from "~/vue/window/TitleBar.vue";
import IdeTitlebarNav from "~/vue/window/IdeTitlebarNav.vue";
import { useEditor, handleExternalChange } from "~/vue/composables/useEditor.ts";
import { FILES_CHANGED_EVENT, type FilesChangedPayload } from "~/vue/composables/fileWatcher";
import { useToasts } from "~/vue/components/toasts";
import LumToolStripe from "./components/LumToolStripe.vue";
import { useToolWindows, registerToolWindow, setToolWindowWorkspace, type ToolPosition, type ToolSide } from "~/vue/composables/useToolWindows";
import { FolderIcon, CommitIcon, TerminalIcon, AnalyzerIcon, NotificationIcon } from "~/vue/icons/ToolIcons";
import ProjectExplorerPanel from "./components/ProjectExplorer.vue";
import TerminalPanel from "./toolwindows/TerminalPanel.vue";
import AnalyzerPanel from "./toolwindows/AnalyzerPanel.vue";
import GitCommitPanel from "./toolwindows/GitCommitPanel.vue";
import NotificationsPanel from "./toolwindows/NotificationsPanel.vue";
import {
  handleActionKeydown,
  registerActions,
  updateActionContext,
} from "~/vue/composables/actions";
import { Menus, registerMenu } from "~/vue/composables/menus";
import { commandPaletteOpen } from "~/vue/composables/commandPalette";

interface ActiveWorkspace { project_uuid: string; title: string; path: string; git_branch?: string; icon_path?: string }

const { openFiles, activeFileId, activeFile, loadError: editorLoadError, closeFile, saveFile, saveAll, setContents, setWorkspacePath } = useEditor();
const { toast } = useToasts();
const tools = useToolWindows();

registerToolWindow({ id: "project-explorer", title: "Project", position: "left-top", icon: FolderIcon, component: ProjectExplorerPanel, order: 10, hotkey: "Alt+1", defaultOpen: true });
registerToolWindow({ id: "git-commit", title: "Commit", position: "left-top", icon: CommitIcon, component: GitCommitPanel, order: 20, hotkey: "Alt+0" });
registerToolWindow({ id: "terminal", title: "Terminal", position: "left-bottom", icon: TerminalIcon, component: TerminalPanel, order: 20, hotkey: "Alt+F12" });
registerToolWindow({ id: "analyzer", title: "Analyzer", position: "right-bottom", icon: AnalyzerIcon, component: AnalyzerPanel, order: 30, hotkey: "Ctrl+Shift+A" });
registerToolWindow({ id: "notifications", title: "Notifications", position: "right-top", icon: NotificationIcon, component: NotificationsPanel, order: 20 });

// ── Unified action system: commands, keybindings and menus share one registry ──
const actionDisposers: Array<() => void> = [];
const menuDisposers: Array<() => void> = [];

actionDisposers.push(registerActions([
  {
    id: "file.save",
    label: "Save",
    category: "File",
    shortcut: "Ctrl+S",
    keys: "Ctrl+S",
    icon: "M5 3h12l2 2v16H5zM8 3v6h8V3M8 21v-8h8v8",
    when: (context) => !!context.activeFile,
    run: () => handleSave(),
  },
  {
    id: "file.saveAll",
    label: "Save All",
    category: "File",
    when: () => openFiles.value.some((file) => file.dirty),
    run: () => handleSaveAll(),
  },
  {
    id: "editor.closeTab",
    label: "Close Tab",
    category: "Editor",
    when: (context) => !!context.activeFile,
    run: (context) => handleCloseTab(context.activeFile?.id ?? activeFileId.value),
  },
  {
    id: "editor.closeAllTabs",
    label: "Close All Tabs",
    category: "Editor",
    when: () => openFiles.value.length > 0,
    run: () => {
      for (const file of [...openFiles.value]) closeFile(file.id);
    },
  },
  {
    id: "view.hideToolWindows",
    label: "Hide All Tool Windows",
    category: "View",
    shortcut: "Ctrl+Shift+F12",
    keys: "Ctrl+Shift+F12",
    run: () => tools.hideAll(),
  },
  {
    id: "git.commit",
    label: "Git: Commit...",
    category: "Version Control",
    when: (context) => !!context.workspace,
    run: () => open("git-commit"),
  },
  {
    id: "git.push",
    label: "Git: Push...",
    category: "Version Control",
    when: (context) => !!context.workspace,
    run: () => {
      open("git-commit");
      void emit("git-panel-action", { action: "push" });
    },
  },
  {
    id: "git.update",
    label: "Git: Update Project",
    category: "Version Control",
    when: (context) => !!context.workspace,
    run: () => {
      open("git-commit");
      void emit("git-panel-action", { action: "pull" });
    },
  },
]));

const toolWindowEntries = tools.allTools.value.map((tool, index) => {
  const id = `view.toggle.${tool.id}`;
  actionDisposers.push(registerActions([{
    id,
    label: `Toggle ${tool.title} Tool Window`,
    category: "View",
    shortcut: tool.hotkey,
    keys: tool.hotkey,
    when: (context) => !!context.workspace,
    run: () => toggle(tool.id),
  }]));
  return { commandId: id, group: "04_view", order: 10 + index };
});

menuDisposers.push(
  registerMenu(Menus.titlebarMain, [
    { commandId: "file.save", group: "01_file", order: 10 },
    { commandId: "file.saveAll", group: "01_file", order: 20 },
    ...toolWindowEntries,
  ]),
  registerMenu(Menus.editorTabContext, [
    { commandId: "file.save", group: "01_file", order: 10 },
    { commandId: "file.saveAll", group: "01_file", order: 20 },
    { commandId: "editor.closeTab", group: "02_tabs", order: 10 },
    { commandId: "editor.closeAllTabs", group: "02_tabs", order: 20 },
  ]),
);

const {
  leftTopTools, leftBottomTools, rightTopTools, rightBottomTools,
  leftTool, rightTool, bottomLeftTool, bottomRightTool, activeIds, sizes, toggle, open, close, setSize, move,
} = tools;

const activeWorkspace = ref<ActiveWorkspace | null>(null);
const resizeSide = ref<ToolSide | "bottom-split" | null>(null);
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
  if (!side || side === "bottom-split") return;
  const delta = side === "left"
    ? event.clientX - resizeStart
    : side === "right"
      ? resizeStart - event.clientX
      : side === "bottom-left"
        ? event.clientX - resizeStart
        : side === "bottom-right"
          ? resizeStart - event.clientX
          : resizeStart - event.clientY;
  setSize(side, resizeInitial + delta);
}

function stopDockResize() {
  resizeSide.value = null;
  window.removeEventListener("mousemove", onDockResize);
  window.removeEventListener("mouseup", stopDockResize);
}

let bottomSplitStart = 0;
let bottomSplitInitialLeft = 0;
let bottomSplitInitialRight = 0;

function startBottomSplitResize(event: MouseEvent) {
  resizeSide.value = "bottom-split";
  bottomSplitStart = event.clientX;
  bottomSplitInitialLeft = sizes["bottom-left"];
  bottomSplitInitialRight = sizes["bottom-right"];
  window.addEventListener("mousemove", onBottomSplitResize);
  window.addEventListener("mouseup", stopBottomSplitResize);
  event.preventDefault();
}

function onBottomSplitResize(event: MouseEvent) {
  const delta = event.clientX - bottomSplitStart;
  let newLeft = bottomSplitInitialLeft + delta;
  let newRight = bottomSplitInitialRight - delta;
  const min = 200;
  newLeft = Math.max(min, newLeft);
  newRight = Math.max(min, newRight);
  setSize("bottom-left", newLeft);
  setSize("bottom-right", newRight);
}

function stopBottomSplitResize() {
  resizeSide.value = null;
  window.removeEventListener("mousemove", onBottomSplitResize);
  window.removeEventListener("mouseup", stopBottomSplitResize);
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
let unlistenFiles: UnlistenFn | null = null;
let unlistenGitBranch: UnlistenFn | null = null;
const AUTOSAVE_MS = 1000;
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

/** JetBrains-style autosave: 1 second after the last content/keyboard activity. */
function scheduleAutosave() {
  if (autosaveTimer) {
    clearTimeout(autosaveTimer);
    autosaveTimer = null;
  }
  if (!openFiles.value.some((file) => file.dirty)) return;
  autosaveTimer = setTimeout(() => {
    autosaveTimer = null;
    void saveAll(true); // never pop a conflict dialog in the background
  }, AUTOSAVE_MS);
}

function flushAutosave() {
  if (autosaveTimer) {
    clearTimeout(autosaveTimer);
    autosaveTimer = null;
  }
  void saveAll(true);
}

watch(openFiles, scheduleAutosave, { deep: true });
watch(activeFileId, flushAutosave);
const tabFiles = computed(() => openFiles.value.map((file) => ({ id: file.id, name: file.name, dirty: file.dirty })));
const saveTooltip = computed(() => activeFile.value ? `Save ${activeFile.value.name}` : "Save file");
const editorValue = computed({
  get: () => activeFile.value?.contents ?? "",
  set: (value: string) => { if (activeFile.value) setContents(activeFile.value.id, value); },
});

watch(activeFile, (file) => {
  updateActionContext({
    activeFile: file ? { id: file.id, path: file.path, name: file.name, dirty: file.dirty } : null,
  });
}, { immediate: true, deep: true });

async function syncWorkspace(workspace: ActiveWorkspace | null) {
  activeWorkspace.value = workspace;
  updateActionContext({ workspace: workspace ? { path: workspace.path, title: workspace.title } : null });
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
  // All keybindings come from the action registry now.
  handleActionKeydown(event);
  // Keep pushing the autosave deadline while the user is typing.
  if (activeFile.value?.dirty) scheduleAutosave();
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
  unlistenFiles = await listen<FilesChangedPayload>(FILES_CHANGED_EVENT, (event) => {
    for (const change of event.payload.changes) void handleExternalChange(change.path);
  });
  unlistenGitBranch = await listen<{ branch: string }>("workspace-git-branch-changed", (event) => {
    if (activeWorkspace.value) activeWorkspace.value.git_branch = event.payload.branch;
  });
  unlistenClose = await getCurrentWindow().onCloseRequested(handleCloseRequested);
  await loadActiveWorkspace();
  window.addEventListener("keydown", handleKeydown, true);
  window.addEventListener("blur", flushAutosave);
  document.addEventListener("visibilitychange", onVisibilityChange);
});

function onVisibilityChange() {
  if (document.hidden) flushAutosave();
}

onBeforeUnmount(() => {
  actionDisposers.forEach((dispose) => dispose());
  menuDisposers.forEach((dispose) => dispose());
  stopDockResize();
  stopBottomSplitResize();
  cleanupToolDrag();
  if (autosaveTimer) clearTimeout(autosaveTimer);
  autosaveTimer = null;
  unlistenWorkspace.value?.();
  unlistenFiles?.();
  unlistenGitBranch?.();
  unlistenClose?.();
  window.removeEventListener("keydown", handleKeydown, true);
  window.removeEventListener("blur", flushAutosave);
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
        <div class="ide-tabs" v-context-menu="'editor/tab/context'">
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
          v-if="bottomLeftTool || bottomRightTool"
          class="lum-resizer ide-dock-resizer-h"
          :class="{ 'is-dragging': resizeSide === 'bottom' }"
          @mousedown="startDockResize('bottom', $event)"
        />
        <div v-if="bottomLeftTool || bottomRightTool" class="ide-bottom-row" :style="{ height: sizes.bottom + 'px' }">
          <aside v-if="bottomLeftTool" class="ide-dock ide-dock-bottom-left" :style="{ flex: `1 1 ${sizes['bottom-left']}px`, minWidth: '200px' }">
            <keep-alive>
              <component
                :is="bottomLeftTool.component"
                :workspace-path="activeWorkspace.path"
                @hide="close('bottom-left')"
              />
            </keep-alive>
          </aside>
          <div
            v-if="bottomLeftTool && bottomRightTool"
            class="lum-resizer ide-dock-resizer-v"
            @mousedown="startBottomSplitResize($event)"
          />
          <aside v-if="bottomRightTool" class="ide-dock ide-dock-bottom-right" :style="{ flex: `1 1 ${sizes['bottom-right']}px`, minWidth: '200px' }">
            <keep-alive>
              <component
                :is="bottomRightTool.component"
                :workspace-path="activeWorkspace.path"
                @hide="close('bottom-right')"
              />
            </keep-alive>
          </aside>
        </div>
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

  <LumCommandPalette v-model:open="commandPaletteOpen" />
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

.ide-bottom-row {
  display: flex;
  flex: 0 0 auto;
  min-height: 0;
  overflow: hidden;
  border-top: 1px solid var(--lum-border);
}

.ide-dock-bottom-left,
.ide-dock-bottom-right {
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
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

.ide-dock-resizer-v {
  flex: 0 0 3px;
  width: 3px;
  height: 100%;
  cursor: col-resize;
  touch-action: none;
  background: transparent;
}

.ide-dock-resizer-v:hover,
.ide-dock-resizer-v.is-dragging {
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
