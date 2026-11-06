<script setup lang="ts">
/** LumTabs — underline style tab bar (.lum-tabs). v-model is the active index. */
defineOptions({ inheritAttrs: false, name: "LumTabs" });

withDefaults(
  defineProps<{
    items: Array<{ label: string; disabled?: boolean }>;
  }>(),
  {},
);

const model = defineModel<number>({ default: 0 });
</script>

<template>
  <div class="lum-tabs" role="tablist" v-bind="$attrs">
    <button
      v-for="(tab, i) in items"
      :key="tab.label"
      class="lum-tab"
      role="tab"
      :aria-selected="model === i"
      :disabled="tab.disabled"
      @click="model = i"
    >{{ tab.label }}</button>
  </div>
</template>
