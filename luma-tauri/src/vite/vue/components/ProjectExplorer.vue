<script setup lang="ts">
/**
 * ProjectExplorer — workspace file tree (.lum-project-explorer).
 * Directories are loaded on demand; filesystem watcher events refresh only the
 * affected directories instead of rebuilding the whole tree.
 */
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEditor } from "~/vue/composables/useEditor";
import { registerContextMenu } from "~/vue/composables/contextMenu";
import { registerActions } from "~/vue/composables/actions";
import { Menus, registerMenu } from "~/vue/composables/menus";
import { FILES_CHANGED_EVENT, type FilesChangedPayload } from "~/vue/composables/fileWatcher";
import { pathKey, parentPath } from "~/vue/composables/paths";
import type { TreeNode } from "~/vue/components/types";
import LumTree from "~/vue/components/LumTree.vue";
import { useToasts } from "~/vue/components/toasts";

const props = defineProps<{
  workspacePath: string | null;
}>();

const { openFile } = useEditor();
const { toast } = useToasts();

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

interface ExplorerNode extends TreeNode {
  path?: string;
  is_dir?: boolean;
}

const rootNode = ref<ExplorerNode | null>(null);
const treeNodes = computed<ExplorerNode[]>(() => (rootNode.value ? [rootNode.value] : []));
const expanded = ref<Set<string>>(new Set<string>());
const nodeIndex = new Map<string, ExplorerNode>();

const loading = ref(false);
const error = ref<string | null>(null);
const explorerRef = ref<HTMLElement | null>(null);

const creatingType = ref<"file" | "folder" | null>(null);
const newItemName = ref("");
const creationInputRef = ref<HTMLInputElement | null>(null);
const creationError = ref("");

let unlistenFiles: UnlistenFn | null = null;
let unregisterMenu: (() => void) | null = null;
let unregisterActions: (() => void) | null = null;
let unregisterMenuEntries: (() => void) | null = null;

function reindex() {
  nodeIndex.clear();
  const visit = (node: ExplorerNode) => {
    if (node.path) nodeIndex.set(pathKey(node.path), node);
    for (const child of (node.children ?? []) as ExplorerNode[]) visit(child);
  };
  if (rootNode.value) visit(rootNode.value);
}

function toNode(entry: FileEntry): ExplorerNode {
  const node: ExplorerNode = {
    label: entry.name,
    path: entry.path,
    is_dir: entry.is_dir,
  };
  if (entry.is_dir) {
    node.children = [];
    node.loaded = false;
  }
  return node;
}

/** Reuse existing nodes so reloading a folder keeps its expanded descendants. */
function nodesFor(entries: FileEntry[]): ExplorerNode[] {
  return entries.map((entry) => {
    const existing = nodeIndex.get(pathKey(entry.path));
    if (existing && !!existing.is_dir === entry.is_dir) {
      existing.label = entry.name;
      existing.path = entry.path;
      return existing;
    }
    return toNode(entry);
  });
}

function readDir(path: string) {
  return invoke<FileEntry[]>("read_directory", { path });
}

async function loadRoot(initial = false) {
  if (!props.workspacePath) {
    rootNode.value = null;
    nodeIndex.clear();
    expanded.value = new Set<string>();
    return;
  }
  if (initial) loading.value = true;
  error.value = null;
  try {
    const entries = await readDir(props.workspacePath);
    const root: ExplorerNode = rootNode.value ?? {
      label: props.workspacePath.split(/[/\\]/).pop() || "Workspace",
      path: props.workspacePath,
      is_dir: true,
    };
    root.path = props.workspacePath;
    root.children = nodesFor(entries);
    root.loaded = true;
    rootNode.value = root;
    reindex();
  } catch (cause) {
    error.value = `Unable to load project files: ${String(cause)}`;
    rootNode.value = null;
    nodeIndex.clear();
  } finally {
    if (initial) loading.value = false;
  }
}

/** Load (or reload) one directory's immediate children. */
async function loadDirectory(node: ExplorerNode) {
  if (!node.is_dir || !node.path || node.loading) return;
  node.loading = true;
  try {
    const entries = await readDir(node.path);
    node.children = nodesFor(entries);
    node.loaded = true;
    reindex();
  } catch {
    // The folder may have been removed externally; its parent will drop it.
    node.children = [];
  } finally {
    node.loading = false;
  }
}

