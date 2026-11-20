<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import splashUrl from "@splash/LumaSplash.png";

withDefaults(
    defineProps<{
      /** Fades the splash out when true. */
      leaving?: boolean;
      /** Fade-out duration (ms). */
      fadeOutMs?: number;
    }>(),
    {
      leaving: false,
      fadeOutMs: 400,
    }
);

/** Fired once the splash image has loaded (or failed / timed out). */
const emit = defineEmits<{ (e: "ready"): void }>();

let announced = false;
let fallbackTimer = 0;

function announce() {
  if (announced) return;
  announced = true;
  emit("ready");
}

onMounted(() => {
  // Safety net: never leave the splash window hidden forever.
  fallbackTimer = window.setTimeout(announce, 2000);
});

onBeforeUnmount(() => clearTimeout(fallbackTimer));
</script>

<template>
  <div
      class="splash"
      :class="{ leaving }"
      :style="{ '--splash-fade': `${fadeOutMs}ms` }"
  >
    <img
        class="art"
        :src="splashUrl"
        alt="Luma"
        draggable="false"
        decoding="sync"
        @load="announce"
        @error="announce"
    />
  </div>
</template>

<style scoped lang="scss">
.splash {
  position: fixed;
  inset: 0;
  background: #0f081a; // matches the art's edge color while it loads
  overflow: hidden;
  user-select: none;
  transition: opacity var(--splash-fade, 400ms) ease;

  &.leaving {
    opacity: 0;
  }
}

// Art is 2550x1650 (aspect 1.545), the window is 850x550 (same aspect),
// so "cover" fills it exactly with no cropping.
.art {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  pointer-events: none;
}
</style>
