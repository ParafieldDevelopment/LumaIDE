<script setup lang="ts">
/** LumTree — file-explorer tree (.lum-tree). Pass a `nodes` forest of TreeNode. */
import type { TreeNode } from "./types";
import LumTreeNode from "./LumTreeNode.vue";

defineOptions({ inheritAttrs: false, name: "LumTree" });

withDefaults(
  defineProps<{
    nodes: TreeNode[];
    selected?: TreeNode | null;
  }>(),
  { selected: null },
);

defineEmits<{ select: [node: TreeNode] }>();
</script>

<template>
  <div class="lum-tree" v-bind="$attrs">
    <LumTreeNode
      v-for="node in nodes"
      :key="node.label"
      :node="node"
      :depth="0"
      :selected="selected"
      @select="$emit('select', $event)"
    />
  </div>
</template>
