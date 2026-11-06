<script setup lang="ts">
/** LumPagination — page selector (.lum-pagination). v-model is the current page (1–${pages}). */
defineOptions({ inheritAttrs: false, name: "LumPagination" });

withDefaults(defineProps<{ pages: number }>(), {});

const model = defineModel<number>({ default: 1 });
</script>

<template>
  <div class="lum-pagination" v-bind="$attrs">
    <button class="lum-page-btn" :disabled="model <= 1" aria-label="Previous page" @click="model--">&#8249;</button>
    <button
      v-for="p in pages"
      :key="p"
      class="lum-page-btn"
      :aria-current="model === p ? 'true' : undefined"
      :aria-label="`Page ${p}`"
      @click="model = p"
    >{{ p }}</button>
    <button class="lum-page-btn" :disabled="model >= pages" aria-label="Next page" @click="model++">&#8250;</button>
  </div>
</template>
