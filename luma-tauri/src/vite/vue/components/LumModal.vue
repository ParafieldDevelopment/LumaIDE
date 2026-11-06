<script setup lang="ts">
/**
 * LumModal — dialog (.lum-overlay + .lum-modal). v-model:open controls it.
 * Closes on Esc / backdrop click (unless `persistent`). Teleported to body.
 */
import { onBeforeUnmount, watch } from "vue";

defineOptions({ inheritAttrs: false, name: "LumModal" });

withDefaults(
  defineProps<{
    size?: "sm" | "md" | "lg" | "xl";
    persistent?: boolean;
  }>(),
  { size: "md" },
);

const open = defineModel<boolean>("open", { default: false });

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") open.value = false;
}

watch(open, (v) => {
  if (v) document.addEventListener("keydown", onKey);
  else document.removeEventListener("keydown", onKey);
});

onBeforeUnmount(() => document.removeEventListener("keydown", onKey));
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="lum-overlay" @click.self="!persistent && (open = false)">
      <div class="lum-modal" :class="size !== 'md' ? `lum-${size}` : undefined" role="dialog" aria-modal="true" v-bind="$attrs">
        <div class="lum-modal-header">
          <span class="lum-modal-title"><slot name="title" /></span>
          <button class="lum-icon-btn" aria-label="Close" @click="open = false">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18M6 6l12 12" /></svg>
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
  </Teleport>
</template>
