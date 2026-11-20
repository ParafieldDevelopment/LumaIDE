<script setup lang="ts">
import type { Workspace } from "~/luma/Workspace.ts";
import { getProjectInitials } from "~/luma/Workspace.ts";
import MoreVerticalIcon from "~/vue/icons/MoreVerticalIcon.vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { registerContextMenu, type ContextMenuItem } from "~/vue/composables/contextMenu";

const props = defineProps<{ project: Workspace }>();
const emit = defineEmits<{
  open: [project: Workspace];
  reveal: [project: Workspace];
  "copy-path": [project: Workspace];
  remove: [project: Workspace];
}>();

const projectDiv = ref<HTMLElement>();
let unregisterContextMenu: (() => void) | undefined;

function click() {
  emit("open", props.project);
}

function menuItems(): ContextMenuItem[] {
  return [
    { label: "Open", action: () => emit("open", props.project) },
    { separator: true },
    { label: "Reveal in File Manager", action: () => emit("reveal", props.project) },
    { label: "Copy Path", action: () => emit("copy-path", props.project) },
    { separator: true },
    { label: "Remove from Recent", action: () => emit("remove", props.project) },
  ];
}

// The "..." button opens the same menu, anchored under the button.
function openMenuFromButton(event: Event) {
  const target = event.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  target.dispatchEvent(
    new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      composed: true,
      clientX: rect.left,
      clientY: rect.bottom,
    }),
  );
}

onMounted(() => {
  if (projectDiv.value) unregisterContextMenu = registerContextMenu(projectDiv.value, menuItems);
});

onBeforeUnmount(() => unregisterContextMenu?.());

const iconSrc = computed(() => {
  if (!props.project.icon_path) return null;
  return convertFileSrc(props.project.icon_path);
});
</script>

<template>
  <div
      ref="projectDiv"
      class="project"
      role="button"
      tabindex="0"
      :aria-label="`Open workspace ${project.title}`"
      @click="click"
      @keydown.enter.prevent="click"
      @keydown.space.prevent="click"
  >
    <div
        class="project_icon"
        :style="{ backgroundColor: project.icon_path ? 'transparent' : project.color }"
    >
      <img v-if="iconSrc" :src="iconSrc" alt="Workspace icon" />
      <p v-else>{{ getProjectInitials(project.title) }}</p>
    </div>

    <div class="project_content">
      <h3>{{ project.title }}</h3>
      <p>{{ project.path }}</p>
      <p v-if="project.git_branch">{{ project.git_branch }}</p>
    </div>

    <div
        class="more"
        role="button"
        tabindex="0"
        title="Workspace options"
        aria-label="Workspace options"
        @click.stop="openMenuFromButton"
        @keydown.enter.stop.prevent="openMenuFromButton"
        @keydown.space.stop.prevent="openMenuFromButton"
    >
      <MoreVerticalIcon />
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
  cursor: pointer;
  transition: background-color 0.12s ease;

  &:hover,
  &:focus-visible {
    background-color: rgba(255, 255, 255, 0.1);
    outline: none;
  }

  &:active {
    background-color: transparent;
  }

  .project_icon {
    display: flex;
    flex-shrink: 0;
    height: 35px;
    width: 35px;
    border-radius: 10px;
    align-items: center;
    justify-content: center;
    font-size: var(--lum-text-lg);

    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
      border-radius: inherit;
    }
  }

  .project_content {
    margin-left: 10px;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    align-items: flex-start;
    min-width: 0;

    h3,
    p {
      margin: 0;
      overflow-wrap: anywhere;
    }

    p {
      opacity: 0.7;
      font-size: var(--lum-text-sm);
    }
  }

  &:hover .more,
  &:focus-within .more {
    display: flex;
  }

  .more {
    display: none;
    margin-left: auto;
    align-items: center;
    justify-content: center;
    width: 35px;
  }
}
</style>