async function toggleNode(node: ExplorerNode) {
  if (!node.path) return;
  const key = pathKey(node.path);
  const next = new Set(expanded.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  expanded.value = next;

  if (next.has(key) && node.is_dir && !node.loaded) {
    await loadDirectory(node);
  }
}

async function handleSelect(node: TreeNode) {
  const explorerNode = node as ExplorerNode;
  if (explorerNode.is_dir) {
    await toggleNode(explorerNode);
    return;
  }
  if (explorerNode.path) await openFile(explorerNode.path, explorerNode.label);
}

/** Refresh the root plus every currently expanded directory. */
async function refreshLoaded() {
  if (!props.workspacePath) return;
  await loadRoot();
  const openDirs = [...nodeIndex.values()].filter(
    (node) => node.is_dir && node.loaded && node !== rootNode.value && expanded.value.has(pathKey(node.path!)),
  );
  await Promise.all(openDirs.map((node) => loadDirectory(node)));
}

/** Apply watcher events to just the directories that changed. */
async function handleChanges(payload: FilesChangedPayload) {
  const parents = new Set<string>();
  for (const change of payload.changes) {
    parents.add(pathKey(parentPath(change.path)));
  }
  await Promise.all(
    [...parents].map(async (key) => {
      const node = nodeIndex.get(key);
      if (node?.is_dir && node.loaded) await loadDirectory(node);
    }),
  );
}

function startCreating(type: "file" | "folder") {
  creatingType.value = type;
  newItemName.value = "";
  creationError.value = "";
  void nextTick(() => {
    creationInputRef.value?.focus();
  });
}

function cancelCreating() {
  creatingType.value = null;
  newItemName.value = "";
  creationError.value = "";
}

async function submitCreation() {
  const name = newItemName.value.trim();
  if (!name) {
    cancelCreating();
    return;
  }
  if (!props.workspacePath) return;

  const sep = props.workspacePath.includes("\\") ? "\\" : "/";
  const targetPath = `${props.workspacePath}${sep}${name}`;

  try {
    if (creatingType.value === "file") {
      await invoke("create_file", { path: targetPath });
      toast("success", "Created", name);
      cancelCreating();
      await refreshLoaded();
      await openFile(targetPath, name);
    } else if (creatingType.value === "folder") {
      await invoke("create_directory", { path: targetPath });
      toast("success", "Created", name);
      cancelCreating();
      await refreshLoaded();
    }
  } catch (cause) {
    creationError.value = String(cause);
    toast("danger", "Failed to create", String(cause));
  }
}

/** Register explorer actions once so the context menu and command palette share them. */
function registerExplorerActions() {
  unregisterActions = registerActions([
    {
      id: "explorer.newFile",
      label: "New File...",
      category: "Explorer",
      when: (context) => !!(context.workspace || props.workspacePath),
      run: () => startCreating("file"),
    },
    {
      id: "explorer.newFolder",
      label: "New Folder...",
      category: "Explorer",
      when: (context) => !!(context.workspace || props.workspacePath),
      run: () => startCreating("folder"),
    },
    {
      id: "explorer.refresh",
      label: "Refresh Explorer",
      category: "Explorer",
      when: (context) => !!(context.workspace || props.workspacePath),
      run: () => refreshLoaded(),
    },
    {
      id: "explorer.reveal",
      label: "Reveal in File Manager",
      category: "Explorer",
      when: (context) => !!(context.workspace || props.workspacePath),
      run: (context) => invoke("reveal_workspace", { path: context.workspace?.path ?? props.workspacePath }),
    },
    {
      id: "explorer.copyPath",
      label: "Copy Path",
      category: "Explorer",
      when: (context) => !!(context.workspace || props.workspacePath),
      run: async (context) => {
        const path = context.workspace?.path ?? props.workspacePath;
        if (!path) return;
        await navigator.clipboard.writeText(path);
        toast("success", "Copied", "Workspace path copied to clipboard");
      },
    },
  ]);

  unregisterMenuEntries = registerMenu(Menus.explorerContext, [
    { commandId: "explorer.newFile", group: "01_create", order: 10 },
    { commandId: "explorer.newFolder", group: "01_create", order: 20 },
    { commandId: "explorer.refresh", group: "02_refresh", order: 10 },
    { commandId: "explorer.reveal", group: "03_system", order: 10 },
    { commandId: "explorer.copyPath", group: "03_system", order: 20 },
  ]);
}

registerExplorerActions();

onMounted(async () => {
  loading.value = true;
  await loadRoot(true);
  loading.value = false;

  if (explorerRef.value) {
    unregisterMenu = registerContextMenu(explorerRef.value, Menus.explorerContext);
  }

  unlistenFiles = await listen<FilesChangedPayload>(FILES_CHANGED_EVENT, (event) => {
    void handleChanges(event.payload);
  });
  // Watcher events are suppressed while unfocused; catch up on focus.
  window.addEventListener("focus", refreshLoaded);
});

onBeforeUnmount(() => {
  unlistenFiles?.();
  unregisterMenu?.();
  unregisterActions?.();
  unregisterMenuEntries?.();
  window.removeEventListener("focus", refreshLoaded);
});

watch(
  () => props.workspacePath,
  () => {
    expanded.value = new Set<string>();
    loading.value = true;
    void loadRoot(true).finally(() => {
      loading.value = false;
    });
  },
);
</script>

<template>
  <div ref="explorerRef" class="lum-project-explorer">
    <div class="lum-project-explorer-header">
      <span class="lum-project-explorer-title" :title="props.workspacePath ?? undefined">
        {{ props.workspacePath ? props.workspacePath.split(/[/\\]/).pop() : "Workspace" }}
      </span>

      <div class="lum-project-explorer-actions">
        <button
          class="lum-explorer-btn"
          title="New File"
          aria-label="New File"
          @click="startCreating('file')"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="12" y1="18" x2="12" y2="12" />
            <line x1="9" y1="15" x2="15" y2="15" />
          </svg>
        </button>
        <button
          class="lum-explorer-btn"
          title="New Folder"
          aria-label="New Folder"
          @click="startCreating('folder')"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            <line x1="12" y1="11" x2="12" y2="17" />
            <line x1="9" y1="14" x2="15" y2="14" />
          </svg>
        </button>
        <button
          class="lum-explorer-btn"
          title="Refresh Explorer"
          aria-label="Refresh Explorer"
          @click="refreshLoaded"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
            <polyline points="23 4 23 10 17 10" />
            <polyline points="1 20 1 14 7 14" />
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Inline file / folder creation form -->
    <div v-if="creatingType" class="lum-creation-row">
      <span class="lum-creation-icon">
        <svg v-if="creatingType === 'file'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        </svg>
      </span>
      <input
        ref="creationInputRef"
        v-model="newItemName"
        class="lum-creation-input"
        :placeholder="creatingType === 'file' ? 'filename.lua' : 'folder_name'"
        @keydown.enter.prevent="submitCreation"
        @keydown.escape.prevent="cancelCreating"
        @blur="submitCreation"
      />
    </div>

    <div v-if="loading" class="lum-project-explorer-loading">
      Loading…
    </div>
    <div v-else-if="error" class="lum-project-explorer-error">
      {{ error }}
    </div>
    <div v-else class="lum-project-explorer-body">
      <LumTree :nodes="treeNodes" :expanded="expanded" @select="handleSelect" @toggle="toggleNode" />
    </div>
  </div>
</template>

<style scoped>
.lum-project-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  border-right: 1px solid var(--lum-border);
  background: var(--lum-bg-surface);
}

