/** Shared types and asset helpers for extension-contributed frontend UI. */

import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export interface ExtensionCommandContribution {
  id: string;
  label: string;
  category: string | null;
  keys: string | null;
  icon: string | null;
  /** Bus event fired when the command is invoked. */
  event: string;
}

export interface ExtensionMenuContribution {
  location: string;
  commandId: string;
  group: string | null;
  order: number | null;
}

export interface ExtensionToolWindowContribution {
  id: string;
  title: string;
  position: "left-top" | "left-bottom" | "right-top" | "right-bottom";
  icon: string | null;
  order: number | null;
  hotkey: string | null;
  defaultOpen: boolean;
}

export interface FrontendContribution {
  extension: string;
  /** HTML entry loaded into the sandboxed iframe, relative to the extension root. */
  entry: string | null;
  commands: ExtensionCommandContribution[];
  menus: ExtensionMenuContribution[];
  toolWindows: ExtensionToolWindowContribution[];
  /** Named bus events the iframe may send and receive. */
  events: string[];
}

export interface ActiveWorkspaceInfo {
  project_uuid: string;
  title: string;
  path: string;
  git_branch?: string;
  icon_path?: string;
}

/** Message envelope used between a sandboxed extension frame and the Vue host. */
export const EXTENSION_FRAME_SOURCE = "luma-extension-frame";
export const HOST_TO_FRAME = "host-to-iframe";
export const FRAME_TO_HOST = "iframe-to-host";

/** Control event (not a bus event) used to push workspace context into a frame. */
export const WORKSPACE_CONTEXT_EVENT = "luma:extension-ui:workspace";

/** Resolve an extension asset to a Tauri asset-protocol URL. */
export async function extensionAssetUrl(
  extension: string,
  path: string | null | undefined,
): Promise<string | null> {
  if (!path) return null;
  try {
    const absolute = await invoke<string | null>("extension_asset_path", { extension, path });
    return absolute ? convertFileSrc(absolute) : null;
  } catch (error) {
    console.warn(`[extensions] Unable to resolve asset '${path}' for ${extension}`, error);
    return null;
  }
}
