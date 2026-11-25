<script setup lang="ts">
/**
 * Host for an extension-provided HTML panel.
 *
 * The iframe is sandboxed with `allow-scripts` only: extension code cannot
 * reach the Vue app, Tauri IPC, or the parent DOM. Communication happens via
 * a narrow postMessage protocol and the shared Luma event bus.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onBeforeUnmount, onMounted, ref } from "vue";
import {
  extensionAssetUrl,
  EXTENSION_FRAME_SOURCE,
  FRAME_TO_HOST,
  HOST_TO_FRAME,
  WORKSPACE_CONTEXT_EVENT,
  type ActiveWorkspaceInfo,
  type ExtensionToolWindowContribution,
  type FrontendContribution,
} from "~/vue/composables/extensionUI";
import { fireEvent, registerEventHandler } from "~/vue/composables/useEvents";

const props = defineProps<{
  contribution: FrontendContribution;
  tool?: ExtensionToolWindowContribution;
}>();

interface FrameMessage {
  source?: unknown;
  direction?: unknown;
  event?: unknown;
  payload?: unknown;
  cancelled?: unknown;
}

const frame = ref<HTMLIFrameElement | null>(null);
const iframeSrc = ref("");
const error = ref("");
const allowedEvents = new Set(props.contribution.events);
const disposers: Array<() => Promise<void>> = [];
let mounted = false;
let unlistenWorkspace: UnlistenFn | undefined;

function postToFrame(event: string, payload: unknown, cancelled = false) {
  // Opaque-origin sandboxed frames require "*" as targetOrigin.
  frame.value?.contentWindow?.postMessage(
    {
      source: EXTENSION_FRAME_SOURCE,
      direction: HOST_TO_FRAME,
      event,
      payload,
      cancelled,
    },
    "*",
  );
}

async function sendWorkspaceContext() {
  try {
    const workspace = await invoke<ActiveWorkspaceInfo | null>("get_active_workspace");
    postToFrame(WORKSPACE_CONTEXT_EVENT, workspace);
  } catch (cause) {
    console.warn("[extensions] Unable to read active workspace for extension frame", cause);
  }
}

function onFrameMessage(message: MessageEvent) {
  if (message.source !== frame.value?.contentWindow) return;
  const data = message.data as FrameMessage | null;
  if (!data || typeof data !== "object") return;
  if (data.source !== EXTENSION_FRAME_SOURCE || data.direction !== FRAME_TO_HOST) return;
  if (typeof data.event !== "string" || !allowedEvents.has(data.event)) return;

  void fireEvent(data.event, data.payload ?? null, data.cancelled === true);
}

onMounted(async () => {
  mounted = true;
  window.addEventListener("message", onFrameMessage);

  try {
    unlistenWorkspace = await listen<ActiveWorkspaceInfo | null>(
      "workspace-opened",
      (event) => postToFrame(WORKSPACE_CONTEXT_EVENT, event.payload),
    );
  } catch (cause) {
    console.warn("[extensions] Unable to watch workspace changes for extension frame", cause);
  }

  if (!props.contribution.entry) {
    error.value = "This extension does not declare a frontend entry page.";
  } else {
    const url = await extensionAssetUrl(props.contribution.extension, props.contribution.entry);
    if (!mounted) return;
    if (url) iframeSrc.value = url;
    else error.value = `Frontend entry '${props.contribution.entry}' was not found.`;
  }

  for (const event of allowedEvents) {
    const dispose = await registerEventHandler(event, {
      handler: (payload, meta) => {
        postToFrame(event, payload, meta.cancelled);
      },
    });
    if (!mounted) {
      void dispose();
      continue;
    }
    disposers.push(dispose);
  }

  if (mounted) void sendWorkspaceContext();
});

onBeforeUnmount(() => {
  mounted = false;
  window.removeEventListener("message", onFrameMessage);
  unlistenWorkspace?.();
  for (const dispose of disposers.splice(0).reverse()) void dispose();
});
</script>

<template>
  <div class="extension-frame-wrap">
    <p v-if="error" class="extension-frame-error">{{ error }}</p>
    <iframe
      v-else-if="iframeSrc"
      ref="frame"
      :src="iframeSrc"
      :title="tool?.title ?? `${contribution.extension} extension panel`"
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
      class="extension-frame"
      @load="sendWorkspaceContext"
    />
    <div v-else class="extension-frame-loading">Loading extension UI…</div>
  </div>
</template>

<style scoped>
.extension-frame-wrap {
  position: relative;
  flex: 1 1 auto;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.extension-frame {
  display: block;
  width: 100%;
  height: 100%;
  border: 0;
  background: var(--lum-bg-surface);
}

.extension-frame-error,
.extension-frame-loading {
  margin: 0;
  padding: var(--lum-space-4);
  color: var(--lum-fg-muted);
  font-size: var(--lum-text-sm);
}
</style>