.lum-project-explorer-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--lum-space-2) var(--lum-space-3);
  border-bottom: 1px solid var(--lum-border-muted);
  font-size: var(--lum-text-sm);
  font-weight: var(--lum-font-weight-semibold);
  color: var(--lum-fg-muted);
  user-select: none;
}

.lum-project-explorer-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.lum-project-explorer-actions {
  display: flex;
  gap: 2px;
  margin-left: 0.5rem;
}

.lum-explorer-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--lum-fg-muted);
  border-radius: var(--lum-radius-sm);
  cursor: pointer;
  transition: background var(--lum-transition-fast), color var(--lum-transition-fast);
}

.lum-explorer-btn:hover {
  background: var(--lum-bg-hover);
  color: var(--lum-fg-strong);
}

.lum-creation-row {
  display: flex;
  align-items: center;
  padding: 4px 8px;
  background: var(--lum-bg-app);
  border-bottom: 1px solid var(--lum-border);
  gap: 6px;
}

.lum-creation-icon {
  color: var(--lum-fg-muted);
  display: inline-flex;
  align-items: center;
}

.lum-creation-input {
  flex: 1;
  min-width: 0;
  background: var(--lum-bg-surface);
  border: 1px solid var(--lum-accent);
  border-radius: var(--lum-radius-sm);
  color: var(--lum-fg);
  font-size: var(--lum-text-xs);
  padding: 2px 6px;
  outline: none;
  font-family: var(--lum-font-mono);
}

.lum-project-explorer-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
}

.lum-project-explorer-loading,
.lum-project-explorer-error {
  padding: var(--lum-space-4);
  text-align: center;
  font-size: var(--lum-text-sm);
  color: var(--lum-fg-muted);
}

.lum-project-explorer-error {
  color: var(--lum-danger);
}
</style>
