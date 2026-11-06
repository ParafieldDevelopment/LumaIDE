<script setup lang="ts">
/** LumSwitch — toggle switch (.lum-switch) with v-model. Wrap in a label for a text caption. */
defineOptions({ inheritAttrs: false, name: "LumSwitch" });

const model = defineModel<boolean>({ default: false });
</script>

<template>
  <label class="lum-switch" v-bind="$attrs">
    <input v-model="model" type="checkbox" />
    <span class="lum-switch-track"></span>
  </label>
</template>
