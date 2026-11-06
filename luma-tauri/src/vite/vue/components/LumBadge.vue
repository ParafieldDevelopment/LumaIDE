<script setup lang="ts">
/** LumBadge — status pill (.lum-badge). variant tinted colors; `dot` renders the round status dot. */
defineOptions({ name: "LumBadge" });

const props = withDefaults(
  defineProps<{
    variant?: "neutral" | "primary" | "soft" | "success" | "warning" | "danger" | "info";
    dot?: "online" | "idle" | "busy" | "offline";
  }>(),
  { variant: "neutral" },
);

const variantClass = () =>
  props.variant === "neutral"
    ? undefined
    : props.variant === "soft"
      ? "lum-badge-primary-soft"
      : `lum-badge-${props.variant}`;
</script>

<template>
  <span v-if="dot" class="lum-badge lum-badge-dot" :class="dot" v-bind="$attrs"></span>
  <span v-else class="lum-badge" :class="variantClass()" v-bind="$attrs"><slot /></span>
</template>
