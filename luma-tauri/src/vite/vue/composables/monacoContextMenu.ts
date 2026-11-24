/**
 * Bridges Monaco's own EditorContext menu into the Luma menu renderer.
 *
 * Monaco does not expose its context menu publicly, but the editor's
 * contextmenu contribution holds the exact menu service, scoped context key
 * service and keybinding service it uses to build the native menu. Reading
 * those gives us the full action list with the correct `when` filtering,
 * order, groups, checked state and platform keybinding labels — without
 * importing Monaco internals (which can create duplicate singletons in Vite).
 */
import type * as monaco from "monaco-editor";
import type { MenuItem } from "./menus";

interface MonacoActionLike {
  id: string;
  label: string;
  enabled?: boolean;
  checked?: boolean;
  actions?: MonacoActionLike[];
  run: (...args: unknown[]) => Promise<unknown> | unknown;
}

interface MonacoContextController {
  _menuService: {
    getMenuActions(
      menuId: unknown,
      contextKeyService: unknown,
      options?: unknown,
    ): Array<[string, MonacoActionLike[]]>;
  };
  _contextKeyService: unknown;
  _keybindingFor?: (action: { id: string }) => { getLabel(): string | null } | null | undefined;
}

const SEPARATOR_ID = "vs.actions.separator";

/**
 * Resolve all actions Monaco would show for a right-click in this editor.
 * Call this at right-click time so context-sensitive entries are accurate.
 */
export function getMonacoContextMenuItems(
  editor: monaco.editor.IStandaloneCodeEditor | null,
  event?: MouseEvent,
): MenuItem[] {
  const model = editor?.getModel();
  if (!editor || !model) return [];

  // Monaco normally does this before opening its own menu. Because the native
  // menu is disabled, reproduce it here so actions operate on the clicked
  // position rather than a stale selection.
  if (event) {
    editor.focus();
    const target = editor.getTargetAtClientPoint(event.clientX, event.clientY);
    const position = target?.position;
    if (position && !editor.getSelection()?.containsPosition(position)) {
      editor.setPosition(position);
    }
  }

  const controller = editor.getContribution("editor.contrib.contextmenu") as unknown as MonacoContextController | null;
  const menuId = (editor as unknown as { contextMenuId?: unknown }).contextMenuId;
  if (!controller?._menuService?.getMenuActions || !menuId) return [];

  let groups: Array<[string, MonacoActionLike[]]>;
  try {
    groups = controller._menuService.getMenuActions(menuId, controller._contextKeyService, { arg: model.uri });
  } catch (error) {
    console.warn("Unable to read Monaco's context menu", error);
    return [];
  }

  const items: MenuItem[] = [];
  for (const [, actions] of groups) {
    if (!actions?.length) continue;
    if (items.length) items.push({ separator: true });
    for (const action of actions) {
      const item = toMenuItem(action, controller);
      if (items[items.length - 1]?.separator && item?.separator) continue;
      if (item) items.push(item);
    }
  }
  while (items.length && items[items.length - 1].separator) items.pop();
  return items;
}

function toMenuItem(action: MonacoActionLike, controller: MonacoContextController): MenuItem | null {
  if (!action || action.id === SEPARATOR_ID) return { separator: true };

  const children = action.actions
    ?.map((child) => toMenuItem(child, controller))
    .filter((child): child is MenuItem => child !== null);

  const keybinding = controller._keybindingFor?.(action)?.getLabel?.() ?? null;

  return {
    id: action.id,
    label: action.label || action.id,
    shortcut: keybinding ?? undefined,
    checked: action.checked === true ? true : undefined,
    disabled: action.enabled === false,
    children: children?.length ? children : undefined,
    action: async () => {
      try {
        await action.run();
      } catch (error) {
        console.warn(`Monaco action "${action.id}" failed`, error);
      }
    },
  };
}
