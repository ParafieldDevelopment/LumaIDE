<script setup lang="ts">
/** LumCheckbox — custom-drawn checkbox (.lum-checkbox), supports indeterminate. */
defineOptions({ inheritAttrs: false, name: "LumCheckbox" });

withDefaults(defineProps<{ indeterminate?: boolean; disabled?: boolean }>(), {});

const model = defineModel<boolean>({ default: false });
</script>

<template>
  <input
    v-model="model"
    type="checkbox"
    class="lum-checkbox"
    :indeterminate="indeterminate"
    :disabled="disabled"
    v-bind="$attrs"
  />
</template>
