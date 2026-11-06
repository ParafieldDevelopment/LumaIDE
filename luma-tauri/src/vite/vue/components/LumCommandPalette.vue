<script setup lang="ts">
/**
 * LumCommandPalette — IDE command box (.lum-command). v-model:open controls it.
 * Filters `commands` as you type; Enter runs the top match; click runs any.
 */
import { computed, ref } from "vue";

defineOptions({ name: "LumCommandPalette" });

const props = withDefaults(
  defineProps<{
    commands?: Array<{ id: number | string; label: string; shortcut?: string }>;
  }>(),
  { commands: () => [] },
);

const open = defineModel<boolean>("open", { default: false });

const emit = defineEmits<{ run: [id: number | string] }>();

const search = ref("");

const filtered = computed(() => {
  const q = search.value.toLowerCase();
  return props.commands.filter((c) => c.label.toLowerCase().includes(q));
});

function run(cmd: { id: number | string }) {
  emit("run", cmd.id);
  open.value = false;
  search.value = "";
}
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="lum-overlay" @click.self="open = false">
      <div class="lum-command">
        <input
          v-model="search"
          type="text"
          placeholder="Type a command…"
          @keydown.enter="filtered[0] && run(filtered[0])"
        />
        <div class="lum-command-list">
          <button v-for="cmd in filtered" :key="cmd.id" class="lum-list-item" @click="run(cmd)">
            <span class="lum-list-item-label">{{ cmd.label }}</span>
            <span v-if="cmd.shortcut" class="lum-list-item-meta">{{ cmd.shortcut }}</span>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
