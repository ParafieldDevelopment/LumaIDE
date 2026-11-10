<script setup lang="ts">

</script>

<template>
  <div class="project">
    <div class="project_icon">

    </div>
  </div>
</template>

<style scoped lang="scss">
  div.project {
    background-color: transparent;
    border-radius: 15px;
    border: 1px solid var(--lum-border);

    padding: 10px;

    width: 100%;

    display: flex;
    align-items: center;

    & .project_icon {
      display: flex;

      height: 40px;
      width: 40px;
      border-radius: 10px;

      background-color: var(--lum-red-400);
    }
  }
</style>