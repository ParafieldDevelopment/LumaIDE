<script setup lang="ts">
/** LumChip — removable tag (.lum-chip). Emits `remove` when the × is clicked. */
defineOptions({ name: "LumChip" });

const props = withDefaults(defineProps<{ label: string; removable?: boolean }>(), { removable: false });

defineEmits<{ remove: [label: string] }>();
</script>

<template>
  <span class="lum-chip" v-bind="$attrs">
    {{ label }}
    <button
      v-if="removable"
      class="lum-chip-remove"
      :aria-label="`Remove ${label}`"
      @click.stop="$emit('remove', label)"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="10" height="10"><path d="M18 6 6 18M6 6l12 12" /></svg>
    </button>
  </span>
</template>
