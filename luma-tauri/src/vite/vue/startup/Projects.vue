<script setup lang="ts">

import {LumButton, LumInput} from "~/vue/components";
</script>

<template>
  <div class="container">
    <div class="header">
      <LumInput placeholder="Search Projects">
        <template #icon>
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-search">
            <circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          </svg>
        </template>
      </LumInput>
      <div class="buttons">
        <LumButton>New Project</LumButton>
        <LumButton>Open</LumButton>
        <LumButton>Clone Repository</LumButton>
      </div>
    </div>
    <div class="projects">
    </div>
  </div>
</template>

<style scoped>
  .container {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .header {
    display: flex;
    flex-direction: column;
    gap: 1rem;

    border-bottom: 1px solid var(--lum-border);
    padding: 1rem;

    div.buttons {
      display: flex;
      gap: 0.8rem;
    }
  }

  .projects {
    padding: 1rem;
    overflow-y: auto;
  }
</style>