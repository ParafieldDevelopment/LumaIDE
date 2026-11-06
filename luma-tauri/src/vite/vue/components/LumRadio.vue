<script setup lang="ts">
/** LumRadio — radio dot (.lum-radio). Share `v-model` across a group, each with a distinct `value`. */
defineOptions({ inheritAttrs: false, name: "LumRadio" });

withDefaults(defineProps<{ value: string | number; disabled?: boolean }>(), {});

const model = defineModel<string | number>({ default: "" });
</script>

<template>
  <input
    v-model="model"
    type="radio"
    class="lum-radio"
    :value="value"
    :disabled="disabled"
    v-bind="$attrs"
  />
</template>
