<script setup lang="ts">
/** LumMenuItem — menu action (.lum-menu-item). `shortcut` right-aligns a key hint. */
defineOptions({ inheritAttrs: false, name: "LumMenuItem" });

withDefaults(
  defineProps<{
    danger?: boolean;
    disabled?: boolean;
    shortcut?: string;
  }>(),
  {},
);

defineEmits<{ click: [] }>();
</script>

<template>
  <button class="lum-menu-item" :class="{ danger }" :disabled="disabled" v-bind="$attrs" @click="!disabled && $emit('click')">
    <slot name="icon" />
    <slot />
    <span v-if="shortcut" class="lum-menu-shortcut">{{ shortcut }}</span>
  </button>
</template>
