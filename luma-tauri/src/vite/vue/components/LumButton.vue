<script setup lang="ts">
/**
 * LumButton — themed button (styles: .lum-btn, .lum-btn-*)
 * variant: default/secondary look. sizes: sm/md/lg.
 */
import { computed } from "vue";

defineOptions({ inheritAttrs: false, name: "LumButton" });

const props = withDefaults(
  defineProps<{
    variant?: "default" | "primary" | "danger" | "ghost" | "link";
    size?: "sm" | "md" | "lg";
    block?: boolean;
    icon?: boolean;
    href?: string;
    loading?: boolean;
    disabled?: boolean;
  }>(),
  { variant: "default", size: "md" },
);

const classes = computed(() => [
  "lum-btn",
  props.variant !== "default" && `lum-btn-${props.variant}`,
  props.size !== "md" && `lum-btn-${props.size}`,
  props.block && "lum-btn-block",
  props.icon && "lum-btn-icon",
]);
</script>

<template>
  <a v-if="href" :href="href" :class="classes" v-bind="$attrs"><slot /></a>
  <button v-else :class="classes" :disabled="disabled" :data-loading="loading || undefined" v-bind="$attrs"><slot /></button>
</template>
