<script setup lang="ts">
import DiscordRpcSettings from "~/vue/extensions/DiscordRpcSettings.vue";
</script>

<template>
  <div class="extensions-container">
    <div class="header">
      <div class="header-titles">
        <h2>Installed Extensions</h2>
        <p class="subtitle">Manage extensions and external integrations for LumaIDE.</p>
      </div>
    </div>

    <div class="extensions-list">
      <div class="extension-item">
        <DiscordRpcSettings />
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.extensions-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.header {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  border-bottom: 1px solid var(--lum-border);
  padding: 1.25rem 1.5rem;

  h2 {
    font-size: 18px;
    font-weight: 600;
    color: var(--lum-gray-100);
    margin: 0;
  }

  .subtitle {
    font-size: 13px;
    color: var(--lum-gray-400);
    margin: 0;
  }
}

.extensions-list {
  display: flex;
  flex-direction: column;
  padding: 1.5rem;
  overflow-y: auto;
  gap: 1.25rem;
  max-width: 720px;
}

.extension-item {
  width: 100%;
}
</style>
