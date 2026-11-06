<script setup lang="ts">
/**
 * LumCodeBlock — code surface (.lum-code-block).
 * Default slot for syntax-highlighted spans (use .kw/.str/.com/.fn classes
 * that the stylesheet defines), or pass `code` as plain text.
 */
defineOptions({ inheritAttrs: false, name: "LumCodeBlock" });

withDefaults(defineProps<{ code?: string }>(), {});
</script>

<template>
  <pre class="lum-code-block" v-bind="$attrs"><slot>{{ code }}</slot></pre>
</template>
