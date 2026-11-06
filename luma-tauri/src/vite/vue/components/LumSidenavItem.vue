<script setup lang="ts">
/** LumSidenavItem — nav entry (.lum-sidenav-item). `active` highlights it; `badge` shows a counter. */
defineOptions({ name: "LumSidenavItem" });

withDefaults(defineProps<{ active?: boolean; badge?: string | number }>(), { active: false });

defineEmits<{ select: [] }>();
</script>

<template>
  <button class="lum-sidenav-item" :class="{ active }" @click="$emit('select')">
    <slot />
    <span v-if="badge !== undefined && badge !== ''" class="lum-badge">{{ badge }}</span>
  </button>
</template>
