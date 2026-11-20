/**
 * Shared store for analyzer results ("Problems").
 * Anything (an extension, a language server bridge, a Tauri event listener)
 * can call `setProblems()`; the Analyzer panel just renders this list.
 */
import { computed, ref } from "vue";

export type Severity = "error" | "warning" | "info";

export interface Problem {
  id: string;
  severity: Severity;
  message: string;
  /** Absolute file path. */
  path: string;
  line?: number;
  column?: number;
  /** Who reported it, e.g. "lua-ls". */
  source?: string;
}

const problems = ref<Problem[]>([]);

const counts = computed(() => {
  const result: Record<Severity, number> = { error: 0, warning: 0, info: 0 };
  for (const problem of problems.value) result[problem.severity]++;
  return result;
});

export function useDiagnostics() {
  return {
    problems,
    counts,
    setProblems(next: Problem[]) {
      problems.value = next;
    },
    clearProblems() {
      problems.value = [];
    },
  };
}
