<script setup lang="ts">
/** LumSpinner — circular loader (.lum-spinner). sizes sm/md/lg, colors success/warning/danger. */
defineOptions({ inheritAttrs: false, name: "LumSpinner" });

withDefaults(
  defineProps<{
    size?: "sm" | "md" | "lg";
    color?: "primary" | "success" | "warning" | "danger";
  }>(),
  { size: "md", color: "primary" },
);
</script>

<template>
  <span class="lum-spinner" :class="[size !== 'md' ? size : undefined, color !== 'primary' ? color : undefined]" v-bind="$attrs"></span>
</template>
