<script setup lang="ts">
/** LumMenu — dropdown/context menu container (.lum-menu). Fill with LumMenuItem / LumMenu. */
defineOptions({ inheritAttrs: false, name: "LumMenu" });
</script>

<template>
  <div class="lum-menu" v-bind="$attrs">
    <div class="lum-menu-label" v-if="$slots.label"><slot name="label" /></div>
    <slot />
  </div>
</template>
