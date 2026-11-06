<script setup lang="ts">
/** LumBreadcrumbs — path trail (.lum-breadcrumbs). Last item is auto-highlighted. */
defineOptions({ name: "LumBreadcrumbs" });

withDefaults(defineProps<{ items: string[] }>(), {});
</script>

<template>
  <nav class="lum-breadcrumbs" v-bind="$attrs">
    <span
      v-for="(item, i) in items"
      :key="i"
      class="lum-breadcrumb"
      :aria-current="i === items.length - 1 ? 'page' : undefined"
    >{{ item }}</span>
  </nav>
</template>
