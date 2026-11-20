<script setup lang="ts">
/**
 * ProjectExplorer — workspace file tree (.lum-project-explorer).
 * Loads the recursive directory tree for the active workspace and lets the
 * user open files. Reuses the existing LumTree / LumTreeNode components.
 */
import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useEditor } from "~/vue/composables/useEditor";
import type { TreeNode } from "~/vue/components/types";
import LumTree from "~/vue/components/LumTree.vue";

const props = defineProps<{
  workspacePath: string | null;
}>();

const { openFile } = useEditor();

interface TreeNodeWithPath extends TreeNode {
  path?: string;
}

const treeNodes = ref<TreeNode[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

function toNode(entry: any): TreeNodeWithPath {
  const node: TreeNodeWithPath = { label: entry.name };
  if (entry.is_dir) {
    node.children = (entry.children ?? []).map(toNode);
  } else {
    node.path = entry.path;
  }
  return node;
}

async function loadTree() {
  if (!props.workspacePath) {
    treeNodes.value = [];
    return;
  }
  loading.value = true;
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
  const explorerNode = node as TreeNode & { path?: string };
  if (explorerNode.children && explorerNode.children.length > 0) {
    return; // directory: expand/collapse is handled by the tree itself
  }
  if (!explorerNode.path) return;
  await openFile(explorerNode.path);
}

onMounted(loadTree);
watch(
  () => props.workspacePath,
  (path) => {
    if (path) loadTree();
  }
);
</script>

<template>
  <div class="lum-project-explorer">
    <div class="lum-project-explorer-header">
      <span class="lum-project-explorer-title">
        {{ props.workspacePath ? props.workspacePath.split(/[/\\]/).pop() : "Workspace" }}
      </span>
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
  padding: var(--lum-space-2) var(--lum-space-3);
  border-bottom: 1px solid var(--lum-border-muted);
  font-size: var(--lum-text-sm);
  font-weight: var(--lum-font-weight-semibold);
  color: var(--lum-fg-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
