<script setup lang="ts">
/**
 * IdeTitlebarNav — JetBrains-style titlebar navigation widgets.
 * Displays:
 *  - Luma brand badge and main menu (hamburger)
 *  - Project widget dropdown ([Badge] ProjectName ⌄) with Recent Projects menu
 *  - Version Control / Git Branch widget dropdown (⑂ branch ⌄)
 */
import { ref, onMounted, onBeforeUnmount, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { getProjectColor, getProjectInitials } from "~/luma/Workspace.ts";
import { useModals } from "~/vue/components";
import ModalNewWorkspace from "~/vue/startup/ModalNewWorkspace.vue";
import { useToasts } from "~/vue/components/toasts";
import { askOpenMode } from "~/vue/composables/useConfirm";

interface WorkspaceInfo {
  project_uuid: string;
  title: string;
  path: string;
  git_branch?: string;
  icon_path?: string;
}

const props = defineProps<{
  activeWorkspace: WorkspaceInfo | null;
}>();

const emit = defineEmits<{
  "workspace-changed": [workspace: WorkspaceInfo];
}>();

const { toast } = useToasts();
const modals = useModals();

const recentWorkspaces = ref<WorkspaceInfo[]>([]);
const isProjectMenuOpen = ref(false);
const isVcsMenuOpen = ref(false);
const isMainMenuOpen = ref(false);

const navRef = ref<HTMLElement | null>(null);

function cleanDisplayPath(path: string): string {
  return path.replace(/^[A-Za-z]:\\Users\\[^\\]+/, "~").replace(/^\/Users\/[^/]+/, "~");
}

const projectColor = computed(() => {
  return props.activeWorkspace ? getProjectColor(props.activeWorkspace.title) : "#4f46e5";
});

const projectInitials = computed(() => {
  return props.activeWorkspace ? getProjectInitials(props.activeWorkspace.title) : "LM";
});

async function refreshRecent() {
  try {
    const list = await invoke<WorkspaceInfo[]>("get_recent_workspaces");
    recentWorkspaces.value = list;
  } catch (error) {
    console.error("Failed to load recent workspaces in titlebar:", error);
  }
}

function closeAllMenus() {
  isProjectMenuOpen.value = false;
  isVcsMenuOpen.value = false;
  isMainMenuOpen.value = false;
}

function toggleProjectMenu() {
  const next = !isProjectMenuOpen.value;
  closeAllMenus();
  if (next) {
    void refreshRecent();
    isProjectMenuOpen.value = true;
  }
}

function toggleVcsMenu() {
  const next = !isVcsMenuOpen.value;
  closeAllMenus();
  isVcsMenuOpen.value = next;
}

function toggleMainMenu() {
  const next = !isMainMenuOpen.value;
  closeAllMenus();
  isMainMenuOpen.value = next;
}

function handleDocClick(event: MouseEvent) {
  if (!navRef.value?.contains(event.target as Node)) {
    closeAllMenus();
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeAllMenus();
  }
}

onMounted(() => {
  void refreshRecent();
  document.addEventListener("mousedown", handleDocClick);
  document.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener("mousedown", handleDocClick);
  document.removeEventListener("keydown", onKeydown);
});

function openNewWorkspaceModal() {
  closeAllMenus();
  const item = modals.modal("New Workspace", ModalNewWorkspace, { size: "xl" });
  item.componentProps = {
    onCreated: async () => {
      await refreshRecent();
    },
    onClose: () => modals.dismiss(item.id),
    onOpenWorkspace: (path: string, title: string) => openProject(path, title),
  };
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

/**
 * Open a project from inside the IDE. Opening a different project than the current one
 * asks whether to use this window or a new one (unless the user chose "Don't ask again").
 */
async function openProject(path: string, title: string) {
  const current = props.activeWorkspace;
  if (current && normalizePath(current.path) === normalizePath(path)) return;

  const mode = current ? await askOpenMode(title) : "this";
  if (mode === "cancel") return;

  try {
    const project = await invoke<WorkspaceInfo>("open_workspace", { path, new_window: mode === "new" });
    if (mode === "this") emit("workspace-changed", project);
    await refreshRecent();
  } catch (error) {
    toast("danger", "Failed to open workspace", String(error));
  }
}

async function handleOpenWorkspaceDialog() {
  closeAllMenus();
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "Open Workspace Folder",
    });
    if (typeof selected !== "string") return;
    const folderName = selected.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || selected;
    await openProject(selected, folderName);
  } catch (error) {
    toast("danger", "Failed to open workspace", String(error));
  }
}

async function switchWorkspace(project: WorkspaceInfo) {
  closeAllMenus();
  if (props.activeWorkspace?.project_uuid === project.project_uuid) return;
  await openProject(project.path, project.title);
}

async function openSettings() {
  closeAllMenus();
  try {
    await invoke("open_settings_window");
  } catch (error) {
    toast("danger", "Failed to open settings", String(error));
  }
}

