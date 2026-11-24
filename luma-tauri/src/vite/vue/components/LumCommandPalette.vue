<script setup lang="ts">
/**
 * LumCommandPalette — the IDE-wide command box.
 * With no `commands` prop it lists every available action from the central
 * registry; a custom list still works for design-system demos.
 */
import { computed, nextTick, ref, watch } from "vue";
import {
  getActionContext,
  isActionAvailable,
  listActions,
  runAction,
  scoreAction,
  type Action,
} from "~/vue/composables/actions";

defineOptions({ name: "LumCommandPalette" });

const props = withDefaults(
  defineProps<{
    commands?: Array<{ id: number | string; label: string; shortcut?: string; category?: string }>;
  }>(),
  { commands: () => [] },
);

const open = defineModel<boolean>("open", { default: false });
const emit = defineEmits<{ run: [id: number | string] }>();

const search = ref("");
const input = ref<HTMLInputElement>();
const activeIndex = ref(0);

interface PaletteCommand {
  id: string | number;
  label: string;
  shortcut?: string;
  category?: string;
  keywords?: string[];
  icon?: string;
  action?: Action;
}

const candidates = computed<PaletteCommand[]>(() => {
  if (props.commands.length) {
    return props.commands.map((command) => ({ ...command, action: undefined }));
  }
  const context = getActionContext();
  return listActions()
    .filter((action) => isActionAvailable(action, context))
    .map((action) => ({
      id: action.id,
      label: action.label,
      shortcut: action.shortcut,
      category: action.category,
      keywords: action.keywords,
      icon: action.icon,
      action,
    }));
});

const preferredCategories = ["File", "Editor", "View", "Explorer", "Tools", "Project", "Git"];

const groups = computed<Array<{ label: string; commands: PaletteCommand[] }>>(() => {
  const query = search.value;
  const scored = candidates.value
    .map((command) => ({ command, score: scoreAction(command, query) }))
    .filter((entry): entry is { command: PaletteCommand; score: number } => entry.score !== null)
    .sort((left, right) => right.score - left.score || left.command.label.localeCompare(right.command.label));

  const byCategory = new Map<string, PaletteCommand[]>();
  for (const { command } of scored) {
    const category = command.category || "Commands";
    const list = byCategory.get(category) ?? [];
    list.push(command);
    byCategory.set(category, list);
  }

  return [...byCategory.entries()]
    .sort(([left], [right]) => {
      const leftIndex = preferredCategories.indexOf(left);
      const rightIndex = preferredCategories.indexOf(right);
      if (leftIndex >= 0 || rightIndex >= 0) {
        return (leftIndex < 0 ? 999 : leftIndex) - (rightIndex < 0 ? 999 : rightIndex);
      }
      if (left === "Commands") return 1;
      if (right === "Commands") return -1;
      return left.localeCompare(right);
    })
    .map(([label, commands]) => ({ label, commands }));
});

type FlatEntry = { kind: "label"; label: string } | { kind: "command"; command: PaletteCommand; index: number };

const flat = computed<FlatEntry[]>(() => {
  const entries: FlatEntry[] = [];
  let index = 0;
  for (const group of groups.value) {
    entries.push({ kind: "label", label: group.label });
    for (const command of group.commands) entries.push({ kind: "command", command, index: index++ });
  }
  return entries;
});

const resultCount = computed(() => flat.value.filter((entry) => entry.kind === "command").length);

watch(flat, () => {
  activeIndex.value = 0;
});

watch(open, async (isOpen) => {
  if (!isOpen) return;
  search.value = "";
  activeIndex.value = 0;
  await nextTick();
  input.value?.focus();
});

watch(activeIndex, async (index) => {
  await nextTick();
  input.value?.closest(".lum-command")?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
});

function run(command: PaletteCommand) {
  if (command.action) void runAction(command.action.id);
  else emit("run", command.id);
  open.value = false;
  search.value = "";
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (!resultCount.value) return;
    const step = event.key === "ArrowDown" ? 1 : -1;
    activeIndex.value = (activeIndex.value + step + resultCount.value) % resultCount.value;
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    const entry = flat.value.find((item) => item.kind === "command" && item.index === activeIndex.value);
    if (entry?.kind === "command") run(entry.command);
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    open.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="lum-overlay lum-command-overlay" @click.self="open = false">
      <div class="lum-command">
        <input
          ref="input"
          v-model="search"
          type="text"
          placeholder="Type a command or search…"
          spellcheck="false"
          autocomplete="off"
          @keydown="onKeydown"
        />
        <div class="lum-command-list">
          <template v-for="entry in flat" :key="entry.kind === 'label' ? `label-${entry.label}` : `cmd-${entry.command.id}`">
            <div v-if="entry.kind === 'label'" class="lum-command-group-label">{{ entry.label }}</div>
            <button
              v-else
              class="lum-list-item lum-command-item"
              :data-index="entry.index"
              :aria-selected="entry.index === activeIndex || undefined"
              @mouseenter="activeIndex = entry.index"
              @click="run(entry.command)"
            >
              <svg v-if="entry.command.icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
                <path :d="entry.command.icon" />
              </svg>
              <span class="lum-list-item-label">{{ entry.command.label }}</span>
              <span v-if="entry.command.shortcut" class="lum-list-item-meta">{{ entry.command.shortcut }}</span>
            </button>
          </template>
          <div v-if="!resultCount" class="lum-command-empty">No matching commands</div>
        </div>
        <div class="lum-command-footer">
          <span><kbd>↑</kbd><kbd>↓</kbd> navigate</span>
          <span><kbd>↵</kbd> run</span>
          <span><kbd>esc</kbd> close</span>
        </div>
      </div>
    </div>
  </Teleport>
</template>
