<script setup lang="ts">
/** LumSkeleton — shimmering placeholder (.lum-skeleton). Sizes/width via attributes or style. */
defineOptions({ inheritAttrs: false, name: "LumSkeleton" });

withDefaults(defineProps<{ shape?: "text" | "rect" | "circle" }>(), { shape: "text" });
</script>

<template>
  <span class="lum-skeleton" :class="shape" v-bind="$attrs"></span>
</template>
