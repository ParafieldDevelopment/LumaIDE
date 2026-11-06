<script setup lang="ts">
/** LumDivider — horizontal rule (.lum-divider). vertical / dashed / labeled variants. */
defineOptions({ inheritAttrs: false, name: "LumDivider" });

withDefaults(
  defineProps<{
    vertical?: boolean;
    dashed?: boolean;
    label?: string;
  }>(),
  {},
);
</script>

<template>
  <div v-if="label || $slots.default" class="lum-divider label">
    <span><slot>{{ label }}</slot></span>
  </div>
  <div v-else-if="vertical" class="lum-divider vertical" :class="{ dashed }"></div>
  <hr v-else class="lum-divider" :class="{ dashed }" />
</template>
