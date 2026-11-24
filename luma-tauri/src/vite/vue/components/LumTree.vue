<script setup lang="ts">
/** LumTree — file-explorer tree (.lum-tree). Pass a `nodes` forest of TreeNode. */
import type { TreeNode } from "./types";
import LumTreeNode from "./LumTreeNode.vue";

defineOptions({ inheritAttrs: false, name: "LumTree" });

withDefaults(
  defineProps<{
    nodes: TreeNode[];
    selected?: TreeNode | null;
    /** Path keys of expanded directories. */
    expanded?: Set<string>;
  }>(),
  { selected: null, expanded: () => new Set<string>() },
);

defineEmits<{
  select: [node: TreeNode];
  toggle: [node: TreeNode];
}>();
</script>

<template>
  <div class="lum-tree" v-bind="$attrs">
    <LumTreeNode
      v-for="node in nodes"
      :key="node.path ?? node.label"
      :node="node"
      :depth="0"
      :selected="selected"
      :expanded="expanded"
      @select="$emit('select', $event)"
      @toggle="$emit('toggle', $event)"
    />
  </div>
</template>
