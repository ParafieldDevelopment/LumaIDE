<script setup lang="ts">
/** LumAvatar — identity bubble (.lum-avatar). size, optional status dot, square variant. Slot holds img or initials. */
defineOptions({ name: "LumAvatar" });

withDefaults(
  defineProps<{
    size?: "sm" | "md" | "lg" | "xl";
    status?: "online" | "idle" | "busy";
    square?: boolean;
  }>(),
  { size: "md" },
);
</script>

<template>
  <span class="lum-avatar" :class="[size !== 'md' ? size : undefined, square ? 'square' : undefined]" :data-status="status" v-bind="$attrs">
    <slot />
  </span>
</template>
