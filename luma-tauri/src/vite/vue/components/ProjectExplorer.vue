<script setup lang="ts">
/**
 * ProjectExplorer — workspace file tree (.lum-project-explorer).
 * Loads the recursive directory tree for the active workspace and lets the
 * user create, delete and open files with auto-updating and manual refresh.
 */
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useEditor } from "~/vue/composables/useEditor";
import { registerContextMenu } from "~/vue/composables/contextMenu";
import { registerActions } from "~/vue/composables/actions";
import { Menus, registerMenu } from "~/vue/composables/menus";
import type { TreeNode } from "~/vue/components/types";
import LumTree from "~/vue/components/LumTree.vue";
import { useToasts } from "~/vue/components/toasts";

const props = defineProps<{
  workspacePath: string | null;
}>();

const { openFile } = useEditor();
const { toast } = useToasts();

interface TreeNodeWithPath extends TreeNode {
  path?: string;
  is_dir?: boolean;
}

const treeNodes = ref<TreeNodeWithPath[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const explorerRef = ref<HTMLElement | null>(null);

const creatingType = ref<"file" | "folder" | null>(null);
const newItemName = ref("");
const creationInputRef = ref<HTMLInputElement | null>(null);
const creationError = ref("");

let pollInterval: ReturnType<typeof setInterval> | null = null;
let unregisterMenu: (() => void) | null = null;
let unregisterActions: (() => void) | null = null;
let unregisterMenuEntries: (() => void) | null = null;

function toNode(entry: any): TreeNodeWithPath {
  const node: TreeNodeWithPath = {
    label: entry.name,
    path: entry.path,
    is_dir: entry.is_dir,
  };
  if (entry.is_dir) {
    node.children = (entry.children ?? []).map(toNode);
  }
  return node;
}

async function loadTree() {
  if (!props.workspacePath) {
    treeNodes.value = [];
    return;
  }
  error.value = null;
  try {
    const root = await invoke<any>("read_directory_tree", {
      path: props.workspacePath,
    });
    treeNodes.value = root ? [toNode(root)] : [];
  } catch (e) {
    error.value = `Unable to load project files: ${String(e)}`;
    treeNodes.value = [];
  } finally {
    loading.value = false;
  }
}

async function handleSelect(node: TreeNode) {
  const explorerNode = node as TreeNodeWithPath;
  if (explorerNode.is_dir || (explorerNode.children && explorerNode.children.length > 0)) {
    return; // Directory: expansion is handled by LumTreeNode
  }
  if (!explorerNode.path) return;
  await openFile(explorerNode.path);
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
      await loadTree();
      await openFile(targetPath, name);
    } else if (creatingType.value === "folder") {
      await invoke("create_directory", { path: targetPath });
      toast("success", "Created", name);
      cancelCreating();
      await loadTree();
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
      run: () => loadTree(),
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

onMounted(() => {
  loading.value = true;
  void loadTree();

  if (explorerRef.value) {
    unregisterMenu = registerContextMenu(explorerRef.value, Menus.explorerContext);
  }

  // Auto-refresh when user focuses window
  window.addEventListener("focus", loadTree);

  // Background poll every 4 seconds to sync external changes
  pollInterval = setInterval(loadTree, 4000);
});

onBeforeUnmount(() => {
  unregisterMenu?.();
  unregisterActions?.();
  unregisterMenuEntries?.();
  window.removeEventListener("focus", loadTree);
  if (pollInterval) clearInterval(pollInterval);
});

watch(
  () => props.workspacePath,
  (path) => {
    if (path) {
      loading.value = true;
      void loadTree();
    }
  }
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
          @click="loadTree"
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
      <LumTree :nodes="treeNodes" @select="handleSelect" />
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
