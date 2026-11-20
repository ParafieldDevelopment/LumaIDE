<script setup lang="ts">
/**
 * LumToolStripe — the JetBrains-style button strip along a window edge.
 * `top` buttons sit at the top, `bottom` buttons are pinned to the bottom
 * (new-UI style: bottom-docked tools like Terminal live here).
 * Set `labels` for classic rotated text buttons instead of icon-only.
 */
import type { ToolWindowDef } from "~/vue/composables/useToolWindows";

withDefaults(
    defineProps<{
      side: "left" | "right";
      top: ToolWindowDef[];
      bottom?: ToolWindowDef[];
      activeIds: string[];
      labels?: boolean;
    }>(),
    {
      bottom: () => [],
      labels: false,
    }
);

const emit = defineEmits<{ (e: "toggle", id: string): void }>();

function tip(tool: ToolWindowDef) {
  return tool.hotkey ? `${tool.title} (${tool.hotkey})` : tool.title;
}
</script>

<template>
  <nav class="lum-stripe" :class="[`is-${side}`, { 'has-labels': labels }]" aria-label="Tool windows">
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
          @click="emit('toggle', tool.id)"
      >
        <component :is="tool.icon" />
        <span v-if="labels" class="lum-stripe-label">{{ tool.title }}</span>
      </button>
    </div>

    <div class="lum-stripe-spacer" />

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
          @click="emit('toggle', tool.id)"
      >
        <component :is="tool.icon" />
        <span v-if="labels" class="lum-stripe-label">{{ tool.title }}</span>
      </button>
    </div>
  </nav>
</template>

<style scoped>
.lum-stripe {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
  width: 40px;
  padding: 6px 4px;
  gap: 4px;
  background: var(--lum-bg-surface);
  user-select: none;
}

.lum-stripe.is-left {
  border-right: 1px solid var(--lum-border);
}

.lum-stripe.is-right {
  border-left: 1px solid var(--lum-border);
}

.lum-stripe-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.lum-stripe-spacer {
  flex: 1 1 auto;
}

.lum-stripe-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  border-radius: var(--lum-radius-sm);
  background: transparent;
  color: var(--lum-fg-muted);
  cursor: pointer;
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

/* Classic labelled (rotated) buttons */
.has-labels .lum-stripe-btn {
  height: auto;
  padding: 8px 0;
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
