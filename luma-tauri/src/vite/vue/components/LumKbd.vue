<script setup lang="ts">
/**
 * LumKbd — keyboard key hint (.lum-kbd).
 * Pass `keys="Ctrl+Shift+P"` to render a split shortcut, or use the default
 * slot for a single key.
 */
import { computed } from "vue";

defineOptions({ inheritAttrs: false, name: "LumKbd" });

const props = withDefaults(defineProps<{ keys?: string }>(), { keys: "" });

const keys = computed(() => props.keys.split("+").filter(Boolean));
</script>

<template>
  <template v-if="keys.length > 1">
    <kbd v-for="k in keys" :key="k" class="lum-kbd" style="margin-right: 2px">{{ k }}</kbd>
  </template>
  <kbd v-else class="lum-kbd"><slot>{{ keys[0] }}</slot></kbd>
</template>
