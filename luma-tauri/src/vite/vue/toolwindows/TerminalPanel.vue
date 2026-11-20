<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import "@xterm/xterm/css/xterm.css";
import LumToolWindow from "../components/LumToolWindow.vue";

const props = defineProps<{ workspacePath: string }>();
const emit = defineEmits<{ (event: "hide"): void }>();

interface TerminalSessionInfo {
  id: string;
  shell: string;
  cwd: string;
  exited: boolean;
}
interface TerminalTab {
  id: string;
  shell: string;
  cwd: string;
  label: string;
  exited: boolean;
}
interface TerminalView {
  terminal: Terminal;
  fitAddon: FitAddon;
  unicodeAddon: Unicode11Addon;
  inputSubscription: { dispose: () => void };
  resizeObserver: ResizeObserver;
  poller: ReturnType<typeof setInterval> | undefined;
  offset: number;
  writeQueue: Promise<void>;
  disposed: boolean;
  exited: boolean;
  exitMessageShown: boolean;
  readErrorShown: boolean;
  writeErrorShown: boolean;
}

const tabs = ref<TerminalTab[]>([]);
const activeTabId = ref("");
const hosts = new Map<string, HTMLDivElement>();
const views = new Map<string, TerminalView>();
const creating = ref(false);
const loading = ref(true);
let disposed = false;

function normalizedPath(path: string) {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLocaleLowerCase();
}

function setHost(id: string, element: Element | null) {
  if (element instanceof HTMLDivElement) hosts.set(id, element);
  else hosts.delete(id);
}

