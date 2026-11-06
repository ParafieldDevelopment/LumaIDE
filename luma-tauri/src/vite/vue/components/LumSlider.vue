<script setup lang="ts">
/**
 * LumSlider — range slider (.lum-slider) with v-model.
 * Auto-fills the colored portion via --lum-slider-progress (no JS needed at
 * call sites). `ticks` draws tick marks.
 */
defineOptions({ inheritAttrs: false, name: "LumSlider" });

const props = withDefaults(
  defineProps<{
    min?: number;
    max?: number;
    step?: number;
    ticks?: boolean;
  }>(),
  { min: 0, max: 100, step: 1 },
);

const model = defineModel<number>({ default: 0 });
</script>

<template>
  <input
    v-model.number="model"
    type="range"
    class="lum-slider"
    :min="min"
    :max="max"
    :step="step"
    :data-ticks="ticks || undefined"
    :style="{ '--lum-slider-progress': `${((model - min) / (max - min)) * 100}%` }"
    v-bind="$attrs"
  />
</template>
