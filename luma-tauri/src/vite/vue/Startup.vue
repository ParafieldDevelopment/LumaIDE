<script setup lang="ts">
import TitleBar from "./TitleBar.vue";
import {useWindow} from "~/vue/composables/useWindow.ts";

</script>

<template>
  <TitleBar contentClass="startup-title">
    <img src="/public/logo.svg" v-if="!useWindow().state.isMacOS">
    <p>Welcome to Luma</p>
  </TitleBar>
</template>

<style scoped lang="scss">
  .startup-title {
    display: flex;
    gap: 10px;
  }

  img {
    height: auto;
    width: 20px;
  }
</style>