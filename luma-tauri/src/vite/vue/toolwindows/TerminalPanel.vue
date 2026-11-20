<script setup lang="ts">
import { ref, computed, nextTick } from "vue";
import LumToolWindow from "../components/LumToolWindow.vue";
import { runTerminalCommand, changeDirectory, type CommandOutput } from "../composables/useShell";

const props = defineProps<{ workspacePath: string }>();
const emit = defineEmits<{ (event: "hide"): void }>();
const cwd = ref(props.workspacePath);
const command = ref("");
const running = ref(false);
const output = ref<{ command: string; cwd: string; result: CommandOutput | null; error?: string }[]>([]);
const input = ref<HTMLInputElement | null>(null);
const scroll = ref<HTMLElement | null>(null);
const prompt = computed(() => cwd.value);
async function execute() {
  const text = command.value.trim();
  if (!text || running.value) return;
  command.value = "";
  if (text === "clear" || text === "cls") { output.value = []; return; }
  if (text === "cd" || text.startsWith("cd ")) {
    const target = text.slice(2).trim() || "~";
    const previous = cwd.value;
    try {
      cwd.value = await changeDirectory(cwd.value, target);
      output.value.push({ command: text, cwd: previous, result: { code: 0, stdout: cwd.value, stderr: "", timedOut: false } });
    } catch (error) { output.value.push({ command: text, cwd: previous, result: null, error: String(error) }); }
    await nextTick(); scroll.value?.scrollTo({ top: scroll.value.scrollHeight });
    return;
  }
  running.value = true;
  output.value.push({ command: text, cwd: prompt.value, result: null });
  const index = output.value.length - 1;
  try { output.value[index].result = await runTerminalCommand(cwd.value, text); }
  catch (error) { output.value[index].error = String(error); }
  finally { running.value = false; await nextTick(); scroll.value?.scrollTo({ top: scroll.value.scrollHeight }); input.value?.focus(); }
}
</script>
<template>
  <LumToolWindow title="Terminal" @hide="emit('hide')">
    <div class="lum-terminal" ref="scroll">
      <div class="lum-terminal-welcome">Command runner (non-interactive) · {{ cwd }}</div>
      <div v-for="(item, i) in output" :key="i" class="lum-terminal-entry">
        <div class="lum-terminal-prompt"><span>PS</span> {{ item.cwd }} &gt; {{ item.command }}</div>
        <pre v-if="item.result?.stdout" class="lum-terminal-out">{{ item.result.stdout }}</pre>
        <pre v-if="item.result?.stderr" class="lum-terminal-err">{{ item.result.stderr }}</pre>
        <pre v-if="item.error" class="lum-terminal-err">{{ item.error }}</pre>
        <div v-if="item.result?.timedOut" class="lum-terminal-err">Command timed out (60 seconds).</div>
        <div v-if="item.result && item.result.code !== 0 && !item.result.timedOut" class="lum-terminal-err">Exit code: {{ item.result.code }}</div>
      </div>
      <div v-if="running" class="lum-terminal-status">Running…</div>
    </div>
    <form class="lum-terminal-input-row" @submit.prevent="execute">
      <span class="lum-terminal-input-prompt">&gt;</span>
      <input ref="input" v-model="command" :disabled="running" spellcheck="false" autocomplete="off" placeholder="Enter command…" />
      <button type="submit" :disabled="running || !command.trim()">Run</button>
    </form>
  </LumToolWindow>
</template>
<style scoped>
.lum-terminal { flex:1; min-height:0; overflow:auto; padding:10px 12px; background:var(--lum-bg-app); color:var(--lum-fg); font-family:var(--lum-font-mono); font-size:var(--lum-text-xs); }
.lum-terminal-welcome { color:var(--lum-fg-muted); padding-bottom:8px; }
.lum-terminal-entry { margin:0 0 10px; }
.lum-terminal-prompt { color:var(--lum-accent); white-space:pre-wrap; overflow-wrap:anywhere; }
.lum-terminal-prompt span { color:var(--lum-fg-muted); }
.lum-terminal-out,.lum-terminal-err { margin:3px 0 0; white-space:pre-wrap; overflow-wrap:anywhere; font:inherit; }
.lum-terminal-err { color:var(--lum-danger); }
.lum-terminal-status { color:var(--lum-fg-muted); }
.lum-terminal-input-row { display:flex; align-items:center; gap:8px; flex-shrink:0; padding:7px 10px; border-top:1px solid var(--lum-border); background:var(--lum-bg-surface); }
.lum-terminal-input-prompt { color:var(--lum-accent); font-family:var(--lum-font-mono); }
.lum-terminal-input-row input { flex:1; min-width:0; border:0; outline:0; background:transparent; color:var(--lum-fg); font:var(--lum-text-xs) var(--lum-font-mono); }
.lum-terminal-input-row button { border:1px solid var(--lum-border); border-radius:var(--lum-radius-sm); background:var(--lum-bg-hover); color:var(--lum-fg); padding:3px 9px; cursor:pointer; }
.lum-terminal-input-row button:disabled { opacity:.45; cursor:default; }
</style>