async function openDevtools() {
  closeAllMenus();
  try {
    await invoke("open_devtools");
  } catch (error) {
    toast("danger", "Failed to open devtools", String(error));
  }
}
</script>

<template>
  <div ref="navRef" class="ide-titlebar-nav" data-tauri-drag-region="false">
    <!-- Brand badge -->
    <div class="brand-badge" title="Luma IDE">
      <img src="/logo.svg" alt="Luma" class="brand-logo" />
    </div>

    <!-- Main menu button (hamburger) -->
    <div class="menu-container">
      <button
        class="nav-icon-btn"
        :class="{ active: isMainMenuOpen }"
        title="Main Menu"
        aria-label="Main Menu"
        @click.stop="toggleMainMenu"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
          <line x1="3" y1="6" x2="21" y2="6" />
          <line x1="3" y1="12" x2="21" y2="12" />
          <line x1="3" y1="18" x2="21" y2="18" />
        </svg>
      </button>

      <!-- Main menu dropdown -->
      <div v-if="isMainMenuOpen" class="lum-popover main-menu-dropdown">
        <button class="lum-menu-item" @click="openNewWorkspaceModal">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          New Project...
        </button>
        <button class="lum-menu-item" @click="handleOpenWorkspaceDialog">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
          Open...
          <span class="lum-menu-shortcut">Ctrl+O</span>
        </button>
        <div class="lum-menu-separator" />
        <button class="lum-menu-item" @click="openSettings">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
          Settings
          <span class="lum-menu-shortcut">Ctrl+,</span>
        </button>
        <button class="lum-menu-item" @click="openDevtools">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="16 18 22 12 16 6" />
            <polyline points="8 6 2 12 8 18" />
          </svg>
          Developer Tools
          <span class="lum-menu-shortcut">F12</span>
        </button>
      </div>
    </div>

    <!-- Project Widget Dropdown Pill (JetBrains style) -->
    <div class="menu-container">
      <button
        class="nav-pill-btn project-pill"
        :class="{ active: isProjectMenuOpen }"
        title="Switch or Open Project"
        aria-label="Switch or Open Project"
        @click.stop="toggleProjectMenu"
      >
        <span class="project-pill-badge" :style="{ backgroundColor: projectColor }">
          {{ projectInitials }}
        </span>
        <span class="project-pill-title">
          {{ props.activeWorkspace?.title ?? "Workspace" }}
        </span>
        <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      <!-- Project Popup Menu (Image 2 & 3) -->
      <div v-if="isProjectMenuOpen" class="lum-popover project-menu-dropdown">
        <button class="lum-menu-item" @click="openNewWorkspaceModal">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          New Project...
        </button>

        <button class="lum-menu-item" @click="handleOpenWorkspaceDialog">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
          Open...
          <span class="lum-menu-shortcut">Ctrl+O</span>
        </button>

        <button class="lum-menu-item" disabled title="Repository cloning will be added in a later pass">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="6" y1="3" x2="6" y2="15" />
            <circle cx="18" cy="6" r="3" />
            <circle cx="6" cy="18" r="3" />
            <path d="M18 9a9 9 0 0 1-9 9" />
          </svg>
          Clone Repository...
        </button>

        <template v-if="recentWorkspaces.length > 0">
          <div class="lum-menu-separator" />
          <div class="lum-menu-label">Recent Projects</div>

          <div class="recent-projects-list">
            <div
              v-for="project in recentWorkspaces"
              :key="project.project_uuid"
              class="recent-project-row"
              :class="{ current: props.activeWorkspace?.project_uuid === project.project_uuid }"
              role="button"
              tabindex="0"
              @click="switchWorkspace(project)"
            >
              <div class="row-badge" :style="{ backgroundColor: getProjectColor(project.title) }">
                {{ getProjectInitials(project.title) }}
              </div>
              <div class="row-info">
                <div class="row-title">{{ project.title }}</div>
                <div class="row-path">{{ cleanDisplayPath(project.path) }}</div>
                <div v-if="project.git_branch" class="row-branch">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="10" height="10">
                    <line x1="6" y1="3" x2="6" y2="15" />
                    <circle cx="18" cy="6" r="3" />
                    <circle cx="6" cy="18" r="3" />
                    <path d="M18 9a9 9 0 0 1-9 9" />
                  </svg>
                  <span>{{ project.git_branch }}</span>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- Version Control Widget Dropdown Pill (Image 0 & 1) -->
    <div class="menu-container">
      <button
        class="nav-pill-btn vcs-pill"
        :class="{ active: isVcsMenuOpen }"
        title="Version Control"
        aria-label="Version Control"
        @click.stop="toggleVcsMenu"
      >
        <svg v-if="props.activeWorkspace?.git_branch" class="branch-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
          <line x1="6" y1="3" x2="6" y2="15" />
          <circle cx="18" cy="6" r="3" />
          <circle cx="6" cy="18" r="3" />
          <path d="M18 9a9 9 0 0 1-9 9" />
        </svg>
        <span class="vcs-title">
          {{ props.activeWorkspace?.git_branch || "Version Control" }}
        </span>
        <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      <!-- VCS Popup Menu (Image 1) -->
      <div v-if="isVcsMenuOpen" class="lum-popover vcs-menu-dropdown">
        <template v-if="props.activeWorkspace?.git_branch">
          <div class="lum-menu-label">Git: {{ props.activeWorkspace.git_branch }}</div>
          <button class="lum-menu-item" disabled>
            Commit...
            <span class="lum-menu-shortcut">Ctrl+K</span>
          </button>
          <button class="lum-menu-item" disabled>
            Push...
            <span class="lum-menu-shortcut">Ctrl+Shift+K</span>
          </button>
          <button class="lum-menu-item" disabled>
            Update Project...
            <span class="lum-menu-shortcut">Ctrl+T</span>
          </button>
          <div class="lum-menu-separator" />
        </template>

        <button class="lum-menu-item" disabled>
          Share Project On...
        </button>
        <button class="lum-menu-item" disabled>
          Create Git Repository...
        </button>
        <button class="lum-menu-item" disabled>
          Other VCS...
        </button>
        <button class="lum-menu-item" disabled>
          Local History...
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ide-titlebar-nav {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 100%;
  user-select: none;
}

