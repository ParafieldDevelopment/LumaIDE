<script setup lang="ts">
/**
 * LumFileTabs — editor file tabs (.lum-filetabs). v-model is the active file
 * id; emit `close` when a tab's × is clicked. `dirty` shows the unsaved dot.
 */
defineOptions({ inheritAttrs: false, name: "LumFileTabs" });

withDefaults(
  defineProps<{
    files: Array<{ id: number; name: string; dirty?: boolean }>;
  }>(),
  {},
);

const model = defineModel<number>({ default: 0 });

defineEmits<{ close: [id: number] }>();
</script>

<template>
  <div class="lum-filetabs" v-bind="$attrs">
    <button
      v-for="file in files"
      :key="file.id"
      class="lum-filetab"
      :aria-selected="model === file.id"
      :data-dirty="file.dirty || undefined"
      @click="model = file.id"
    >
      <span class="lum-filetab-name">{{ file.name }}</span>
      <span class="lum-filetab-close" role="button" :aria-label="`Close ${file.name}`" @click.stop="$emit('close', file.id)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12"><path d="M18 6 6 18M6 6l12 12" /></svg>
      </span>
    </button>
    <div style="flex: 1"></div>
    <slot name="extra" />
  </div>
</template>
