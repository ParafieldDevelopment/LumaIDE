import type { App, Directive } from "vue";

export interface ContextMenuItem {
  label?: string;
  shortcut?: string;
  disabled?: boolean;
  danger?: boolean;
  separator?: boolean;
  action?: () => void | Promise<void>;
}

export type ContextMenuBuilder = (
  event: MouseEvent,
  target: HTMLElement,
) => ContextMenuItem[] | null | undefined | Promise<ContextMenuItem[] | null | undefined>;

export interface ContextMenuRequest {
  event: MouseEvent;
  target: HTMLElement;
  items: ContextMenuItem[];
}

const builders = new WeakMap<Element, ContextMenuBuilder>();
let requestListener: ((request: ContextMenuRequest) => void) | undefined;
let installed = false;

/** Register a builder imperatively when a component uses a template ref. */
export function registerContextMenu(element: Element, builder: ContextMenuBuilder) {
  builders.set(element, builder);
  return () => builders.delete(element);
}

function findBuilder(event: MouseEvent): [HTMLElement, ContextMenuBuilder] | undefined {
  for (const node of event.composedPath()) {
    if (!(node instanceof HTMLElement)) continue;
    const builder = builders.get(node);
    if (builder) return [node, builder];
  }
  return undefined;
}

async function handleContextMenu(event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();

  const match = findBuilder(event);
  if (!match || !requestListener) return;

  const [target, builder] = match;
  const items = await builder(event, target);
  if (items?.length) requestListener({ event, target, items });
}

export const contextMenuDirective: Directive<HTMLElement, ContextMenuBuilder> = {
  mounted(element, binding) {
    if (typeof binding.value === "function") builders.set(element, binding.value);
  },
  updated(element, binding) {
    if (typeof binding.value === "function") builders.set(element, binding.value);
    else builders.delete(element);
  },
  unmounted(element) {
    builders.delete(element);
  },
};

export function installContextMenu(app: App) {
  app.directive("context-menu", contextMenuDirective);
  if (!installed && typeof document !== "undefined") {
    document.addEventListener("contextmenu", handleContextMenu, true);
    installed = true;
  }
}

export function onContextMenuRequest(listener: (request: ContextMenuRequest) => void) {
  requestListener = listener;
  return () => {
    if (requestListener === listener) requestListener = undefined;
  };
}
