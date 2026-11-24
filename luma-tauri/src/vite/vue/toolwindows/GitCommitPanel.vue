<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from "vue";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import LumToolWindow from "../components/LumToolWindow.vue";
import { FILES_CHANGED_EVENT, type FilesChangedPayload } from "../composables/fileWatcher";
import {
  gitStatus,
  gitBranches,
  gitLog,
  gitDiff,
  gitInit,
  gitStage,
  gitUnstage,
  gitCommit,
  gitCheckout,
  gitCreateBranch,
  gitFetch,
  gitPull,
  gitPush,
  type GitBranch,
  type GitFileChange,
  type GitLogEntry,
  type GitStatus,
} from "../composables/useGit";

const props = defineProps<{ workspacePath: string }>();
const emitHide = defineEmits<{ (event: "hide"): void }>();

const status = ref<GitStatus>({ branch: "", changes: [] });
const branches = ref<GitBranch[]>([]);
const log = ref<GitLogEntry[]>([]);
const initialized = ref(true);
const busy = ref(false);
const error = ref("");
const notice = ref("");
const message = ref("");
const newBranch = ref("");
const showNewBranch = ref(false);
const selectedPath = ref<string | null>(null);
const unstagedDiff = ref("");
const stagedDiff = ref("");

const staged = computed(() => status.value.changes.filter((c) => c.index !== " " && c.index !== "?"));
const unstaged = computed(() =>
  status.value.changes.filter((c) => c.worktree !== " " || c.index === " " || c.index === "?"),
);
const stagedCount = computed(() => staged.value.length);

let unlistenFiles: UnlistenFn | null = null;
let unlistenPanelAction: UnlistenFn | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;

function statusLabel(change: GitFileChange): string {
  if (change.index === "?") return "??";
  return (change.index.trim() + change.worktree.trim()) || "M";
}

function emitBranch(branch: string) {
  void emit("workspace-git-branch-changed", { branch });
}

async function refresh() {
  try {
    const [s, b] = await Promise.all([gitStatus(props.workspacePath), gitBranches(props.workspacePath)]);
    const previousBranch = status.value.branch;
    status.value = s;
    branches.value = b;
    initialized.value = true;
    error.value = "";
    if (s.branch && s.branch !== previousBranch) emitBranch(s.branch);
    try {
      log.value = await gitLog(props.workspacePath);
    } catch {
      log.value = []; // repository with no commits yet
    }
  } catch (cause) {
    const text = String(cause);
    if (/not a git repository/i.test(text)) {
      initialized.value = false;
      status.value = { branch: "", changes: [] };
      branches.value = [];
      log.value = [];
      error.value = "";
    } else {
      error.value = text;
    }
  }
}

