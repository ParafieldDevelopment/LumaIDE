<script setup lang="ts">
/**
 * LumTreeNode — one node of a LumTree (recursive; styles .lum-tree-*).
 * Toggling the chevron opens/closes children. Emits `select` with the node.
 */
import { ref } from "vue";
import type { TreeNode } from "./types";

defineOptions({ name: "LumTreeNode" });

withDefaults(
  defineProps<{
    node: TreeNode;
    depth: number;
    selected?: TreeNode | null;
  }>(),
  { depth: 0, selected: null },
);

defineEmits<{ select: [node: TreeNode] }>();

const open = ref(false);
const hasChildren = (children?: TreeNode[]) => !!children?.length;
</script>

<template>
  <button
    class="lum-tree-item"
    :class="{ open, selected: node === selected, cut: node.cut }"
    :style="{ '--lum-tree-depth': depth }"
    @click="$emit('select', node)"
  >
    <span
      class="lum-tree-chevron"
      :class="{ open, leaf: !hasChildren(node.children) }"
      :aria-label="open ? 'Collapse' : 'Expand'"
      @click.stop="open = !open"
    ></span>
    <span class="lum-tree-label" :class="{ italic: node.ignored }">{{ node.label }}</span>
  </button>

  <div v-if="hasChildren(node.children)" class="lum-tree-children">
    <LumTreeNode
      v-for="child in node.children"
      :key="child.label"
      :node="child"
      :depth="depth + 1"
      :selected="selected"
      @select="(n) => $emit('select', n)"
    />
  </div>
</template>
