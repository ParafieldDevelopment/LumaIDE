<script setup lang="ts">
/** LumNumber — number stepper (.lum-number) with − / + buttons and v-model. */
defineOptions({ inheritAttrs: false, name: "LumNumber" });

const props = withDefaults(
  defineProps<{
    min?: number;
    max?: number;
    step?: number;
  }>(),
  { step: 1 },
);

const model = defineModel<number>({ default: 0 });

function adjust(dir: number) {
  let next = (model.value ?? 0) + dir * props.step;
  if (props.min !== undefined) next = Math.max(props.min, next);
  if (props.max !== undefined) next = Math.min(props.max, next);
  model.value = next;
}
</script>

<template>
  <div class="lum-number">
    <button class="lum-number-btn" type="button" :disabled="min !== undefined && model <= min" @click="adjust(-1)">−</button>
    <input v-model.number="model" type="number" class="lum-input" readonly tabindex="-1" v-bind="$attrs" />
    <button class="lum-number-btn" type="button" :disabled="max !== undefined && model >= max" @click="adjust(1)">+</button>
  </div>
</template>
