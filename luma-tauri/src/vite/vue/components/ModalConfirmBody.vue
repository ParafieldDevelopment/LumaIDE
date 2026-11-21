<script setup lang="ts">
/** Body of a JetBrains-style confirm dialog: "?" badge, message and an optional checkbox. */
import { nextTick, onMounted } from "vue";
import LumCheckbox from "./LumCheckbox.vue";

defineProps<{
  message: string;
  checkboxLabel?: string;
  state: { dontAsk: boolean };
}>();

// Focus the first (default) footer button so Enter confirms.
onMounted(async () => {
  await nextTick();
  document.querySelector<HTMLButtonElement>(".lum-modal-footer button")?.focus();
});
</script>

<template>
  <div class="confirm-body">
    <span class="confirm-icon" aria-hidden="true">?</span>
    <div class="confirm-text">
      <p class="confirm-message">{{ message }}</p>
      <label v-if="checkboxLabel" class="confirm-check">
        <LumCheckbox v-model="state.dontAsk" />
        <span>{{ checkboxLabel }}</span>
      </label>
    </div>
  </div>
</template>

<style scoped>
.confirm-body {
  display: flex;
  align-items: flex-start;
  gap: var(--lum-space-4, 16px);
}

.confirm-icon {
  display: inline-flex;
  flex: 0 0 28px;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--lum-accent);
  color: #fff;
  font-weight: 700;
  font-size: 16px;
}

.confirm-text {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--lum-space-3, 12px);
  min-width: 0;
}

.confirm-message {
  margin: 0;
  color: var(--lum-fg);
  overflow-wrap: anywhere;
}

.confirm-check {
  display: inline-flex;
  align-items: center;
  gap: var(--lum-space-2, 8px);
  color: var(--lum-fg-muted);
  cursor: pointer;
  user-select: none;
}
</style>
