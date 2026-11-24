/**
 * Menu registry.
 *
 * Menus never contain behaviour of their own: they reference action ids from
 * `actions.ts`. This lets a context menu, the titlebar and the command palette
 * all show the same commands, and lets future extensions contribute items.
 */
import { shallowRef } from "vue";
import { getAction, getActionContext, isActionAvailable, runAction, type ActionContext } from "./actions";

export const Menus = {
  editorContext: "editor/context",
  editorTabContext: "editor/tab/context",
  explorerContext: "explorer/context",
  titlebarMain: "titlebar/main",
  titlebarProject: "titlebar/project",
  titlebarVcs: "titlebar/vcs",
} as const;

export type MenuLocation = string;

export interface MenuItem {
  /** Action id backing this item. */
  id?: string;
  label?: string;
  shortcut?: string;
  disabled?: boolean;
  danger?: boolean;
  checked?: boolean;
  separator?: boolean;
  /** SVG path data. */
  icon?: string;
  category?: string;
  /** Nested items rendered as a submenu (e.g. Monaco's "Copy As" / "Peek"). */
  children?: MenuItem[];
  /** Inline fallback for menus that are not backed by a global action. */
  action?: () => void | Promise<void>;
}

export interface MenuCommandEntry {
  commandId: string;
  /** Items are sorted by group, then order; a separator is inserted between groups. */
  group?: string;
  order?: number;
  when?: (context: ActionContext) => boolean;
  /** Override the action's label for this menu. */
  label?: string;
}

export interface MenuSeparatorEntry {
  separator: true;
  group?: string;
  order?: number;
  when?: (context: ActionContext) => boolean;
}

export type MenuEntry = MenuCommandEntry | MenuSeparatorEntry;

interface StoredEntry {
  location: MenuLocation;
  entry: MenuEntry;
  token: number;
}

const contributions = shallowRef<readonly StoredEntry[]>([]);
let nextToken = 1;

/** Add entries to a named menu. Returns an unregister function. */
export function registerMenu(location: MenuLocation, entries: MenuEntry[]): () => void {
  const tokens = entries.map(() => nextToken++);
  contributions.value = [
    ...contributions.value,
    ...entries.map((entry, index) => ({ location, entry, token: tokens[index] })),
  ];
  return () => {
    contributions.value = contributions.value.filter((stored) => !tokens.includes(stored.token));
  };
}

export function registerMenuCommand(
  location: MenuLocation,
  commandId: string,
  options: Omit<MenuCommandEntry, "commandId"> = {},
): () => void {
  return registerMenu(location, [{ commandId, ...options }]);
}

/** Resolve a menu location into renderable items using the current action context. */
export function getMenuItems(location: MenuLocation, overrides: Partial<ActionContext> = {}): MenuItem[] {
  const context = getActionContext({
    ...overrides,
    event: overrides.event ?? null,
    target: overrides.target ?? null,
  });

  const sorted = contributions.value
    .filter((stored) => stored.location === location)
    .sort((left, right) => {
      const groupOrder = (left.entry.group ?? "").localeCompare(right.entry.group ?? "");
      return groupOrder || (left.entry.order ?? 0) - (right.entry.order ?? 0);
    });

  const items: MenuItem[] = [];
  let lastGroup: string | undefined;

  for (const { entry } of sorted) {
    if (entry.when && !entry.when(context)) continue;

    if ("separator" in entry) {
      if (items.length && !items[items.length - 1].separator) items.push({ separator: true });
      continue;
    }

    const action = getAction(entry.commandId);
    if (!action || !isActionAvailable(action, context)) continue;

    const group = entry.group ?? "";
    if (lastGroup !== undefined && group !== lastGroup && items.length && !items[items.length - 1].separator) {
      items.push({ separator: true });
    }
    lastGroup = group;

    items.push({
      id: action.id,
      label: entry.label ?? action.label,
      shortcut: action.shortcut,
      icon: action.icon,
      danger: action.danger,
      checked: action.checked?.(context),
      disabled: action.enabled ? !action.enabled(context) : false,
      // Pass through event context so menu actions can act on what was clicked.
      action: () => { void runAction(action.id, { event: context.event, target: context.target }); },
    });
  }

  while (items.length && items[items.length - 1].separator) items.pop();
  return items;
}
