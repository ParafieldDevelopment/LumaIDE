<script setup lang="ts">
/**
 * LumModal — dialog (.lum-overlay + .lum-modal). v-model:open controls it.
 * Closes on Esc and optionally on backdrop click. Rendered in place below the titlebar.
 */
import { onBeforeUnmount, watch } from "vue";

defineOptions({ inheritAttrs: false, name: "LumModal" });

const props = withDefaults(
  defineProps<{
    size?: "sm" | "md" | "lg" | "xl";
    persistent?: boolean;
    closeOnBackdrop?: boolean;
    showCloseButton?: boolean;
  }>(),
  { size: "md", closeOnBackdrop: false, showCloseButton: true },
);

const open = defineModel<boolean>("open", { default: false });

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && !props.persistent) open.value = false;
}

watch(open, (v) => {
  if (v) document.addEventListener("keydown", onKey);
  else document.removeEventListener("keydown", onKey);
}, { immediate: true });

onBeforeUnmount(() => document.removeEventListener("keydown", onKey));
</script>

<template>
  <Transition name="lum-modal" appear>
    <div
        v-if="open"
        class="lum-overlay"
        @click.self="!props.persistent && props.closeOnBackdrop && (open = false)"
    >
      <div
          class="lum-modal"
          :class="props.size !== 'md' ? `lum-${props.size}` : undefined"
          role="dialog"
          aria-modal="true"
          v-bind="$attrs"
      >
        <div v-if="$slots.title || props.showCloseButton" class="lum-modal-header">
          <span v-if="$slots.title" class="lum-modal-title">
            <slot name="title" />
          </span>

          <button
              v-if="props.showCloseButton"
              class="lum-icon-btn"
              aria-label="Close"
              @click="open = false"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="lum-modal-body">
          <slot />
        </div>

        <div v-if="$slots.footer" class="lum-modal-footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Transition>
</template>