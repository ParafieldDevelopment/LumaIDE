/** Bridge between the Vue frontend and the shared Luma event bus. */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onBeforeUnmount } from "vue";

/** Tauri event used by the host to mirror bus events into the frontend. */
export const LUMA_EVENT = "luma-event";

export type EventPriority = "lowest" | "low" | "normal" | "high" | "highest" | "monitor";

export interface LumaEventEnvelope {
  subscriptionId: number;
  name: string;
  payload: unknown;
  cancelled: boolean;
}

export interface EventMeta {
  name: string;
  /** Cancellation state at the moment the host emitted this event. */
  cancelled: boolean;
}

export type LumaEventHandler<T = unknown> = (payload: T, meta: EventMeta) => void;

export interface RegisterOptions<T = unknown> {
  priority?: EventPriority;
  ignoreCancelled?: boolean;
  handler: LumaEventHandler<T>;
}

export interface FireResult {
  cancelled: boolean;
  handlersCalled: number;
}

interface Subscription {
  name: string;
  handler: LumaEventHandler<unknown>;
}

const PRIORITIES: Record<EventPriority, number> = {
  lowest: 0,
  low: 1,
  normal: 2,
  high: 3,
  highest: 4,
  monitor: 5,
};

const subscriptions = new Map<number, Subscription>();
let listenerPromise: Promise<UnlistenFn> | null = null;

async function ensureListener(): Promise<void> {
  if (!listenerPromise) {
    listenerPromise = listen<LumaEventEnvelope>(LUMA_EVENT, (event) => {
      const subscription = subscriptions.get(event.payload.subscriptionId);
      if (!subscription) return;
      try {
        subscription.handler(event.payload.payload, {
          name: event.payload.name,
          cancelled: event.payload.cancelled,
        });
      } catch (error) {
        console.error(`[luma-events] handler for "${event.payload.name}" failed`, error);
      }
    });
  }
  await listenerPromise;
}

/** Subscribe to any named bus event. Returns a function that removes the handler. */
export async function registerEventHandler<T = unknown>(
  name: string,
  options: RegisterOptions<T>,
): Promise<() => Promise<void>> {
  await ensureListener();
  const id = await invoke<number>("event_register", {
    name,
    priority: PRIORITIES[options.priority ?? "normal"],
    ignoreCancelled: options.ignoreCancelled ?? false,
  });
  subscriptions.set(id, {
    name,
    handler: options.handler as unknown as LumaEventHandler<unknown>,
  });
  return () => unregisterEventHandler(id);
}

/** Remove a subscription created with {@link registerEventHandler}. */
export async function unregisterEventHandler(id: number): Promise<void> {
  subscriptions.delete(id);
  await invoke("event_unregister", { id });
}

/**
 * Fire a named event from the frontend. Extensions and Rust handlers receive
 * it like any other bus event. `cancelled` seeds the event's initial state.
 */
export async function fireEvent(
  name: string,
  payload: unknown = null,
  cancelled = false,
): Promise<FireResult> {
  return await invoke<FireResult>("event_fire", { name, payload, cancelled });
}

/** Notify the bus about editor actions that only the frontend knows about. */
export async function notifyFileOpened(path: string): Promise<void> {
  await invoke("notify_file_opened", { path });
}

export async function notifyFileClosed(path: string): Promise<void> {
  await invoke("notify_file_closed", { path });
}

export async function notifyFileActivated(path: string | null): Promise<void> {
  await invoke("notify_file_activated", { path });
}

/** Composable variant that disposes subscriptions when the component unmounts. */
export function useLumaEvents() {
  const disposers: Array<() => Promise<void>> = [];

  onBeforeUnmount(() => {
    for (const dispose of disposers) void dispose();
    disposers.length = 0;
  });

  return {
    register<T = unknown>(name: string, options: RegisterOptions<T>) {
      const pending = registerEventHandler<T>(name, options);
      void pending.then((dispose) => disposers.push(dispose));
      return pending;
    },
    fire: fireEvent,
    unregister: unregisterEventHandler,
  };
}
