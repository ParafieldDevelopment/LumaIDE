<script setup lang="ts">
/**
 * LumAlert — callout banner (.lum-alert). kind: info/success/warning/danger.
 * Override the icon with the `icon` slot. `dismissible` shows a close button.
 */
defineOptions({ name: "LumAlert" });

withDefaults(
  defineProps<{
    kind?: "info" | "success" | "warning" | "danger";
    title?: string;
    dismissible?: boolean;
  }>(),
  { kind: "info" },
);

defineEmits<{ close: [] }>();
</script>

<template>
  <div class="lum-alert" :class="kind">
    <slot name="icon">
      <svg v-if="kind === 'success'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 6 9 17l-5-5" /></svg>
      <svg v-else-if="kind === 'warning'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m21.7 18-8-14a2 2 0 0 0-3.4 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.7-3z" /><path d="M12 9v4M12 17h.01" /></svg>
      <svg v-else-if="kind === 'danger'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" /><path d="m15 9-6 6M9 9l6 6" /></svg>
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" /><path d="M12 16v-4M12 8h.01" /></svg>
    </slot>
    <div>
      <div v-if="title" class="lum-alert-title">{{ title }}</div>
      <div class="lum-alert-body"><slot /></div>
    </div>
    <button v-if="dismissible" class="lum-icon-btn lum-alert-close" aria-label="Dismiss" @click="$emit('close')">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12"><path d="M18 6 6 18M6 6l12 12" /></svg>
    </button>
  </div>
</template>
