<script setup lang="ts">
/** LumIconBtn — square icon-only button (.lum-icon-btn). Pass `tip` to render a tooltip. */
defineOptions({ inheritAttrs: false, name: "LumIconBtn" });

defineProps<{ tip?: string; disabled?: boolean }>();
</script>

<template>
  <button
    class="lum-icon-btn"
    :class="tip ? 'lum-tooltip' : undefined"
    :data-tip="tip"
    :disabled="disabled"
    v-bind="$attrs"
  >
    <slot />
  </button>
</template>
