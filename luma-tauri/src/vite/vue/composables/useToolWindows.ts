/**
 * Tool windows (JetBrains-style panels).
 *
 * A tool window is a panel docked to the left, right or bottom of the editor.
 * Each anchor shows at most one tool at a time. Tools are toggled from the
 * stripe buttons (LumToolStripe) or with a hotkey.
 *
 * To add a panel, see vue/toolwindows/README.md.
 */
import { computed, reactive, shallowRef, watch, type Component } from "vue";

export type ToolAnchor = "left" | "right" | "bottom";

export interface ToolWindowDef {
  /** Unique id, e.g. "terminal". Also used for persistence. */
  id: string;
  /** Shown in the tooltip and in the panel header. */
  title: string;
  /** Which edge the panel docks to. */
  anchor: ToolAnchor;
  /** Icon component for the stripe button. */
  icon: Component;
  /**
   * The panel component. It receives `workspacePath: string` and should emit
   * `hide`. Wrap its content in <LumToolWindow> for the standard header.
   */
  component: Component;
  /** Stripe ordering within its group (lower first). Default 100. */
  order?: number;
  /** e.g. "Alt+1", "Alt+F12", "Ctrl+Shift+K". */
  hotkey?: string;
  /** Open on first launch (only if the user never toggled this anchor). */
  defaultOpen?: boolean;
}

interface PersistedState {
  /** Active tool id per anchor. `null` = user closed it. Missing = never set. */
  active: Partial<Record<ToolAnchor, string | null>>;
  /** Panel size in px: width for left/right, height for bottom. */
  sizes: Record<ToolAnchor, number>;
}

const STORAGE_KEY = "luma.toolWindows.v1";

const DEFAULT_SIZES: Record<ToolAnchor, number> = {
  left: 260,
  right: 280,
  bottom: 240,
};

const SIZE_LIMITS: Record<ToolAnchor, { min: number; max: number }> = {
  left: { min: 180, max: 700 },
  right: { min: 180, max: 700 },
  bottom: { min: 120, max: 700 },
};

function load(): PersistedState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        active:
          parsed && typeof parsed.active === "object" && parsed.active
            ? parsed.active
            : {},
        sizes: { ...DEFAULT_SIZES, ...(parsed?.sizes ?? {}) },
      };
    }
  } catch {
    // Storage can be unavailable or corrupt; fall back to defaults.
  }
  return { active: {}, sizes: { ...DEFAULT_SIZES } };
}

function save() {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ active: state.active, sizes: state.sizes })
    );
  } catch {
    // Ignore: persistence is best-effort.
  }
}

const state = reactive<PersistedState>(load());
const registry = shallowRef<ToolWindowDef[]>([]);

let saveTimer: number | undefined;
watch(
  state,
  () => {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(save, 200);
  },
  { deep: true }
);

/** Register (or replace) a tool window. Safe to call more than once. */
export function registerToolWindow(def: ToolWindowDef) {
  const next = registry.value.filter((tool) => tool.id !== def.id);
  next.push(def);
  next.sort((a, b) => (a.order ?? 100) - (b.order ?? 100));
  registry.value = next;

  if (def.defaultOpen && state.active[def.anchor] === undefined) {
    state.active[def.anchor] = def.id;
  }
}

function matchesHotkey(event: KeyboardEvent, hotkey: string): boolean {
  const parts = hotkey.toLowerCase().split("+");
  const key = parts.pop();
  if (!key) return false;

  const wantAlt = parts.includes("alt");
  const wantCtrl = parts.includes("ctrl");
  const wantShift = parts.includes("shift");

  return (
    event.altKey === wantAlt &&
    event.ctrlKey === wantCtrl &&
    event.shiftKey === wantShift &&
    event.key.toLowerCase() === key
  );
}

export function useToolWindows() {
  const byAnchor = (anchor: ToolAnchor) =>
    registry.value.filter((tool) => tool.anchor === anchor);

  const activeTool = (anchor: ToolAnchor): ToolWindowDef | null => {
    const id = state.active[anchor];
    if (!id) return null;
    return registry.value.find((t) => t.id === id && t.anchor === anchor) ?? null;
  };

  const leftTools = computed(() => byAnchor("left"));
  const rightTools = computed(() => byAnchor("right"));
  const bottomTools = computed(() => byAnchor("bottom"));

  const leftTool = computed(() => activeTool("left"));
  const rightTool = computed(() => activeTool("right"));
  const bottomTool = computed(() => activeTool("bottom"));

  /** Ids of every tool that is currently shown. */
  const activeIds = computed(() =>
    [leftTool.value, rightTool.value, bottomTool.value]
      .filter((tool): tool is ToolWindowDef => tool !== null)
      .map((tool) => tool.id)
  );

  function toggle(id: string) {
    const tool = registry.value.find((t) => t.id === id);
    if (!tool) return;
    state.active[tool.anchor] = state.active[tool.anchor] === id ? null : id;
  }

  function open(id: string) {
    const tool = registry.value.find((t) => t.id === id);
    if (tool) state.active[tool.anchor] = id;
  }

  function close(anchor: ToolAnchor) {
    state.active[anchor] = null;
  }

  /** Hide every panel (JetBrains: Ctrl+Shift+F12). */
  function hideAll() {
    state.active.left = null;
    state.active.right = null;
    state.active.bottom = null;
  }

  function setSize(anchor: ToolAnchor, value: number) {
    const { min, max } = SIZE_LIMITS[anchor];
    // Never let a panel take more than 70% of the window.
    const viewport = anchor === "bottom" ? window.innerHeight : window.innerWidth;
    const upper = Math.min(max, Math.round(viewport * 0.7));
    state.sizes[anchor] = Math.round(Math.min(Math.max(value, min), upper));
  }

  /** Returns true if the event was a tool window hotkey (and was handled). */
  function handleHotkey(event: KeyboardEvent): boolean {
    if (event.ctrlKey && event.shiftKey && !event.altKey && event.key === "F12") {
      hideAll();
      return true;
    }
    for (const tool of registry.value) {
      if (tool.hotkey && matchesHotkey(event, tool.hotkey)) {
        toggle(tool.id);
        return true;
      }
    }
    return false;
  }

  return {
    leftTools,
    rightTools,
    bottomTools,
    leftTool,
    rightTool,
    bottomTool,
    activeIds,
    sizes: state.sizes,
    toggle,
    open,
    close,
    hideAll,
    setSize,
    handleHotkey,
  };
}
