<script setup lang="ts">
import { useToasts } from "~/vue/components/toasts";

const { toasts, dismiss } = useToasts();
</script>

<template>
  <div class="lum-toast-stack">
    <TransitionGroup
        name="lum-toast"
        tag="div"
        class="lum-toast-group"
        appear

        :duration="200"
    >
      <div
          v-for="t in toasts"
          :key="t.id"
          class="lum-toast"
          :class="t.kind"
          role="status"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
        </svg>

        <div class="lum-toast-body">
          <div class="lum-toast-title">{{ t.title }}</div>
          <div>{{ t.body }}</div>
        </div>

        <button
            class="lum-icon-btn lum-toast-close"
            aria-label="Dismiss"
            @click.stop="dismiss(t.id)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
