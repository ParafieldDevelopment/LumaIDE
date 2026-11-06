<script setup lang="ts">
/** LumListItem — selectable row (.lum-list-item). `meta` slot for right-aligned hints. */
defineOptions({ inheritAttrs: false, name: "LumListItem" });

withDefaults(
  defineProps<{
    selected?: boolean;
    disabled?: boolean;
  }>(),
  { selected: false, disabled: false },
);

defineEmits<{ select: [] }>();
</script>

<template>
  <button
    class="lum-list-item"
    :class="{ selected, disabled }"
    :aria-selected="selected"
    v-bind="$attrs"
    @click="!disabled && $emit('select')"
  >
    <span class="lum-list-item-label"><slot /></span>
    <span v-if="$slots.meta" class="lum-list-item-meta"><slot name="meta" /></span>
  </button>
</template>
