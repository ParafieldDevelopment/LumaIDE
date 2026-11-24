<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from "vue";
import LumMenu from "./LumMenu.vue";
import LumMenuItems from "./LumMenuItems.vue";
import { onContextMenuRequest, type ContextMenuRequest, type ContextMenuItem } from "../composables/contextMenu";

const open = ref(false);
const position = ref({ x: 0, y: 0 });
const items = ref<ContextMenuItem[]>([]);
const menu = ref<HTMLElement>();

function close() {
  open.value = false;
  items.value = [];
}

function onMouseDown(event: MouseEvent) {
  if (!menu.value?.contains(event.target as Node)) close();
}

function show(request: ContextMenuRequest) {
  items.value = request.items;
  position.value = request.anchor
    ? { x: request.anchor.left, y: request.anchor.top }
    : {
        x: request.event?.clientX ?? 0,
        y: request.event?.clientY ?? 0,
      };
  open.value = true;
  void nextTick(() => {
    const el = menu.value;
    if (!el) return;
    position.value.x = Math.min(position.value.x, window.innerWidth - el.offsetWidth - 8);
    position.value.y = Math.min(position.value.y, window.innerHeight - el.offsetHeight - 8);
  });
}

const removeRequestListener = onContextMenuRequest(show);
document.addEventListener("mousedown", onMouseDown);

onBeforeUnmount(() => {
  removeRequestListener();
  document.removeEventListener("mousedown", onMouseDown);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="open" ref="menu" class="lum-context-menu" :style="{ left: `${position.x}px`, top: `${position.y}px` }">
      <LumMenu>
        <LumMenuItems :items="items" @close="close" />
      </LumMenu>
    </div>
  </Teleport>
</template>

<style scoped>
.lum-context-menu { position: fixed; z-index: var(--lum-z-popover); }

/* Flat, full-width rows (JetBrains-style) instead of rounded button-like items. */
.lum-context-menu :deep(.lum-menu) {
  min-width: 200px;
  padding: 4px 0;
  border-radius: 6px;
}

.lum-context-menu :deep(.lum-menu-item) {
  width: 100%;
  padding: 0 14px;
  border-radius: 0;
}

.lum-context-menu :deep(.lum-menu-item[aria-selected="true"]) {
  background: var(--lum-menu-item-bg-hover);
  color: var(--lum-menu-item-fg-hover);
}

.lum-context-menu :deep(.lum-menu-separator) {
  margin: 4px 0;
}
</style>
