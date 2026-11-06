<script setup lang="ts">
/** LumTextarea — multi-line input (.lum-textarea) with v-model. Rows/cols/etc. via attributes. */
defineOptions({ inheritAttrs: false, name: "LumTextarea" });

const model = defineModel<string>({ default: "" });
</script>

<template>
  <textarea v-model="model" class="lum-textarea" rows="3" v-bind="$attrs"></textarea>
</template>
