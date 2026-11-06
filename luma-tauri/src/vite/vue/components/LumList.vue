<script setup lang="ts">
/** LumList — vertical selectable list container (.lum-list). Fill with LumListItem. */
defineOptions({ inheritAttrs: false, name: "LumList" });
</script>

<template>
  <div class="lum-list" v-bind="$attrs">
    <slot />
  </div>
</template>
