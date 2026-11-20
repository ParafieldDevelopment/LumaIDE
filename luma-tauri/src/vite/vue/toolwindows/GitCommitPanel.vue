<script setup lang="ts">
import { ref, onMounted } from "vue";
import LumToolWindow from "../components/LumToolWindow.vue";
import { runGit } from "../composables/useShell";
const props = defineProps<{ workspacePath: string }>();
const emit = defineEmits<{ (event: "hide"): void }>();
const status = ref("");
const diff = ref("");
const stagedDiff = ref("");
const message = ref("");
const loading = ref(false);
const committing = ref(false);
const error = ref("");
const result = ref("");
async function refresh() {
  if (loading.value) return;
  loading.value = true; error.value = "";
  try {
    const [s, d, staged] = await Promise.all([runGit(props.workspacePath, ["status", "--short", "--branch"]), runGit(props.workspacePath, ["diff", "--stat"]), runGit(props.workspacePath, ["diff", "--cached", "--stat"])]);
    status.value = s.stdout || s.stderr;
    diff.value = d.stdout || (d.stderr ? d.stderr : "Working tree clean");
    stagedDiff.value = staged.stdout || (staged.stderr ? staged.stderr : "No staged changes");
    if (s.code !== 0) error.value = s.stderr || "Git status failed";
  } catch (e) { error.value = String(e); }
  finally { loading.value = false; }
}
async function stageAll() {
  loading.value = true; error.value = "";
  try { const r = await runGit(props.workspacePath, ["add", "-A"]); if (r.code !== 0) throw new Error(r.stderr || "git add failed"); loading.value = false; await refresh(); }
  catch(e) { error.value = String(e); } finally { loading.value = false; }
}
async function commit() {
  if (!message.value.trim() || committing.value) return;
  committing.value = true; error.value = ""; result.value = "";
  try {
    const staged = await runGit(props.workspacePath, ["diff", "--cached", "--stat"]);
    if (!staged.stdout.trim()) throw new Error("No staged changes to commit. Stage changes first.");
    const r = await runGit(props.workspacePath, ["commit", "-m", message.value.trim()]);
    if (r.code !== 0) throw new Error(r.stderr || "Commit failed");
    result.value = r.stdout || "Commit created.";
    message.value = "";
    await refresh();
  } catch(e) { error.value = String(e); }
  finally { committing.value = false; }
}
onMounted(refresh);
</script>
<template>
  <LumToolWindow title="Git Commit" @hide="emit('hide')">
    <div class="lum-git-panel">
      <div class="lum-git-toolbar">
        <button @click="refresh" :disabled="loading">Refresh</button>
        <button @click="stageAll" :disabled="loading">Stage All</button>
      </div>
      <label class="lum-git-label">Changes</label>
      <pre class="lum-git-status">{{ status || "Loading status…" }}</pre>
      <label class="lum-git-label">Unstaged diff summary</label>
      <pre class="lum-git-diff">{{ diff || "No changes" }}</pre>
      <label class="lum-git-label">Staged diff summary</label>
      <pre class="lum-git-diff">{{ stagedDiff || "No staged changes" }}</pre>
      <div class="lum-git-message">
        <label class="lum-git-label" for="lum-commit-message">Commit message</label>
        <textarea id="lum-commit-message" v-model="message" rows="3" placeholder="Describe your changes"></textarea>
        <button class="lum-git-commit" @click="commit" :disabled="committing || loading || !message.trim()">Commit staged changes</button>
      </div>
      <p v-if="result" class="lum-git-success">{{ result }}</p>
      <p v-if="error" class="lum-git-error">{{ error }}</p>
    </div>
  </LumToolWindow>
</template>
<style scoped>
.lum-git-panel { flex:1; min-height:0; overflow:auto; padding:10px; color:var(--lum-fg); font-size:var(--lum-text-sm); }
.lum-git-toolbar { display:flex; gap:6px; margin-bottom:10px; }
.lum-git-toolbar button,.lum-git-commit { border:1px solid var(--lum-border); border-radius:var(--lum-radius-sm); background:var(--lum-bg-hover); color:var(--lum-fg); padding:5px 10px; cursor:pointer; }
.lum-git-toolbar button:disabled,.lum-git-commit:disabled { opacity:.45; cursor:default; }
.lum-git-label { display:block; margin:9px 0 4px; color:var(--lum-fg-muted); font-size:var(--lum-text-xs); }
.lum-git-status,.lum-git-diff { margin:0; padding:8px; background:var(--lum-bg-app); border:1px solid var(--lum-border-muted); border-radius:var(--lum-radius-sm); font:var(--lum-text-xs) var(--lum-font-mono); white-space:pre-wrap; overflow-wrap:anywhere; }
.lum-git-message textarea { width:100%; box-sizing:border-box; resize:vertical; border:1px solid var(--lum-border); border-radius:var(--lum-radius-sm); padding:8px; background:var(--lum-bg-app); color:var(--lum-fg); font:var(--lum-text-sm) var(--lum-font-mono); }
.lum-git-commit { width:100%; margin-top:8px; background:var(--lum-accent-muted); }
.lum-git-success { color:var(--lum-success, #45a36a); white-space:pre-wrap; }
.lum-git-error { color:var(--lum-danger); white-space:pre-wrap; overflow-wrap:anywhere; }
</style>