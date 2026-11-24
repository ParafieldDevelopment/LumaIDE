/**
 * Project-scoped JetBrains-style tool windows.
 * Buttons live in four corner groups while panels open on the selected side.
 */
import { computed, reactive, shallowRef, watch, type Component } from "vue";
import { invoke } from "@tauri-apps/api/core";

/** Which dock a tool opens in. *-top buttons open a side dock; *-bottom buttons open a bottom dock. */
export type ToolSide = "left" | "right" | "bottom" | "bottom-left" | "bottom-right";
export type ToolPosition = "left-top" | "left-bottom" | "right-top" | "right-bottom";

export interface ToolWindowDef {
  id: string;
  title: string;
  position: ToolPosition;
  icon: Component;
  component: Component;
  order?: number;
  hotkey?: string;
  defaultOpen?: boolean;
}

interface PersistedState {
  version: number;
  positions: Record<string, ToolPosition>;
  order: Record<ToolPosition, string[]>;
  active: Record<ToolSide, string | null>;
  sizes: Record<ToolSide, number>;
}

const positions: ToolPosition[] = ["left-top", "left-bottom", "right-top", "right-bottom"];
const sides: ToolSide[] = ["left", "right", "bottom", "bottom-left", "bottom-right"];
const DEFAULT_SIZES: Record<ToolSide, number> = {
  left: 280,
  right: 280,
  bottom: 260,
  "bottom-left": 400,
  "bottom-right": 400,
};
const LIMITS: Record<ToolSide, { min: number; max: number }> = {
  left: { min: 180, max: 700 },
  right: { min: 180, max: 700 },
  bottom: { min: 120, max: 700 },
  "bottom-left": { min: 200, max: 900 },
  "bottom-right": { min: 200, max: 900 },
};

function emptyState(): PersistedState {
  return {
    version: 5,
    positions: {},
    order: { "left-top": [], "left-bottom": [], "right-top": [], "right-bottom": [] },
    active: { left: null, right: null, bottom: null, "bottom-left": null, "bottom-right": null },
    sizes: { ...DEFAULT_SIZES },
  };
}

function sideOf(position: ToolPosition): ToolSide {
  if (position === "left-bottom") return "bottom-left";
  if (position === "right-bottom") return "bottom-right";
  return position.startsWith("left-") ? "left" : "right";
}

function sanitize(raw: unknown): PersistedState {
  const next = emptyState();
  if (!raw || typeof raw !== "object") return next;
  const data = raw as Partial<PersistedState> & {
    anchors?: Record<string, unknown>;
    order?: Record<string, unknown>;
    active?: Record<string, unknown>;
    sizes?: Record<string, unknown>;
  };

  if ((data.version === 4 || data.version === 5) && data.positions && typeof data.positions === "object") {
    for (const [id, position] of Object.entries(data.positions)) {
      if (positions.includes(position as ToolPosition)) next.positions[id] = position as ToolPosition;
    }
    if (data.order && typeof data.order === "object") {
      for (const position of positions) {
        const list = data.order[position];
        if (Array.isArray(list)) next.order[position] = [...new Set(list.filter((id): id is string => typeof id === "string"))];
      }
    }
    if (data.active && typeof data.active === "object") {
      for (const side of sides) {
        const id = data.active[side];
        next.active[side] = typeof id === "string" ? id : null;
      }
    }
    if (data.sizes && typeof data.sizes === "object") {
      for (const side of sides) {
        const value = data.sizes[side];
        if (typeof value === "number" && Number.isFinite(value)) next.sizes[side] = value;
      }
    }
    return next;
  }

  // Older layouts (v3 and earlier) saved the previous default button placement.
  // Drop their positions so the new defaults apply; keep only dock sizes.
  if (data.sizes && typeof data.sizes === "object") {
    for (const side of sides) {
      const value = data.sizes[side];
      if (typeof value === "number" && Number.isFinite(value)) next.sizes[side] = value;
    }
  }
  return next;
}

const state = reactive<PersistedState>(emptyState());
const registry = shallowRef<ToolWindowDef[]>([]);
let projectPath = "";
let loadGeneration = 0;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
let ready = false;
let dirty = false;

function replaceState(next: PersistedState) {
  state.version = 5;
  state.positions = next.positions;
  for (const position of positions) state.order[position] = next.order[position];
  for (const side of sides) {
    state.active[side] = next.active[side];
    state.sizes[side] = next.sizes[side];
  }
}

async function save() {
  if (!ready || !projectPath || !dirty) return;
  dirty = false;
  try {
    await invoke("save_tool_window_layout", { workspacePath: projectPath, layout: JSON.stringify(state) });
  } catch (error) {
    dirty = true;
    console.warn("Unable to save project tool-window layout", error);
  }
}

