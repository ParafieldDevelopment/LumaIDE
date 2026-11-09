<script setup lang="ts">
/**
 * LumFileTabs — editor file tabs (.lum-filetabs). v-model is the active file
 * id; emit `close` when a tab's × is clicked. `dirty` shows the unsaved dot.
 */
defineOptions({ inheritAttrs: false, name: "LumFileTabs" });

withDefaults(
  defineProps<{
    files: Array<{ id: number; name: string; dirty?: boolean, icon?: any }>;
  }>(),
  {},
);

const model = defineModel<number>({ default: 0 });

defineEmits<{ close: [id: number] }>();
</script>

<template>
  <div class="lum-filetabs" role="tablist" v-bind="$attrs">
    <div
        v-for="file in files"
        :key="file.id"
        class="lum-filetab"
        role="tab"
        tabindex="0"
        :aria-selected="model === file.id"
        :data-active="model === file.id || undefined"
        :data-dirty="file.dirty || undefined"
        @click="model = file.id"
        @keydown.enter="model = file.id"
        @keydown.space.prevent="model = file.id"
    >
      <span class="lum-filetab-name">{{ file.name }}</span>

      <button
          class="lum-filetab-close"
          type="button"
          :aria-label="`Close ${file.name}`"
          @click.stop="$emit('close', file.id)"
      >
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            width="12"
            height="12"
        >
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="lum-filetabs-spacer"></div>
    <slot name="extra" />
  </div>
</template>
