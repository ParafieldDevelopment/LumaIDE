<script setup lang="ts">
/** LumSelect — themed select (.lum-select). Pass `options` or fill the slot manually. */
defineOptions({ inheritAttrs: false, name: "LumSelect" });

interface Option {
  value: string;
  label: string;
  disabled?: boolean;
}

withDefaults(defineProps<{ options?: Option[]; size?: "sm" | "md" | "lg" }>(), { size: "md" });

const model = defineModel<string>({ default: "" });
</script>

<template>
  <div class="lum-select-wrapper">
    <select v-model="model" class="lum-select" :class="size !== 'md' ? `lum-select-${size}` : undefined" v-bind="$attrs">
      <slot>
        <option v-for="o in options" :key="o.value" :value="o.value" :disabled="o.disabled">{{ o.label }}</option>
      </slot>
    </select>
  </div>
</template>
