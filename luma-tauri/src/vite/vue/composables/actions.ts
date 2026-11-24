/**
 * Central action registry.
 *
 * Everything user-invokable (context menus, the command palette, titlebar menus
 * and keyboard shortcuts) is described here once as an `Action`. Menus and the
 * palette only reference action ids, so adding an action automatically makes it
 * available from every entry point (like VS Code commands / JetBrains actions).
 */
import { shallowRef } from "vue";
import type * as monaco from "monaco-editor";

export interface ActionContext {
  editor: monaco.editor.IStandaloneCodeEditor | null;
  event: Event | null;
  target: HTMLElement | null;
  workspace: { path: string; title?: string } | null;
  activeFile: { id: number; path: string; name: string; dirty: boolean } | null;
  [key: string]: unknown;
}

export interface Action {
  /** Stable id, e.g. `file.save` (namespace it the way VS Code does). */
  id: string;
  label: string;
  /** Palette grouping / search term, e.g. `File`, `Editor`, `View`. */
  category?: string;
  keywords?: string[];
  /** Human-readable key hint shown in menus and the palette. */
  shortcut?: string;
  /** One or more bindings separated by `|`, e.g. `Ctrl+Shift+P|F1`. */
  keys?: string;
  /** SVG path data for an optional 16px icon. */
  icon?: string;
  danger?: boolean;
  /** Whether the action exists in the current context. */
  when?: (context: ActionContext) => boolean;
  /** Whether an existing action can currently be invoked. */
  enabled?: (context: ActionContext) => boolean;
  /** For toggle actions: whether the option is currently on. */
  checked?: (context: ActionContext) => boolean;
  run: (context: ActionContext) => void | Promise<void>;
}

const emptyContext = (): ActionContext => ({
  editor: null,
  event: null,
  target: null,
  workspace: null,
  activeFile: null,
});

const registry = shallowRef<readonly Action[]>([]);
const currentContext = shallowRef<ActionContext>(emptyContext());

export function listActions(): readonly Action[] {
  return registry.value;
}

export function getAction(id: string): Action | undefined {
  return registry.value.find((action) => action.id === id);
}

/** Register (or replace) an action. Returns an unregister function. */
export function registerAction(action: Action): () => void {
  registry.value = [...registry.value.filter((existing) => existing.id !== action.id), action];
  return () => {
    registry.value = registry.value.filter((existing) => existing.id !== action.id);
  };
}

/** Register several actions at once. */
export function registerActions(actions: Action[]): () => void {
  const disposers = actions.map(registerAction);
  return () => disposers.forEach((dispose) => dispose());
}

export function updateActionContext(patch: Partial<ActionContext>): void {
  currentContext.value = { ...currentContext.value, ...patch };
}

export function resetActionContext(): void {
  currentContext.value = emptyContext();
}

export function getActionContext(overrides: Partial<ActionContext> = {}): ActionContext {
  return { ...currentContext.value, ...overrides };
}

export function isActionAvailable(action: Action, context: ActionContext = currentContext.value): boolean {
  if (action.when && !action.when(context)) return false;
  if (action.enabled && !action.enabled(context)) return false;
  return true;
}

export async function runAction(id: string, overrides: Partial<ActionContext> = {}): Promise<boolean> {
  const action = getAction(id);
  if (!action) return false;
  const context = getActionContext(overrides);
  if (!isActionAvailable(action, context)) return false;
  try {
    await action.run(context);
    return true;
  } catch (error) {
    console.error(`Action "${id}" failed`, error);
    return false;
  }
}

const keyAliases: Record<string, string> = {
  esc: "escape",
  space: " ",
  spacebar: " ",
  del: "delete",
  ins: "insert",
  cmd: "meta",
  command: "meta",
  option: "alt",
};

function normalizeKey(key: string): string {
  const lower = key.toLowerCase();
  return keyAliases[lower] ?? lower;
}

function chordMatches(event: KeyboardEvent, chord: string): boolean {
  const parts = chord.toLowerCase().split("+").map((part) => part.trim());
  const key = parts.pop();
  if (!key) return false;

  const wantsPrimary = parts.some((part) => ["ctrl", "cmd", "command", "meta", "mod"].includes(part));
  if (wantsPrimary !== (event.ctrlKey || event.metaKey)) return false;
  if (parts.includes("alt") !== event.altKey) return false;
  if (parts.includes("shift") !== event.shiftKey) return false;
  return normalizeKey(key) === normalizeKey(event.key);
}

export function matchesKeyBinding(event: KeyboardEvent, binding: string): boolean {
  return binding.split("|").some((chord) => chordMatches(event, chord));
}

/**
 * Dispatch a keydown to the most specific registered binding.
 * Returns true when an action consumed the event.
 */
export function handleActionKeydown(event: KeyboardEvent): boolean {
  if (event.repeat) return false;
  const context = getActionContext({ event, target: event.target instanceof HTMLElement ? event.target : null });
  const candidates = registry.value
    .filter((action) => action.keys && isActionAvailable(action, context))
    .sort((a, b) => (b.keys?.split("|")[0].split("+").length ?? 0) - (a.keys?.split("|")[0].split("+").length ?? 0));

  for (const action of candidates) {
    if (!action.keys || !matchesKeyBinding(event, action.keys)) continue;
    event.preventDefault();
    event.stopPropagation();
    void runAction(action.id, { event, target: context.target });
    return true;
  }
  return false;
}

/** Subsequence search with a bonus for direct matches. Returns null when there is no match. */
export function scoreAction(
  action: Pick<Action, "label" | "category" | "keywords">,
  query: string,
): number | null {
  if (!query) return 1;
  const needle = query.toLowerCase().trim();
  if (!needle) return 1;

  const label = action.label.toLowerCase();
  const category = (action.category ?? "").toLowerCase();
  const haystack = `${label} ${category} ${(action.keywords ?? []).join(" ")}`.toLowerCase();
  const direct = haystack.indexOf(needle);
  if (direct >= 0) {
    const labelHit = label.indexOf(needle);
    return 1000 - direct + (labelHit >= 0 ? 200 - labelHit : 0);
  }

  let queryIndex = 0;
  let score = 0;
  let lastIndex = -1;
  for (let index = 0; index < label.length && queryIndex < needle.length; index++) {
    if (label[index] !== needle[queryIndex]) continue;
    score += lastIndex === index - 1 ? 10 : 3;
    if (index === 0 || label[index - 1] === " ") score += 5;
    lastIndex = index;
    queryIndex++;
  }
  return queryIndex === needle.length ? 200 + score - label.length * 0.1 : null;
}