function cssVariable(name: string, fallback: string) {
  if (typeof window === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function nextLabel(shell: string) {
  const number = tabs.value.filter((tab) => tab.shell === shell).length + 1;
  return number === 1 ? shell : `${shell} ${number}`;
}

function disposeView(id: string) {
  const view = views.get(id);
  if (!view) return;
  view.disposed = true;
  if (view.poller) clearInterval(view.poller);
  view.poller = undefined;
  view.inputSubscription.dispose();
  view.resizeObserver.disconnect();
  view.unicodeAddon.dispose();
  view.terminal.dispose();
  views.delete(id);
}

function disposeAllViews() {
  for (const id of [...views.keys()]) disposeView(id);
  hosts.clear();
}

function markExited(sessionId: string, view: TerminalView) {
  if (view.exited) return;
  view.exited = true;
  if (view.poller) clearInterval(view.poller);
  view.poller = undefined;
  if (!view.exitMessageShown && !view.disposed) {
    view.terminal.writeln("\r\n[Shell exited]");
    view.exitMessageShown = true;
  }
  const tab = tabs.value.find((item) => item.id === sessionId);
  if (tab) tab.exited = true;
}

async function poll(sessionId: string, view: TerminalView) {
  if (view.disposed || disposed) return;
  if (views.get(sessionId) !== view) return;
  try {
    const result = await invoke<{ output: string; nextOffset: number; exited: boolean }>(
      "terminal_read",
      { sessionId, offset: view.offset },
    );
    if (view.disposed || disposed) return;
    if (result.output) {
      view.offset = result.nextOffset;
      view.terminal.write(result.output);
    }
    if (result.exited) markExited(sessionId, view);
    view.readErrorShown = false;
  } catch (error) {
    if (!view.disposed && !view.readErrorShown) {
      view.terminal.writeln("\r\n\x1b[33mTerminal read temporarily unavailable: " + String(error) + "\x1b[0m");
      view.readErrorShown = true;
    }
    // A transient read failure must not poison the tab. Keep polling so a
    // recovered PTY can resume without closing and reopening the tool window.
  }
}

function send(sessionId: string, view: TerminalView, data: string) {
  if (view.disposed || disposed || view.exited) return;
  view.writeQueue = view.writeQueue.then(async () => {
    if (view.disposed || disposed || view.exited || views.get(sessionId) !== view) return;
    try {
      await invoke("terminal_write", { sessionId, input: data });
      view.writeErrorShown = false;
    } catch (error) {
      if (!view.disposed && !view.writeErrorShown) {
        view.terminal.writeln("\r\n\x1b[33mInput temporarily unavailable: " + String(error) + "\x1b[0m");
        view.writeErrorShown = true;
      }
      // Do not mark the shell dead on a single failed write. The backend
      // checks the child and refreshes its PTY writer before returning errors.
    }
  }).catch(() => undefined);
}

async function mountView(tab: TerminalTab) {
  if (views.has(tab.id) || disposed) return;
  await nextTick();
  const host = hosts.get(tab.id);
  if (!host) return;

  const terminal = new Terminal({
    cursorBlink: true,
    convertEol: false,
    allowProposedApi: true,
    fontFamily: "'Cascadia Mono', 'Cascadia Code', 'Consolas', 'Noto Sans Mono CJK SC', 'Noto Sans Mono CJK JP', 'Noto Sans Mono CJK KR', 'Noto Sans Symbols 2', 'Segoe UI Symbol', 'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', 'Microsoft YaHei UI', 'Yu Gothic UI', 'Malgun Gothic', 'Courier New', monospace",
    fontSize: 13,
    lineHeight: 1.25,
    scrollback: 5000,
    theme: {
      background: cssVariable("--lum-bg-app", "#111318"),
      foreground: cssVariable("--lum-fg", "#d6d9e0"),
      cursor: cssVariable("--lum-accent", "#8ab4f8"),
      selectionBackground: "rgba(120, 150, 210, 0.32)",
    },
  });

  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  const unicodeAddon = new Unicode11Addon();
  terminal.loadAddon(unicodeAddon);
  terminal.unicode.activeVersion = "11";
  terminal.open(host);

  const view: TerminalView = {
    terminal,
    fitAddon,
    unicodeAddon,
    inputSubscription: { dispose: () => undefined },
    resizeObserver: new ResizeObserver(() => {
      requestAnimationFrame(() => {
        if (view.disposed || !host.isConnected || host.clientWidth <= 0 || host.clientHeight <= 0) return;
        try {
          fitAddon.fit();
          if (!view.exited) {
            void invoke("terminal_resize", { sessionId: tab.id, cols: terminal.cols, rows: terminal.rows }).catch(() => undefined);
          }
        } catch {
          // The panel can be temporarily hidden while its side is changing.
        }
      });
    }),
    poller: undefined,
    offset: 0,
    writeQueue: Promise.resolve(),
    disposed: false,
    exited: false,
    exitMessageShown: false,
    readErrorShown: false,
    writeErrorShown: false,
  };

  view.inputSubscription = terminal.onData((data) => send(tab.id, view, data));
  views.set(tab.id, view);
  view.resizeObserver.observe(host);

  await nextTick();
  try {
    if (host.clientWidth > 0 && host.clientHeight > 0) fitAddon.fit();
  } catch {
    // A later ResizeObserver notification will fit the terminal when visible.
  }

  await poll(tab.id, view);
  if (!view.exited && !view.disposed) {
    view.poller = setInterval(() => void poll(tab.id, view), 100);
  }
  terminal.focus();
}

async function selectTab(id: string) {
  activeTabId.value = id;
  await nextTick();
  const tab = tabs.value.find((item) => item.id === id);
  if (!tab) return;
  const view = views.get(id);
  if (!view) {
    await mountView(tab);
    return;
  }
  try {
    view.fitAddon.fit();
    if (!view.exited) {
      await invoke("terminal_resize", { sessionId: id, cols: view.terminal.cols, rows: view.terminal.rows });
    }
  } catch {
    // Resize is best-effort while a tab is becoming visible.
  }
  view.terminal.focus();
}

async function loadSessions() {
  loading.value = true;
  try {
    const sessions = await invoke<TerminalSessionInfo[]>("terminal_list_sessions");
    if (disposed) return;
    const currentPath = normalizedPath(props.workspacePath);
    const matching = sessions.filter((session) => normalizedPath(session.cwd) === currentPath);
    tabs.value = matching.map((session) => ({
      id: session.id,
      shell: session.shell,
      cwd: session.cwd,
      label: session.shell,
      exited: session.exited,
    }));
    tabs.value.forEach((tab, index) => {
      const count = matching.slice(0, index + 1).filter((session) => session.shell === tab.shell).length;
      tab.label = count === 1 ? tab.shell : `${tab.shell} ${count}`;
    });

    if (tabs.value.length === 0) {
      await createTab();
    } else {
      await selectTab(tabs.value[0].id);
    }
  } catch (error) {
    console.error("Unable to restore terminal sessions", error);
    tabs.value = [];
  } finally {
    loading.value = false;
  }
}

async function createTab() {
  if (creating.value || disposed) return;
  creating.value = true;
  try {
    const session = await invoke<TerminalSessionInfo>("terminal_open_session", { cwd: props.workspacePath });
    if (disposed) {
      await invoke("terminal_close_session", { sessionId: session.id }).catch(() => undefined);
      return;
    }
    const tab: TerminalTab = {
      id: session.id,
      shell: session.shell,
      cwd: session.cwd,
      label: nextLabel(session.shell),
      exited: session.exited,
    };
    tabs.value = [...tabs.value, tab];
    await selectTab(tab.id);
  } catch (error) {
    console.error("Failed to create terminal session", error);
  } finally {
    creating.value = false;
  }
}

async function closeTab(id: string) {
  disposeView(id);
  tabs.value = tabs.value.filter((tab) => tab.id !== id);
  if (activeTabId.value === id) activeTabId.value = tabs.value[0]?.id ?? "";
  await invoke("terminal_close_session", { sessionId: id }).catch(() => undefined);
  if (activeTabId.value) await selectTab(activeTabId.value);
}

onMounted(async () => {
  disposed = false;
  await loadSessions();
});

onBeforeUnmount(() => {
  disposed = true;
  // Hiding or closing the dock only detaches its views. The backend owns PTYs
  // for the lifetime of the IDE and restores their scrollback when reopened.
  disposeAllViews();
});

watch(() => props.workspacePath, async (path, previousPath) => {
  if (path === previousPath) return;
  disposeAllViews();
  tabs.value = [];
  activeTabId.value = "";
  await loadSessions();
});
</script>

<template>
  <LumToolWindow title="Terminal" @hide="emit('hide')">
    <template #actions>
      <button
        type="button"
        class="lum-tool-window-btn lum-terminal-new"
        title="New terminal"
        aria-label="New terminal"
        :disabled="creating"
        @click="createTab"
      >+</button>
    </template>

    <div class="lum-terminal-panel">
      <div v-if="tabs.length" class="lum-tabs lum-terminal-tabs" role="tablist" aria-label="Terminal sessions">
        <div v-for="tab in tabs" :key="tab.id" class="lum-terminal-tab-item">
          <button
            type="button"
            class="lum-tab lum-terminal-tab"
            role="tab"
            :aria-selected="activeTabId === tab.id"
            :class="{ active: activeTabId === tab.id, 'is-exited': tab.exited }"
            @click="selectTab(tab.id)"
          >
            <span>{{ tab.label }}</span>
            <span v-if="tab.exited" class="lum-terminal-exited">Exited</span>
          </button>
          <button
            type="button"
            class="lum-tool-window-btn lum-terminal-tab-close"
            :title="`Close ${tab.label}`"
            :aria-label="`Close ${tab.label}`"
            @click="closeTab(tab.id)"
          >×</button>
        </div>
      </div>

      <div v-if="!tabs.length && !loading" class="lum-terminal-empty">
        <span>No terminal sessions are open.</span>
        <button type="button" class="lum-terminal-empty-action" @click="createTab">Open terminal</button>
      </div>

      <div class="lum-terminal-views">
        <div
          v-for="tab in tabs"
          :key="tab.id"
          :ref="(element) => setHost(tab.id, element as HTMLDivElement | null)"
          v-show="activeTabId === tab.id"
          class="lum-terminal-host"
        />
      </div>
    </div>
  </LumToolWindow>
</template>

<style scoped>
.lum-terminal-panel {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--lum-bg-app);
}

.lum-terminal-tabs {
  flex: 0 0 var(--lum-tabs-height);
  min-height: var(--lum-tabs-height);
  padding: 0 var(--lum-space-2);
  gap: var(--lum-tab-gap);
  background: var(--lum-bg-surface);
}

.lum-terminal-tab-item {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  max-width: 220px;
  min-width: 0;
  border-radius: var(--lum-tab-radius);
}

.lum-terminal-tab {
  min-width: 0;
  max-width: 190px;
  gap: var(--lum-space-2);
}

.lum-terminal-tab span:first-child {
  overflow: hidden;
  text-overflow: ellipsis;
}

.lum-terminal-tab[aria-selected="true"] {
  color: var(--lum-tab-fg-active);
}

.lum-terminal-tab-close {
  flex: 0 0 20px;
  width: 20px;
  height: 20px;
  margin-right: var(--lum-space-1);
}

.lum-terminal-exited {
  color: var(--lum-fg-subtle);
  font-size: var(--lum-text-xs);
}

.lum-terminal-views {
  display: flex;
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.lum-terminal-host {
  flex: 1 1 auto;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  padding: var(--lum-space-2) var(--lum-space-3);
  overflow: hidden;
  background: var(--lum-bg-app);
}

.lum-terminal-host :deep(.xterm) {
  height: 100%;
}

.lum-terminal-host :deep(.xterm-viewport) {
  overflow-y: auto;
}

.lum-terminal-empty {
  display: flex;
  flex: 1 1 auto;
  align-items: center;
  justify-content: center;
  gap: var(--lum-space-3);
  color: var(--lum-fg-muted);
}

.lum-terminal-empty-action,
.lum-terminal-new {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 24px;
  padding: 0 var(--lum-space-2);
  border: 1px solid var(--lum-border);
  border-radius: var(--lum-radius-sm);
  background: var(--lum-bg-surface);
  color: var(--lum-fg);
  cursor: pointer;
}

.lum-terminal-empty-action:hover,
.lum-terminal-new:hover:not(:disabled) {
  border-color: var(--lum-border-focus);
  background: var(--lum-bg-hover);
  color: var(--lum-fg-strong);
}

.lum-terminal-new:disabled {
  opacity: 0.45;
  cursor: default;
}
</style>
