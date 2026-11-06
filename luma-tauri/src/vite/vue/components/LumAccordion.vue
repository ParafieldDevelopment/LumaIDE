<script setup lang="ts">
/** LumAccordion — collapsible sections container (.lum-accordion). Fill with LumAccordionItem. */
defineOptions({ inheritAttrs: false, name: "LumAccordion" });
</script>

<template>
  <div class="lum-accordion" v-bind="$attrs">
    <slot />
  </div>
</template>
