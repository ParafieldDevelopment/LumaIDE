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
let latestRequest = 0;

/**
 * Register a context-menu builder for an element.
 * The closest registered element in the event path handles each right-click.
 */
export function registerContextMenu(element: Element, builder: ContextMenuBuilder) {
  builders.set(element, builder);
  return (): void => {
    if (builders.get(element) === builder) builders.delete(element);
  };
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
  const match = findBuilder(event);
  const listener = requestListener;
  if (!match || !listener) return;

  event.preventDefault();
  event.stopPropagation();

  const requestId = ++latestRequest;
  const [target, builder] = match;
  const items = await builder(event, target);
  if (items?.length && requestId === latestRequest && requestListener === listener) {
    listener({ event, target, items });
  }
}

/** Use as `v-context-menu="(event, element) => items"` on the target element. */
export const contextMenuDirective: Directive<HTMLElement, ContextMenuBuilder> = {
  mounted(element, binding) {
    if (typeof binding.value === "function") registerContextMenu(element, binding.value);
  },
  updated(element, binding) {
    if (typeof binding.value === "function") registerContextMenu(element, binding.value);
    else builders.delete(element);
  },
  unmounted(element) {
    builders.delete(element);
  },
};

/** Install once per Vue app to enable `v-context-menu` and delegated event handling. */
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
