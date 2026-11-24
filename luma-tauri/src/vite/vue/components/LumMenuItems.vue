<script setup lang="ts">
/** Recursive menu body: renders actions, separators and submenus. */
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import LumMenu from "./LumMenu.vue";
import LumMenuItem from "./LumMenuItem.vue";
import type { MenuItem } from "~/vue/composables/menus";

defineOptions({ name: "LumMenuItems" });

const props = withDefaults(
  defineProps<{ items: MenuItem[]; level?: number; active?: boolean }>(),
  { level: 0, active: true },
);
const emit = defineEmits<{ close: [] }>();

const container = ref<HTMLElement | null>(null);
const submenuElement = ref<HTMLElement | null>(null);
const submenu = ref<{ items: MenuItem[]; left: number; top: number } | null>(null);
const activeIndex = ref(-1);

const actionable = computed(() =>
  props.items
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => !item.separator && !item.disabled),
);
const activeItemIndex = computed(() =>
  activeIndex.value >= 0 ? actionable.value[activeIndex.value]?.index ?? -1 : -1,
);
const handlesKeyboard = computed(() => props.active && !submenu.value);

function clearSubmenu() {
  submenu.value = null;
}

function openSubmenu(item: MenuItem, index: number) {
  if (!item.children?.length) return;
  const element = container.value?.querySelector<HTMLElement>(`[data-index="${index}"]`);
  if (!element) return;
  const rect = element.getBoundingClientRect();
  submenu.value = { items: item.children, left: rect.right - 4, top: rect.top - 4 };
  void nextTick(() => {
    const popup = submenuElement.value;
    if (!submenu.value || !popup) return;
    submenu.value.left = Math.min(submenu.value.left, window.innerWidth - popup.offsetWidth - 8);
    submenu.value.top = Math.min(submenu.value.top, window.innerHeight - popup.offsetHeight - 8);
  });
}

function onItemHover(item: MenuItem, index: number) {
  const nextActive = actionable.value.findIndex((entry) => entry.index === index);
  if (nextActive === activeIndex.value) return;
  activeIndex.value = nextActive;
  if (item.children?.length) openSubmenu(item, index);
  else clearSubmenu();
}

async function activate(item: MenuItem, index: number) {
  if (item.disabled || item.separator) return;
  if (item.children?.length) {
    openSubmenu(item, index);
    return;
  }
  emit("close");
  await item.action?.();
}

function onKeydown(event: KeyboardEvent) {
  if (!handlesKeyboard.value) return;
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
    return;
  }
  const count = actionable.value.length;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (!count) return;
    const step = event.key === "ArrowDown" ? 1 : -1;
    activeIndex.value = activeIndex.value < 0
      ? (step > 0 ? 0 : count - 1)
      : (activeIndex.value + step + count) % count;
    clearSubmenu();
    return;
  }
  const entry = activeIndex.value >= 0 ? actionable.value[activeIndex.value] : undefined;
  if (event.key === "ArrowRight" && entry?.item.children?.length) {
    event.preventDefault();
    openSubmenu(entry.item, entry.index);
    return;
  }
  if (event.key === "Enter" && entry) {
    event.preventDefault();
    void activate(entry.item, entry.index);
  }
}

document.addEventListener("keydown", onKeydown);
onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div ref="container" class="lum-menu-items">
    <template v-for="(item, index) in items" :key="index">
      <div v-if="item.separator" class="lum-menu-separator" />
      <LumMenuItem
        v-else
        :data-index="index"
        :danger="item.danger"
        :disabled="item.disabled"
        :shortcut="item.shortcut"
        :aria-selected="index === activeItemIndex || undefined"
        @click="activate(item, index)"
        @mousemove="onItemHover(item, index)"
      >
        <template v-if="item.checked || item.icon" #icon>
          <span class="lum-menu-check">{{ item.checked ? "✓" : "" }}</span>
          <svg v-if="item.icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
            <path :d="item.icon" />
          </svg>
        </template>
        {{ item.label }}
        <span v-if="item.children?.length" class="lum-submenu-arrow">›</span>
      </LumMenuItem>
    </template>

    <div
      v-if="submenu"
      ref="submenuElement"
      class="lum-context-menu lum-submenu"
      :style="{ left: `${submenu.left}px`, top: `${submenu.top}px` }"
    >
      <LumMenu>
        <LumMenuItems :items="submenu.items" :level="level + 1" @close="emit('close')" />
      </LumMenu>
    </div>
  </div>
</template>

<style scoped>
.lum-menu-items { position: relative; }

.lum-context-menu {
  position: fixed;
  z-index: calc(var(--lum-z-popover) + 1);
}

.lum-context-menu :deep(.lum-menu) {
  min-width: 180px;
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

.lum-context-menu :deep(.lum-menu-separator) { margin: 4px 0; }

.lum-menu-check {
  display: inline-block;
  width: 12px;
  flex-shrink: 0;
  text-align: center;
}

.lum-submenu-arrow {
  margin-left: auto;
  padding-left: var(--lum-space-2);
  color: var(--lum-fg-subtle);
}
</style>
