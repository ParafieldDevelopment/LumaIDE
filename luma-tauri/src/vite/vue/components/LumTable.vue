<script setup lang="ts">
/** LumTable — scrollable table shell (.lum-table-wrapper + .lum-table). Write th/td directly in the slot. */
defineOptions({ inheritAttrs: false, name: "LumTable" });
</script>

<template>
  <div class="lum-table-wrapper" v-bind="$attrs">
    <table class="lum-table">
      <slot />
    </table>
  </div>
</template>
