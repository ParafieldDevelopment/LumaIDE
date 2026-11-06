<script setup lang="ts">
/** LumToasts — notification stack (.lum-toast-stack). Mount once, call `toast()` anywhere. */
import { useToasts } from "./toasts";

defineOptions({ name: "LumToasts" });

const { toasts, dismiss } = useToasts();
</script>

<template>
  <Teleport to="body">
    <div class="lum-toast-stack">
      <div v-for="t in toasts" :key="t.id" class="lum-toast" :class="t.kind" role="status">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" /></svg>
        <div class="lum-toast-body">
          <div class="lum-toast-title">{{ t.title }}</div>
          <div>{{ t.body }}</div>
        </div>
        <button class="lum-icon-btn lum-toast-close" aria-label="Dismiss" @click="dismiss(t.id)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>
    </div>
  </Teleport>
</template>
