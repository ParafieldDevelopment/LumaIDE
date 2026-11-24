<script setup lang="ts">
/**
 * LumTreeNode — one node of a LumTree (recursive; styles .lum-tree-*).
 * Expansion is controlled by the parent through `expanded` so directories can
 * be loaded lazily and restored after incremental tree updates.
 */
import { computed } from "vue";
import type { TreeNode } from "./types";
import { pathKey } from "~/vue/composables/paths";

defineOptions({ name: "LumTreeNode" });

const props = withDefaults(
  defineProps<{
    node: TreeNode;
    depth: number;
    selected?: TreeNode | null;
    expanded?: Set<string>;
  }>(),
  { depth: 0, selected: null, expanded: () => new Set<string>() },
);

const emit = defineEmits<{
  select: [node: TreeNode];
  toggle: [node: TreeNode];
}>();

// `expanded` holds pathKey()-normalized keys (lowercase, forward slashes), so
// look nodes up the same way. Raw paths never matched on Windows.
const key = computed(() => (props.node.path ? pathKey(props.node.path) : props.node.label));
const isDir = computed(() => props.node.is_dir !== false);
const open = computed(() => props.expanded.has(key.value));
const hasChildren = computed(() => (props.node.children?.length ?? 0) > 0);
</script>

<template>
  <button
    class="lum-tree-item"
    :class="{ open, selected: node === selected, cut: node.cut }"
    :style="{ '--lum-tree-depth': depth }"
    @click="emit('select', node)"
  >
    <span
      class="lum-tree-chevron"
      :class="{ open, leaf: !isDir, loading: node.loading }"
      :aria-label="open ? 'Collapse' : 'Expand'"
      @click.stop="isDir && emit('toggle', node)"
    ></span>
    <span class="lum-tree-label" :class="{ italic: node.ignored }">{{ node.label }}</span>
  </button>

  <div v-if="open && isDir && hasChildren" class="lum-tree-children">
    <LumTreeNode
      v-for="child in node.children"
      :key="child.path ?? child.label"
      :node="child"
      :depth="depth + 1"
      :selected="selected"
      :expanded="expanded"
      @select="(n) => emit('select', n)"
      @toggle="(n) => emit('toggle', n)"
    />
  </div>
</template>
