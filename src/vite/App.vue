<script setup lang="ts">
import { onMounted } from 'vue'
import HelloWorld from './components/HelloWorld.vue'

onMounted(() => {
  // Tell the Rust backend the UI is actually up (used to close the splash
  // screen); a failed page load never fires this.
  ;(window as any).__TAURI__?.event.emit('luma:frontend-ready')
})
</script>

<template>
  <HelloWorld />
</template>
