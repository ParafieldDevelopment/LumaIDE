<script setup lang="ts">
import { onMounted } from 'vue'

onMounted(() => {
  ;(window as any).__TAURI__?.event.emit('luma:frontend-ready')
})
</script>

<template>
  
</template>
