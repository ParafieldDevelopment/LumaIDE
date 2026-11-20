<script setup lang="ts">
/**
 * LumToolWindow — standard chrome for a docked panel: title, an `actions`
 * slot for header buttons, and a hide button. Put the panel body in the
 * default slot.
 */
defineProps<{ title: string }>();
const emit = defineEmits<{ (e: "hide"): void }>();
</script>

<template>
  <section class="lum-tool-window">
    <header class="lum-tool-window-header">
      <span class="lum-tool-window-title">{{ title }}</span>

      <div class="lum-tool-window-actions">
        <slot name="actions" />
        <button
            type="button"
            class="lum-tool-window-btn"
            title="Hide"
            aria-label="Hide"
            @click="emit('hide')"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </button>
      </div>
    </header>

    <div class="lum-tool-window-body">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.lum-tool-window {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  min-width: 0;
  background: var(--lum-bg-surface);
}

.lum-tool-window-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--lum-space-2);
  padding: var(--lum-space-2) var(--lum-space-3);
  border-bottom: 1px solid var(--lum-border-muted);
  font-size: var(--lum-text-sm);
  font-weight: var(--lum-font-weight-semibold);
  color: var(--lum-fg-muted);
  user-select: none;
}

.lum-tool-window-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lum-tool-window-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.lum-tool-window-body {
  flex: 1 1 auto;
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>

<style>
/* Shared by panels that put icon buttons in the `actions` slot. */
.lum-tool-window-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  border-radius: var(--lum-radius-sm);
  background: transparent;
  color: var(--lum-fg-muted);
  cursor: pointer;
  transition: background var(--lum-transition-fast), color var(--lum-transition-fast);
}

.lum-tool-window-btn:hover:not(:disabled) {
  background: var(--lum-bg-hover);
  color: var(--lum-fg-strong);
}

.lum-tool-window-btn:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
