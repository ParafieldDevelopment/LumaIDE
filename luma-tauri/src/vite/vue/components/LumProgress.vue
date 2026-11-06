<script setup lang="ts">
/**
 * LumProgress — progress bar (.lum-progress).
 * Determinate: pass `value` (0–100). Indeterminate: `indeterminate` (macOS sweep).
 * Colors: success/warning/danger.
 */
defineOptions({ name: "LumProgress" });

withDefaults(
  defineProps<{
    value?: number;
    indeterminate?: boolean;
    striped?: boolean;
    color?: "primary" | "success" | "warning" | "danger";
  }>(),
  { color: "primary" },
);
</script>

<template>
  <div
    class="lum-progress"
    role="progressbar"
    :data-indeterminate="indeterminate || undefined"
    :aria-valuenow="value"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div
      class="lum-progress-bar"
      :class="color !== 'primary' ? color : undefined"
      :data-striped="striped || undefined"
      :style="indeterminate ? undefined : { width: `${value ?? 0}%` }"
    ></div>
  </div>
</template>
