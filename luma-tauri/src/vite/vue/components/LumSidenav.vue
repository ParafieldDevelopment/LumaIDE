<script setup lang="ts">
/** LumSidenav — vertical nav container (.lum-sidenav). Fill with LumSidenavItem. */
defineOptions({ inheritAttrs: false, name: "LumSidenav" });
</script>

<template>
  <nav class="lum-sidenav" v-bind="$attrs">
    <slot />
  </nav>
</template>
