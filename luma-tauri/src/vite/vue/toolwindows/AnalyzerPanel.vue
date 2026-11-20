<script setup lang="ts">
import LumToolWindow from "../components/LumToolWindow.vue";
import { useDiagnostics } from "../composables/useDiagnostics";
const emit = defineEmits<{ (event: "hide"): void }>();
const { problems, counts, clearProblems } = useDiagnostics();
</script>
<template>
  <LumToolWindow title="Analyzer" @hide="emit('hide')">
    <div class="lum-analyzer">
      <div class="lum-analyzer-summary">
        <span class="error">{{ counts.error }} Errors</span>
        <span class="warning">{{ counts.warning }} Warnings</span>
        <span class="info">{{ counts.info }} Info</span>
        <button v-if="problems.length" @click="clearProblems">Clear</button>
      </div>
      <div v-if="!problems.length" class="lum-analyzer-empty">
        <p>No diagnostics reported.</p>
        <span>Analyzer results will appear here when connected language or analysis services report issues.</span>
      </div>
      <div v-else class="lum-analyzer-list">
        <div v-for="problem in problems" :key="problem.id" class="lum-analyzer-row" :class="problem.severity">
          <span class="lum-analyzer-severity">{{ problem.severity }}</span>
          <div class="lum-analyzer-detail">
            <div>{{ problem.message }}</div>
            <small>{{ problem.path }}<template v-if="problem.line">:{{ problem.line }}<template v-if="problem.column">:{{ problem.column }}</template></template></small>
            <small v-if="problem.source">Source: {{ problem.source }}</small>
          </div>
        </div>
      </div>
    </div>
  </LumToolWindow>
</template>
<style scoped>
.lum-analyzer { flex:1; min-height:0; overflow:auto; color:var(--lum-fg); font-size:var(--lum-text-sm); }
.lum-analyzer-summary { display:flex; gap:14px; align-items:center; padding:8px 12px; border-bottom:1px solid var(--lum-border-muted); flex-wrap:wrap; }
.lum-analyzer-summary span { color:var(--lum-fg-muted); }
.lum-analyzer-summary .error { color:var(--lum-danger); }
.lum-analyzer-summary .warning { color:var(--lum-warning, #d99a32); }
.lum-analyzer-summary button { margin-left:auto; border:0; background:transparent; color:var(--lum-fg-muted); cursor:pointer; }
.lum-analyzer-empty { display:flex; flex-direction:column; align-items:center; justify-content:center; min-height:140px; padding:20px; text-align:center; color:var(--lum-fg-muted); gap:8px; }
.lum-analyzer-empty p { margin:0; color:var(--lum-fg); }
.lum-analyzer-empty span { max-width:460px; }
.lum-analyzer-row { display:flex; gap:10px; padding:9px 12px; border-bottom:1px solid var(--lum-border-muted); }
.lum-analyzer-severity { width:56px; text-transform:uppercase; font-size:10px; color:var(--lum-fg-muted); }
.lum-analyzer-row.error .lum-analyzer-severity { color:var(--lum-danger); }
.lum-analyzer-row.warning .lum-analyzer-severity { color:var(--lum-warning, #d99a32); }
.lum-analyzer-detail { min-width:0; overflow-wrap:anywhere; }
.lum-analyzer-detail small { display:block; color:var(--lum-fg-muted); margin-top:3px; }
</style>