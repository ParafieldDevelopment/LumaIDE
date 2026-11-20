<script setup lang="ts">
/** JetBrains-style vertical tool-window stripe with four corner drop zones. */
import type { ToolPosition, ToolWindowDef } from "~/vue/composables/useToolWindows";

const props = withDefaults(
  defineProps<{
    side: "left" | "right";
    top: ToolWindowDef[];
    bottom?: ToolWindowDef[];
    activeIds: string[];
    dropTarget?: ToolPosition | null;
    labels?: boolean;
    draggable?: boolean;
  }>(),
  {
    bottom: () => [],
    dropTarget: null,
    labels: false,
    draggable: true,
  },
);

const emit = defineEmits<{
  (event: "toggle", id: string): void;
  (event: "dragstart", id: string, dragEvent: DragEvent): void;
  (event: "dragend"): void;
  (event: "dragover", position: ToolPosition, dragEvent: DragEvent): void;
  (event: "dragleave", position: ToolPosition): void;
  (event: "drop", position: ToolPosition, dragEvent: DragEvent): void;
}>();

const topPosition = `${props.side}-top` as ToolPosition;
const bottomPosition = `${props.side}-bottom` as ToolPosition;

function tip(tool: ToolWindowDef) {
  return tool.hotkey ? `${tool.title} (${tool.hotkey})` : tool.title;
}

function dragOver(position: ToolPosition, event: DragEvent) {
  emit("dragover", position, event);
}

function dragLeave(position: ToolPosition, event: DragEvent) {
  const next = event.relatedTarget;
  if (next instanceof Node && (event.currentTarget as HTMLElement).contains(next)) return;
  emit("dragleave", position);
}
</script>

<template>
  <nav class="lum-stripe" :class="[`is-${side}`, { 'has-labels': labels }]" aria-label="Tool windows">
    <div
      class="lum-stripe-section lum-stripe-section-top"
      :class="{ 'is-drop-target': dropTarget === topPosition }"
      @dragover="dragOver(topPosition, $event)"
      @dragleave="dragLeave(topPosition, $event)"
      @drop="emit('drop', topPosition, $event)"
    >
      <div class="lum-stripe-group">
        <button
          v-for="tool in top"
          :key="tool.id"
          type="button"
          class="lum-stripe-btn"
          :class="{ active: activeIds.includes(tool.id) }"
          :title="tip(tool)"
          :aria-label="tool.title"
          :aria-pressed="activeIds.includes(tool.id)"
          :draggable="draggable"
          @dragstart="emit('dragstart', tool.id, $event)"
          @dragend="emit('dragend')"
          @click="emit('toggle', tool.id)"
        >
          <component :is="tool.icon" />
          <span v-if="labels" class="lum-stripe-label">{{ tool.title }}</span>
        </button>
      </div>
    </div>

    <div
      class="lum-stripe-section lum-stripe-section-bottom"
      :class="{ 'is-drop-target': dropTarget === bottomPosition }"
      @dragover="dragOver(bottomPosition, $event)"
      @dragleave="dragLeave(bottomPosition, $event)"
      @drop="emit('drop', bottomPosition, $event)"
    >
      <div class="lum-stripe-group">
        <button
          v-for="tool in bottom"
          :key="tool.id"
          type="button"
          class="lum-stripe-btn"
          :class="{ active: activeIds.includes(tool.id) }"
          :title="tip(tool)"
          :aria-label="tool.title"
          :aria-pressed="activeIds.includes(tool.id)"
          :draggable="draggable"
          @dragstart="emit('dragstart', tool.id, $event)"
          @dragend="emit('dragend')"
          @click="emit('toggle', tool.id)"
        >
          <component :is="tool.icon" />
          <span v-if="labels" class="lum-stripe-label">{{ tool.title }}</span>
        </button>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.lum-stripe {
  display: flex;
  flex: 0 0 40px;
  flex-direction: column;
  width: 40px;
  min-height: 0;
  padding: var(--lum-space-2) var(--lum-space-1);
  gap: 0;
  background: var(--lum-bg-surface);
  user-select: none;
}

.lum-stripe-section {
  display: flex;
  flex: 1 1 50%;
  min-height: 40px;
  padding: var(--lum-space-1) 0;
  border-radius: var(--lum-radius-sm);
  transition: background var(--lum-transition-fast), outline-color var(--lum-transition-fast);
}

.lum-stripe-section-top {
  align-items: flex-start;
}

.lum-stripe-section-bottom {
  align-items: flex-end;
}

.lum-stripe-section.is-drop-target {
  background: var(--lum-accent-muted);
  outline: 1px dashed var(--lum-accent);
  outline-offset: -2px;
}

.lum-stripe-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--lum-space-1);
  width: 100%;
}

.lum-stripe-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 0;
  border-radius: var(--lum-radius-sm);
  background: transparent;
  color: var(--lum-fg-muted);
  cursor: grab;
  transition: background var(--lum-transition-fast), color var(--lum-transition-fast);
}

.lum-stripe-btn:hover {
  background: var(--lum-bg-hover);
  color: var(--lum-fg-strong);
}

.lum-stripe-btn.active {
  background: var(--lum-accent-muted);
  color: var(--lum-fg-strong);
}

.lum-stripe-btn:focus-visible {
  outline: 1px solid var(--lum-accent);
  outline-offset: 1px;
}

.has-labels .lum-stripe-btn {
  height: auto;
  padding: var(--lum-space-2) 0;
  writing-mode: vertical-rl;
}

.has-labels.is-left .lum-stripe-btn {
  transform: rotate(180deg);
}

.lum-stripe-label {
  font-size: var(--lum-text-xs);
  white-space: nowrap;
}
</style>