function scheduleSave() {
  if (!ready) return;
  dirty = true;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = undefined;
    void save();
  }, 250);
}
watch(state, scheduleSave, { deep: true });

export async function setToolWindowWorkspace(path: string) {
  const generation = ++loadGeneration;
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = undefined;
  }
  await save();
  ready = false;
  projectPath = path;
  let next = emptyState();

  if (path) {
    try {
      const raw = await invoke<string | null>("load_tool_window_layout", { workspacePath: path });
      if (raw) next = sanitize(JSON.parse(raw));
    } catch (error) {
      console.warn("Unable to load project tool-window layout", error);
    }
  }
  if (generation !== loadGeneration) return;

  replaceState(next);
  for (const tool of registry.value) {
    if (!state.positions[tool.id]) state.positions[tool.id] = tool.position;
    const position = state.positions[tool.id];
    if (!state.order[position].includes(tool.id)) state.order[position].push(tool.id);

    const side = sideOf(position);
    if (tool.defaultOpen && state.active[side] === null) state.active[side] = tool.id;
  }
  ready = true;
  dirty = false;
}

export function registerToolWindow(def: ToolWindowDef) {
  registry.value = [...registry.value.filter((tool) => tool.id !== def.id), def];
  if (!state.positions[def.id]) state.positions[def.id] = def.position;
  const position = state.positions[def.id];
  if (!state.order[position].includes(def.id)) state.order[position].push(def.id);
  const side = sideOf(position);
  if (def.defaultOpen && state.active[side] === null) state.active[side] = def.id;
}

function ordered(position: ToolPosition) {
  return state.order[position]
    .map((id) => registry.value.find((tool) => tool.id === id))
    .filter((tool): tool is ToolWindowDef => !!tool);
}

function activeTool(side: ToolSide): ToolWindowDef | null {
  const id = state.active[side];
  if (!id) return null;
  const tool = registry.value.find((item) => item.id === id);
  return tool && sideOf(state.positions[id] ?? tool.position) === side ? tool : null;
}

function toggle(id: string) {
  const position = state.positions[id];
  if (!position) return;
  const side = sideOf(position);
  state.active[side] = state.active[side] === id ? null : id;
}

function open(id: string) {
  const position = state.positions[id];
  if (position) state.active[sideOf(position)] = id;
}

function close(side: ToolSide) {
  state.active[side] = null;
}

function move(id: string, target: ToolPosition, index?: number) {
  const source = state.positions[id];
  if (!source || !positions.includes(target)) return;
  const sourceSide = sideOf(source);
  const targetSide = sideOf(target);
  // Moving a button keeps its panel open only if it was already open.
  const wasActive = state.active[sourceSide] === id;

  state.order[source] = state.order[source].filter((item) => item !== id);
  if (sourceSide !== targetSide && state.active[sourceSide] === id) {
    state.active[sourceSide] = null;
  }
  state.positions[id] = target;

  const list = state.order[target].filter((item) => item !== id);
  const position = Math.max(0, Math.min(index ?? list.length, list.length));
  list.splice(position, 0, id);
  state.order[target] = list;
  if (wasActive) state.active[targetSide] = id;
}

function hideAll() {
  for (const side of sides) state.active[side] = null;
}

function setSize(side: ToolSide, value: number) {
  const limit = LIMITS[side];
  const viewport = side === "bottom" ? window.innerHeight : window.innerWidth;
  state.sizes[side] = Math.round(Math.min(Math.max(value, limit.min), Math.min(limit.max, viewport * 0.7)));
}

function matchesHotkey(event: KeyboardEvent, hotkey: string) {
  const parts = hotkey.toLowerCase().split("+");
  const key = parts.pop();
  if (!key) return false;
  return event.altKey === parts.includes("alt")
    && event.ctrlKey === parts.includes("ctrl")
    && event.shiftKey === parts.includes("shift")
    && event.key.toLowerCase() === key;
}

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

export function useToolWindows() {
  return {
    leftTopTools: computed(() => ordered("left-top")),
    leftBottomTools: computed(() => ordered("left-bottom")),
    rightTopTools: computed(() => ordered("right-top")),
    rightBottomTools: computed(() => ordered("right-bottom")),
    leftTool: computed(() => activeTool("left")),
    rightTool: computed(() => activeTool("right")),
    bottomLeftTool: computed(() => activeTool("bottom-left")),
    bottomRightTool: computed(() => activeTool("bottom-right")),
    activeIds: computed(() => sides.map((side) => activeTool(side)?.id).filter((id): id is string => !!id)),
    allTools: computed(() => registry.value),
    anchors: state.positions,
    sizes: state.sizes,
    toggle,
    open,
    close,
    move,
    hideAll,
    setSize,
    handleHotkey,
  };
}
