<script setup lang="ts">
/** LumToolbar — horizontal action bar (.lum-toolbar). Optional leading `title`. */
defineOptions({ inheritAttrs: false, name: "LumToolbar" });

withDefaults(defineProps<{ title?: string }>(), {});
</script>

<template>
  <div class="lum-toolbar" v-bind="$attrs">
    <span v-if="title" class="lum-toolbar-title">{{ title }}</span>
    <slot />
  </div>
</template>
