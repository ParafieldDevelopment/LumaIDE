
<script setup lang="ts">
/**
 * JetBrains-style vertical tool-window stripe with a top and a bottom button group.
 * Dragging is pointer-based (not HTML5 drag-and-drop, which WebView2 can swallow).
 * The parent runs the drag; this component only reports pointer-down on a button and
 * renders the drop highlight / insertion marker it is told about.
 */
import { computed } from "vue";
import type { ToolPosition, ToolWindowDef } from "~/vue/composables/useToolWindows";

const props = withDefaults(
  defineProps<{
    side: "left" | "right";
    top: ToolWindowDef[];
    bottom?: ToolWindowDef[];
    activeIds: string[];
    dropTarget?: ToolPosition | null;
    /** Insertion index inside `dropTarget`, counted without the dragged button. */
    dropIndex?: number;
    draggingId?: string | null;
    labels?: boolean;
    draggable?: boolean;
  }>(),
  {
    bottom: () => [],
    dropTarget: null,
    dropIndex: 0,
    draggingId: null,
    labels: false,
    draggable: true,
  },
);

const emit = defineEmits<{
  (event: "toggle", id: string): void;
  (event: "toolPointerDown", id: string, pointerEvent: PointerEvent): void;
}>();

const sections = computed(() => [
  { position: `${props.side}-top` as ToolPosition, kind: "top", tools: props.top },
  { position: `${props.side}-bottom` as ToolPosition, kind: "bottom", tools: props.bottom },
]);

function tip(tool: ToolWindowDef) {
  return tool.hotkey ? `${tool.title} (${tool.hotkey})` : tool.title;
}

/** Where to draw the insertion line for a button: "before", "after" or none. */
function marker(position: ToolPosition, tools: ToolWindowDef[], tool: ToolWindowDef) {
  if (props.dropTarget !== position || !props.draggingId) return "";
  const others = tools.filter((item) => item.id !== props.draggingId);
  const index = others.findIndex((item) => item.id === tool.id);
  if (index === -1) return "";
  if (props.dropIndex === index) return "insert-before";
  if (props.dropIndex >= others.length && index === others.length - 1) return "insert-after";
  return "";
}

function onPointerDown(tool: ToolWindowDef, event: PointerEvent) {
  if (!props.draggable) return;
  emit("toolPointerDown", tool.id, event);
}
</script>

<template>
  <nav
    class="lum-stripe"
    :class="[`is-${side}`, { 'has-labels': labels, 'is-dragging': !!draggingId }]"
    :data-stripe-side="side"
    aria-label="Tool windows"
  >
    <div
      v-for="section in sections"
      :key="section.position"
      class="lum-stripe-section"
      :class="[`lum-stripe-section-${section.kind}`, { 'is-drop-target': dropTarget === section.position }]"
      :data-drop-position="section.position"
    >
      <div class="lum-stripe-group">
        <button
          v-for="tool in section.tools"
          :key="tool.id"
          type="button"
          class="lum-stripe-btn"
          :class="[
            { active: activeIds.includes(tool.id), 'is-being-dragged': draggingId === tool.id },
            marker(section.position, section.tools, tool),
          ]"
          :data-tool-id="tool.id"
          :title="tip(tool)"
          :aria-label="tool.title"
          :aria-pressed="activeIds.includes(tool.id)"
          @pointerdown="onPointerDown(tool, $event)"
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
  position: relative;
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
  cursor: pointer;
  touch-action: none;
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

.lum-stripe-btn.is-being-dragged {
  opacity: 0.35;
}

.is-dragging .lum-stripe-btn {
  cursor: grabbing;
}

/* Insertion line shown while dragging a button over a group. */
.lum-stripe-btn.insert-before::before,
.lum-stripe-btn.insert-after::after {
  content: "";
  position: absolute;
  left: 2px;
  right: 2px;
  height: 2px;
  border-radius: 1px;
  background: var(--lum-accent);
  pointer-events: none;
}

.lum-stripe-btn.insert-before::before {
  top: -3px;
}

.lum-stripe-btn.insert-after::after {
  bottom: -3px;
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
