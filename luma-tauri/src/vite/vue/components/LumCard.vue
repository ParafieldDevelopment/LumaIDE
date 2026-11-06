<script setup lang="ts">
/** LumCard — panel/card (.lum-card). Optional `header` and `footer` slots. */
defineOptions({ name: "LumCard" });

withDefaults(
  defineProps<{
    interactive?: boolean;
    elevated?: boolean;
    flat?: boolean;
  }>(),
  {},
);
</script>

<template>
  <div class="lum-card" :class="{ interactive, elevated, flat }" v-bind="$attrs">
    <div v-if="$slots.header" class="lum-card-header"><slot name="header" /></div>
    <slot />
    <div v-if="$slots.footer" class="lum-card-footer"><slot name="footer" /></div>
  </div>
</template>