async function run(action: () => Promise<unknown>, success?: string) {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  notice.value = "";
  const before = status.value.branch;
  try {
    await action();
    await refresh();
    if (success) notice.value = success;
    if (status.value.branch && status.value.branch !== before) emitBranch(status.value.branch);
    if (selectedPath.value) await selectFile(selectedPath.value);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

async function selectFile(path: string) {
  selectedPath.value = path;
  try {
    const [plain, stagedText] = await Promise.all([
      gitDiff(props.workspacePath, path),
      gitDiff(props.workspacePath, path, true),
    ]);
    unstagedDiff.value = plain || "No unstaged changes";
    stagedDiff.value = stagedText || "No staged changes";
  } catch (cause) {
    unstagedDiff.value = String(cause);
    stagedDiff.value = "";
  }
}

async function stage(paths: string[]) {
  await run(() => gitStage(props.workspacePath, paths), `Staged ${paths.length} file(s)`);
}

async function unstage(paths: string[]) {
  await run(() => gitUnstage(props.workspacePath, paths), `Unstaged ${paths.length} file(s)`);
}

async function commit() {
  const text = message.value.trim();
  if (!text) return;
  if (!stagedCount.value) {
    error.value = "Stage changes before committing.";
    return;
  }
  await run(async () => {
    await gitCommit(props.workspacePath, text);
    message.value = "";
  }, "Commit created");
}

async function checkout(branch: string) {
  if (!branch || branch === status.value.branch) return;
  await run(() => gitCheckout(props.workspacePath, branch), `Switched to ${branch}`);
}

async function createBranch() {
  const name = newBranch.value.trim();
  if (!name) return;
  await run(() => gitCreateBranch(props.workspacePath, name), `Created branch ${name}`);
  newBranch.value = "";
  showNewBranch.value = false;
}

async function initGit() {
  await run(() => gitInit(props.workspacePath), "Git repository initialized");
}

watch(selectedPath, (path) => {
  if (path) void selectFile(path);
});

onMounted(async () => {
  await refresh();
  unlistenFiles = await listen<FilesChangedPayload>(FILES_CHANGED_EVENT, () => {
    if (document.hidden) return;
    if (refreshTimer) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => void refresh(), 500);
  });
  unlistenPanelAction = await listen<{ action: string }>("git-panel-action", (event) => {
    if (event.payload.action === "push") {
      void run(() => gitPush(props.workspacePath, status.value.branch), "Pushed");
    } else if (event.payload.action === "pull") {
      void run(() => gitPull(props.workspacePath), "Pulled");
    }
  });
  window.addEventListener("focus", refresh);
});

onBeforeUnmount(() => {
  unlistenFiles?.();
  unlistenPanelAction?.();
  if (refreshTimer) clearTimeout(refreshTimer);
  window.removeEventListener("focus", refresh);
});
</script>

<template>
  <LumToolWindow title="Git Commit" @hide="emitHide('hide')">
    <div class="lum-git-panel">
      <div class="lum-git-toolbar">
        <button :disabled="busy" title="Refresh" @click="refresh">⟳</button>
        <template v-if="initialized">
          <button :disabled="busy || !unstaged.length" @click="stage(unstaged.map((c) => c.path))">Stage All</button>
          <button :disabled="busy || !staged.length" @click="unstage(staged.map((c) => c.path))">Unstage All</button>
          <button :disabled="busy" @click="run(() => gitFetch(workspacePath), 'Fetched')">Fetch</button>
          <button :disabled="busy" @click="run(() => gitPull(workspacePath), 'Pulled')">Pull</button>
          <button :disabled="busy" @click="run(() => gitPush(workspacePath, status.branch), 'Pushed')">Push</button>
          <button :disabled="busy" @click="showNewBranch = !showNewBranch">+ Branch</button>
        </template>
        <button v-else :disabled="busy" class="lum-git-init" @click="initGit">Init Git Repo</button>
      </div>

      <template v-if="initialized">
        <div class="lum-git-branch-row">
          <label for="lum-git-branch">Branch</label>
          <select
            id="lum-git-branch"
            :value="status.branch"
            :disabled="busy"
            @change="checkout(($event.target as HTMLSelectElement).value)"
          >
            <option v-for="branch in branches" :key="branch.name" :value="branch.name">
              {{ branch.name }}{{ branch.upstream ? ` → ${branch.upstream}` : "" }}
            </option>
          </select>
        </div>
        <div v-if="showNewBranch" class="lum-git-new-branch">
          <input
            v-model="newBranch"
            placeholder="new-branch-name"
            @keydown.enter.prevent="createBranch"
            @keydown.escape.prevent="showNewBranch = false"
          />
          <button :disabled="busy || !newBranch.trim()" @click="createBranch">Create</button>
        </div>

        <div class="lum-git-columns">
          <section class="lum-git-list">
            <label class="lum-git-label">Changes ({{ unstaged.length }})</label>
            <div v-if="!unstaged.length" class="lum-git-empty">No unstaged changes</div>
            <div
              v-for="change in unstaged"
              :key="`u-${change.path}`"
              class="lum-git-file"
              :class="{ selected: selectedPath === change.path }"
              @click="selectFile(change.path)"
            >
              <span class="lum-git-file-status">{{ statusLabel(change) }}</span>
              <span class="lum-git-file-path" :title="change.path">{{ change.path }}</span>
              <button class="lum-git-file-action" title="Stage" :disabled="busy" @click.stop="stage([change.path])">+</button>
            </div>

            <label class="lum-git-label">Staged Changes ({{ staged.length }})</label>
            <div v-if="!staged.length" class="lum-git-empty">Nothing staged</div>
            <div
              v-for="change in staged"
              :key="`s-${change.path}`"
              class="lum-git-file"
              :class="{ selected: selectedPath === change.path }"
              @click="selectFile(change.path)"
            >
              <span class="lum-git-file-status">{{ statusLabel(change) }}</span>
              <span class="lum-git-file-path" :title="change.path">{{ change.path }}</span>
              <button class="lum-git-file-action" title="Unstage" :disabled="busy" @click.stop="unstage([change.path])">−</button>
            </div>
          </section>

          <section class="lum-git-diff">
            <label class="lum-git-label">Diff {{ selectedPath ? `— ${selectedPath}` : "" }}</label>
            <template v-if="selectedPath">
              <div class="lum-git-diff-section">Unstaged</div>
              <pre class="lum-git-diff-text">{{ unstagedDiff }}</pre>
              <div class="lum-git-diff-section">Staged</div>
              <pre class="lum-git-diff-text">{{ stagedDiff }}</pre>
            </template>
            <div v-else class="lum-git-empty">Select a file to see its diff</div>
          </section>
        </div>

        <div class="lum-git-message">
          <label class="lum-git-label" for="lum-commit-message">Commit message ({{ stagedCount }} staged)</label>
          <textarea
            id="lum-commit-message"
            v-model="message"
            rows="3"
            placeholder="Describe your changes"
            @keydown.ctrl.enter.prevent="commit"
          ></textarea>
          <button class="lum-git-commit" :disabled="busy || !message.trim() || !stagedCount" @click="commit">
            Commit
          </button>
        </div>

        <label class="lum-git-label">Recent Commits</label>
        <div class="lum-git-log">
          <div v-for="entry in log" :key="entry.hash" class="lum-git-log-entry">
            <span class="lum-git-log-hash">{{ entry.hash }}</span>
            <span class="lum-git-log-message">{{ entry.message }}</span>
          </div>
          <div v-if="!log.length" class="lum-git-empty">No commits yet</div>
        </div>
      </template>

      <div v-else class="lum-git-not-init">
        <p class="lum-git-hint">This project is not a Git repository. Initialize one to start committing.</p>
      </div>

      <p v-if="notice" class="lum-git-success">{{ notice }}</p>
      <p v-if="error" class="lum-git-error">{{ error }}</p>
    </div>
  </LumToolWindow>
</template>

<style scoped>
.lum-git-panel { flex: 1; min-height: 0; overflow: auto; padding: 10px; color: var(--lum-fg); font-size: var(--lum-text-sm); }
.lum-git-toolbar { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
.lum-git-toolbar button,
.lum-git-new-branch button,
.lum-git-commit { border: 1px solid var(--lum-border); border-radius: var(--lum-radius-sm); background: var(--lum-bg-hover); color: var(--lum-fg); padding: 5px 10px; cursor: pointer; }
.lum-git-toolbar button:disabled,
.lum-git-new-branch button:disabled,
.lum-git-commit:disabled { opacity: .45; cursor: default; }
.lum-git-init { background: var(--lum-accent-muted) !important; }
.lum-git-branch-row { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.lum-git-branch-row label { color: var(--lum-fg-muted); font-size: var(--lum-text-xs); }
.lum-git-branch-row select { flex: 1; min-width: 0; background: var(--lum-bg-app); border: 1px solid var(--lum-border); border-radius: var(--lum-radius-sm); color: var(--lum-fg); padding: 4px 6px; }
.lum-git-new-branch { display: flex; gap: 6px; margin-bottom: 8px; }
.lum-git-new-branch input { flex: 1; min-width: 0; background: var(--lum-bg-app); border: 1px solid var(--lum-border); border-radius: var(--lum-radius-sm); color: var(--lum-fg); padding: 4px 6px; }
.lum-git-label { display: block; margin: 9px 0 4px; color: var(--lum-fg-muted); font-size: var(--lum-text-xs); }
.lum-git-columns { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.1fr); gap: 10px; }
.lum-git-list, .lum-git-diff { min-width: 0; }
.lum-git-file { display: flex; align-items: center; gap: 6px; padding: 3px 4px; border-radius: var(--lum-radius-sm); cursor: pointer; }
.lum-git-file:hover { background: var(--lum-bg-hover); }
.lum-git-file.selected { background: var(--lum-accent-muted); }
.lum-git-file-status { color: var(--lum-accent); font: var(--lum-text-xs) var(--lum-font-mono); min-width: 20px; }
.lum-git-file-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: var(--lum-text-xs) var(--lum-font-mono); }
.lum-git-file-action { border: none; background: transparent; color: var(--lum-fg-muted); cursor: pointer; font-size: 14px; padding: 0 4px; }
.lum-git-file-action:hover { color: var(--lum-fg-strong); }
.lum-git-empty { color: var(--lum-fg-muted); font-size: var(--lum-text-xs); padding: 4px 0; }
.lum-git-diff-section { color: var(--lum-fg-muted); font-size: var(--lum-text-xs); margin: 4px 0 2px; }
.lum-git-diff-text { margin: 0 0 6px; padding: 8px; background: var(--lum-bg-app); border: 1px solid var(--lum-border-muted); border-radius: var(--lum-radius-sm); font: var(--lum-text-xs) var(--lum-font-mono); white-space: pre; overflow: auto; max-height: 220px; }
.lum-git-message textarea { width: 100%; box-sizing: border-box; resize: vertical; border: 1px solid var(--lum-border); border-radius: var(--lum-radius-sm); padding: 8px; background: var(--lum-bg-app); color: var(--lum-fg); font: var(--lum-text-sm) var(--lum-font-mono); }
.lum-git-commit { width: 100%; margin-top: 8px; background: var(--lum-accent-muted); }
.lum-git-log { max-height: 150px; overflow: auto; border: 1px solid var(--lum-border-muted); border-radius: var(--lum-radius-sm); }
.lum-git-log-entry { display: flex; gap: 8px; padding: 3px 6px; font: var(--lum-text-xs) var(--lum-font-mono); }
.lum-git-log-hash { color: var(--lum-accent); }
.lum-git-log-message { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.lum-git-success { color: var(--lum-success, #45a36a); white-space: pre-wrap; }
.lum-git-error { color: var(--lum-danger); white-space: pre-wrap; overflow-wrap: anywhere; }
.lum-git-hint { color: var(--lum-fg-muted); margin-top: 6px; font-size: var(--lum-text-sm); }
</style>