.brand-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 5px;
  margin-right: 2px;
  flex-shrink: 0;
  background: var(--lum-orange-400, rgb(168 43 146 / 0));
}

.brand-logo {
  width: 16px;
  height: 16px;
  object-fit: contain;
  display: block;
}

.menu-container {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.nav-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--lum-fg-muted);
  border-radius: var(--lum-radius-md);
  cursor: pointer;
  transition: background var(--lum-transition-fast), color var(--lum-transition-fast);
}

.nav-icon-btn:hover,
.nav-icon-btn.active {
  background: var(--lum-bg-hover, rgba(255, 255, 255, 0.08));
  color: var(--lum-fg-strong);
}

.nav-pill-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 26px;
  padding: 0 8px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--lum-fg);
  font-size: var(--lum-text-sm);
  font-weight: var(--lum-font-weight-medium);
  cursor: pointer;
  transition: background var(--lum-transition-fast), border-color var(--lum-transition-fast);
  white-space: nowrap;
}

.nav-pill-btn:hover,
.nav-pill-btn.active {
  background: var(--lum-bg-hover, rgba(255, 255, 255, 0.1));
  border-color: var(--lum-border);
}

.project-pill-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.project-pill-title {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vcs-pill {
  color: var(--lum-fg-muted);
}

.vcs-pill .branch-icon {
  color: var(--lum-violet-500, #589df6);
}

.vcs-title {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chevron {
  opacity: 0.6;
  margin-left: 2px;
}

/* Dropdown popovers */
.lum-popover {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: var(--lum-z-popover, 1200);
  background: var(--lum-bg-overlay, #1e1f22);
  border: 1px solid var(--lum-border-strong, #3c3f41);
  border-radius: var(--lum-radius-lg, 8px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
  padding: 4px 0;
  min-width: 220px;
}

.project-menu-dropdown {
  min-width: 280px;
  max-width: 360px;
}

.vcs-menu-dropdown {
  min-width: 200px;
}

.main-menu-dropdown {
  min-width: 200px;
}

.lum-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  height: 28px;
  padding: 0 12px;
  background: transparent;
  border: none;
  font-size: var(--lum-text-sm);
  color: var(--lum-fg);
  text-align: left;
  cursor: pointer;
  transition: background var(--lum-transition-fast);

  svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--lum-fg-muted);
  }

  &:hover:not(:disabled) {
    background: var(--lum-bg-hover, rgba(255, 255, 255, 0.08));
    color: var(--lum-fg-strong);
  }

  &:disabled {
    opacity: 0.45;
    cursor: default;
  }
}

.lum-menu-shortcut {
  margin-left: auto;
  font-size: var(--lum-text-xs);
  color: var(--lum-fg-subtle, #707070);
}

.lum-menu-separator {
  height: 1px;
  margin: 4px 0;
  background: var(--lum-border, #313335);
}

.lum-menu-label {
  padding: 6px 12px 2px;
  font-size: var(--lum-text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--lum-fg-subtle, #707070);
}

/* Recent projects list */
.recent-projects-list {
  max-height: 240px;
  overflow-y: auto;
}

.recent-project-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 6px 12px;
  cursor: pointer;
  transition: background var(--lum-transition-fast);

  &:hover {
    background: var(--lum-bg-hover, rgba(255, 255, 255, 0.08));
  }

  &.current {
    background: rgba(88, 157, 246, 0.12);
  }
}

.row-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
  margin-top: 1px;
}

.row-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.row-title {
  font-size: var(--lum-text-sm);
  font-weight: 600;
  color: var(--lum-fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-path {
  font-size: 11px;
  color: var(--lum-fg-subtle, #8c8c8c);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-branch {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--lum-violet-500, #589df6);
  margin-top: 1px;

  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}
</style>
