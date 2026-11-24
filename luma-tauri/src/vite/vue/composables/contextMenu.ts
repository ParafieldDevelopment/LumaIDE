/**
 * Delegated context menus for the whole IDE.
 *
 * Elements register either a builder function (legacy) or the name of a menu
 * contributed to `menus.ts`. Right-clicks are handled once at document level,
 * so Monaco, the explorer, tabs and custom UI all use the same popup renderer.
 */
import type { App, Directive } from "vue";
import { getActionContext, type ActionContext } from "./actions";
import { getMenuItems, type MenuItem } from "./menus";

export type { MenuItem as ContextMenuItem };

export type ContextMenuBuilder = (
  event: MouseEvent,
  target: HTMLElement,
) => MenuItem[] | null | undefined | Promise<MenuItem[] | null | undefined>;

/** A menu location string resolves through the menu registry. */
export type ContextMenuBinding = ContextMenuBuilder | string;

export interface MenuAnchor {
  left: number;
  top: number;
}

export interface ContextMenuRequest {
  event?: MouseEvent;
  target: HTMLElement;
  items: MenuItem[];
  anchor?: MenuAnchor;
  context?: ActionContext;
}

const builders = new WeakMap<Element, ContextMenuBinding>();
const listeners = new Set<(request: ContextMenuRequest) => void>();
let installed = false;
let latestRequest = 0;

/** Register a context menu for an element. The closest registered element wins. */
export function registerContextMenu(element: Element, binding: ContextMenuBinding): () => void {
  builders.set(element, binding);
  return (): void => {
    if (builders.get(element) === binding) builders.delete(element);
  };
}

function findBinding(event: MouseEvent): [HTMLElement, ContextMenuBinding] | undefined {
  for (const node of event.composedPath()) {
    if (!(node instanceof HTMLElement)) continue;
    const binding = builders.get(node);
    if (binding) return [node, binding];
  }
  return undefined;
}

export function notifyContextMenu(request: ContextMenuRequest): void {
  if (!request.items.length) return;
  for (const listener of [...listeners]) listener(request);
}

/** Open a menu programmatically, anchored to an element (e.g. a toolbar button). */
export function showContextMenu(
  source: MouseEvent | HTMLElement,
  items: MenuItem[],
  contextOverrides: Partial<ActionContext> = {},
): void {
  if (!items.length) return;
  let event: MouseEvent | undefined;
  let target: HTMLElement;
  let anchor: MenuAnchor | undefined;

  if (source instanceof HTMLElement) {
    target = source;
    const rect = source.getBoundingClientRect();
    anchor = { left: rect.left, top: rect.bottom };
  } else {
    event = source;
    target = source.target instanceof HTMLElement ? source.target : document.body;
    anchor = { left: source.clientX, top: source.clientY };
  }

  notifyContextMenu({
    event,
    target,
    items,
    anchor,
    context: getActionContext({
      ...contextOverrides,
      event: event ?? null,
      target,
    }),
  });
}

async function handleContextMenu(event: MouseEvent): Promise<void> {
  const match = findBinding(event);
  if (!match || listeners.size === 0) return;

  // Stop Monaco (or any other widget) from showing its own native menu.
  event.preventDefault();
  event.stopPropagation();

  const requestId = ++latestRequest;
  const [target, binding] = match;
  const context = getActionContext({ event, target });
  const items = typeof binding === "string"
    ? getMenuItems(binding, { event, target })
    : await binding(event, target);

  if (items?.length && requestId === latestRequest) {
    notifyContextMenu({ event, target, items, context });
  }
}

/** Use as `v-context-menu="menuLocationOrBuilder"` on the target element. */
export const contextMenuDirective: Directive<HTMLElement, ContextMenuBinding> = {
  mounted(element, binding) {
    if (typeof binding.value === "function" || typeof binding.value === "string") {
      registerContextMenu(element, binding.value);
    }
  },
  updated(element, binding) {
    if (typeof binding.value === "function" || typeof binding.value === "string") {
      registerContextMenu(element, binding.value);
    } else {
      builders.delete(element);
    }
  },
  unmounted(element) {
    builders.delete(element);
  },
};

/** Install once per Vue app to enable `v-context-menu` and delegated handling. */
export function installContextMenu(app: App): void {
  app.directive("context-menu", contextMenuDirective);
  if (!installed && typeof document !== "undefined") {
    document.addEventListener("contextmenu", handleContextMenu, true);
    installed = true;
  }
}

/** Subscribe to menu requests. Multiple renderers can listen at once. */
export function onContextMenuRequest(listener: (request: ContextMenuRequest) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
