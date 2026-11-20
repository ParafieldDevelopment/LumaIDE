<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import LumToolWindow from "../components/LumToolWindow.vue";

const props = defineProps<{ workspacePath: string }>();
const emit = defineEmits<{ (event: "hide"): void }>();

const host = ref<HTMLDivElement | null>(null);
const sessionId = ref("");
const shellName = ref("");
const terminalReady = ref(false);

let terminal: Terminal | undefined;
let fitAddon: FitAddon | undefined;
let resizeObserver: ResizeObserver | undefined;
let poller: ReturnType<typeof setInterval> | undefined;
let offset = 0;
let disposed = false;
let writeQueue = Promise.resolve();

async function openSession() {
  try {
    const session = await invoke<{ id: string; shell: string }>(
      "terminal_open_session",
      { cwd: props.workspacePath },
    );
    if (disposed) {
      await invoke("terminal_close_session", { sessionId: session.id }).catch(() => undefined);
      return;
    }
    sessionId.value = session.id;
    shellName.value = session.shell;
    offset = 0;

    await poll();
    poller = setInterval(() => void poll(), 80);
    await nextTick();
    fitAddon?.fit();
    terminal?.focus();
  } catch (error) {
    terminal?.writeln("");
    terminal?.writeln("\x1b[31mFailed to open terminal: " + String(error) + "\x1b[0m");
  }
}

async function poll() {
  if (!sessionId.value || disposed) return;
  try {
    const result = await invoke<{ output: string; nextOffset: number; exited: boolean }>(
      "terminal_read",
      { sessionId: sessionId.value, offset },
    );
    if (result.output) {
      offset = result.nextOffset;
      terminal?.write(result.output);
    }
    if (result.exited) {
      terminal?.writeln("\r\n[Shell exited]");
      stopPolling();
    }
  } catch (error) {
    if (!disposed) terminal?.writeln("\r\n\x1b[31mTerminal read error: " + String(error) + "\x1b[0m");
    stopPolling();
  }
}

function stopPolling() {
  if (poller) clearInterval(poller);
  poller = undefined;
}

function send(data: string) {
  const currentSession = sessionId.value;
  if (!currentSession) return;
  writeQueue = writeQueue.then(async () => {
    if (sessionId.value === currentSession) {
      await invoke("terminal_write", { sessionId: currentSession, input: data });
    }
  }).catch((error) => {
    terminal?.writeln("\r\n\x1b[31mTerminal input error: " + String(error) + "\x1b[0m");
  });
}

function createTerminal() {
  terminal = new Terminal({
    cursorBlink: true,
    convertEol: false,
    allowProposedApi: true,
    fontFamily: "Cascadia Code, Consolas, 'Courier New', monospace",
    fontSize: 13,
    lineHeight: 1.25,
    scrollback: 5000,
    theme: {
      background: getCssVariable("--lum-bg-app", "#111318"),
      foreground: getCssVariable("--lum-fg", "#d6d9e0"),
      cursor: getCssVariable("--lum-accent", "#8ab4f8"),
      selectionBackground: "rgba(120, 150, 210, 0.32)",
    },
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  terminal.onData((data) => send(data));
  terminal.open(host.value!);
  terminalReady.value = true;

  resizeObserver = new ResizeObserver(() => {
    requestAnimationFrame(() => {
      if (!host.value || !terminal || !fitAddon) return;
      fitAddon.fit();
      const { cols, rows } = terminal;
      if (sessionId.value) {
        void invoke("terminal_resize", { sessionId: sessionId.value, cols, rows }).catch(() => undefined);
      }
    });
  });
  resizeObserver.observe(host.value!);
}

function getCssVariable(name: string, fallback: string) {
  if (typeof window === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function disposeTerminal() {
  resizeObserver?.disconnect();
  resizeObserver = undefined;
  stopPolling();
  terminal?.dispose();
  terminal = undefined;
  fitAddon = undefined;
  terminalReady.value = false;
}

async function closeSession() {
  stopPolling();
  const currentSession = sessionId.value;
  sessionId.value = "";
  if (currentSession) {
    try {
      await invoke("terminal_close_session", { sessionId: currentSession });
    } catch {
      // The shell may already have exited.
    }
  }
}

onMounted(async () => {
  createTerminal();
  await openSession();
});

onBeforeUnmount(() => {
  disposed = true;
  disposeTerminal();
  void closeSession();
});

watch(() => props.workspacePath, async (path, oldPath) => {
  if (path === oldPath) return;
  await closeSession();
  terminal?.clear();
  terminal?.writeln("Opening shell in " + path);
  await openSession();
});
</script>

<template>
  <LumToolWindow title="Terminal" @hide="emit('hide')">
    <div class="lum-terminal-host" ref="host" :data-ready="terminalReady" />
  </LumToolWindow>
</template>

<style scoped>
.lum-terminal-host {
  flex: 1;
  min-width: 0;
  min-height: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  padding: 8px 10px;
  background: var(--lum-bg-app);
}
.lum-terminal-host :deep(.xterm) {
  height: 100%;
}
.lum-terminal-host :deep(.xterm-viewport) {
  overflow-y: auto;
}
</style>
