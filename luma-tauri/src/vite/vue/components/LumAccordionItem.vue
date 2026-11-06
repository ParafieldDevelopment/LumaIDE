<script setup lang="ts">
/** LumAccordionItem — single collapsible section (.lum-accordion-item). v-model tracks open state. */
defineOptions({ name: "LumAccordionItem" });

defineProps<{ title: string }>();

const model = defineModel<boolean>({ default: false });
</script>

<template>
  <div class="lum-accordion-item" :class="{ open: model }" v-bind="$attrs">
    <button class="lum-accordion-header" @click="model = !model">{{ title }}</button>
    <div v-show="model" class="lum-accordion-body">
      <slot />
    </div>
  </div>
</template>
