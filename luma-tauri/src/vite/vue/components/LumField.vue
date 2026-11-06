<script setup lang="ts">
/**
 * LumField — labeled form group (.lum-field). Wraps any input; `error` takes
 * priority over `help` and turns the help line red.
 */
defineOptions({ name: "LumField" });

withDefaults(
  defineProps<{
    label?: string;
    help?: string;
    error?: string;
  }>(),
  {},
);
</script>

<template>
  <div class="lum-field">
    <label v-if="label" class="lum-label">{{ label }}</label>
    <slot />
    <span v-if="help || error || $slots.help" class="lum-help" :data-error="error ? 'true' : undefined">
      <slot name="help">{{ error ?? help }}</slot>
    </span>
  </div>
</template>
