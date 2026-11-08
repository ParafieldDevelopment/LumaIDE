<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref } from "vue";
import LumMenu from "./LumMenu.vue";
import LumMenuItem from "./LumMenuItem.vue";
import { onContextMenuRequest, type ContextMenuItem } from "../composables/contextMenu";

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

function show(request: { event: MouseEvent; items: ContextMenuItem[] }) {
  items.value = request.items;
  position.value = { x: request.event.clientX, y: request.event.clientY };
  open.value = true;
  void nextTick(() => {
    const el = menu.value;
    if (!el) return;
    position.value.x = Math.min(position.value.x, window.innerWidth - el.offsetWidth - 8);
    position.value.y = Math.min(position.value.y, window.innerHeight - el.offsetHeight - 8);
  });
}

async function activate(item: ContextMenuItem) {
  if (item.disabled || item.separator) return;
  close();
  await item.action?.();
}

const removeRequestListener = onContextMenuRequest(show);
onBeforeUnmount(() => {
  removeRequestListener();
  document.removeEventListener("mousedown", onMouseDown);
  document.removeEventListener("keydown", onKeydown);
});

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") close();
}

document.addEventListener("mousedown", onMouseDown);
document.addEventListener("keydown", onKeydown);
</script>

<template>
  <Teleport to="body">
    <div v-if="open" ref="menu" class="lum-context-menu" :style="{ left: `${position.x}px`, top: `${position.y}px` }">
      <LumMenu>
        <template v-for="(item, index) in items" :key="index">
          <div v-if="item.separator" class="lum-menu-separator" />
          <LumMenuItem v-else :danger="item.danger" :disabled="item.disabled" :shortcut="item.shortcut" @click="activate(item)">
            {{ item.label }}
          </LumMenuItem>
        </template>
      </LumMenu>
    </div>
  </Teleport>
</template>

<style scoped>
.lum-context-menu { position: fixed; z-index: var(--lum-z-popover); }
</style>
