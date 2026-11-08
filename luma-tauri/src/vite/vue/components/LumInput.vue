<script setup lang="ts">
/**
 * LumInput — text input (.lum-input). v-model supported.
 * Use the `icon` slot to add a leading icon (rounded like .lum-search).
 * `invalid` sets the danger border.
 */
defineOptions({ inheritAttrs: false, name: "LumInput" });

withDefaults(
  defineProps<{
    invalid?: boolean;
    size?: "sm" | "md" | "lg";
    placeholder?: string;
  }>(),
  { size: "md" },
);

const model = defineModel<string>({ default: "" });
</script>

<template>
  <div v-if="$slots.icon" class="lum-input-group" data-icon="start">
    <slot name="icon" />
    <input
      v-model="model"
      type="text"
      class="lum-input"
      :class="size !== 'md' ? `lum-input-${size}` : undefined"
      :aria-invalid="invalid || undefined"
      :placeholder="placeholder"
      v-bind="$attrs"
    />
  </div>
  <input
    v-else
    v-model="model"
    type="text"
    class="lum-input"
    :class="size !== 'md' ? `lum-input-${size}` : undefined"
    :aria-invalid="invalid || undefined"
    :placeholder="placeholder"
    v-bind="$attrs"
  />
</template>
