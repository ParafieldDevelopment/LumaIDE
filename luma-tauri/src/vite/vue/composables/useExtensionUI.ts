/**
 * Loads UI contributed by installed extensions into the existing Vue shell.
 *
 * Contributions are declarative data from `extension.json`; extensions do not
 * get arbitrary JavaScript execution in the main webview. Panels are rendered
 * in sandboxed iframes and communicate through the shared event bus.
 */

import { defineComponent, h, type Component } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { registerAction } from "./actions";
import { Menus, registerMenu } from "./menus";
import { registerToolWindow, unregisterToolWindow, useToolWindows } from "./useToolWindows";
import { fireEvent } from "./useEvents";
import ExtensionFrame from "../components/ExtensionFrame.vue";
import {
  extensionAssetUrl,
  type FrontendContribution,
} from "./extensionUI";

const DEFAULT_ICON_PATH = "M4 4h16v16H4zM8 8h8v8H8z";

async function iconComponent(
  contribution: FrontendContribution,
  path: string | null,
): Promise<Component> {
  const src = await extensionAssetUrl(contribution.extension, path);
  return defineComponent({
    name: `ExtensionIcon_${contribution.extension}_${path ?? "default"}`,
    setup: () => () =>
      src
        ? h("img", {
            src,
            width: 16,
            height: 16,
            style: { objectFit: "contain", display: "block" },
            alt: "",
          })
        : h(
            "svg",
            {
              viewBox: "0 0 24 24",
              width: 16,
              height: 16,
              fill: "none",
              stroke: "currentColor",
              "stroke-width": 2,
            },
            [h("path", { d: DEFAULT_ICON_PATH })],
          ),
  });
}

/**
 * Register all loaded extension contributions.
 * Call this before hydrating a workspace's tool-window layout. The returned
 * function removes the actions, menus and tool windows that were added.
 */
export async function loadExtensionUI(): Promise<() => void> {
  const contributions = await invoke<FrontendContribution[]>("list_extension_ui");
  const disposers: Array<() => void> = [];

  for (const contribution of contributions) {
    for (const command of contribution.commands) {
      disposers.push(
        registerAction({
          id: command.id,
          label: command.label,
          category: command.category ?? contribution.extension,
          keys: command.keys ?? undefined,
          shortcut: command.keys ?? undefined,
          icon: command.icon ?? undefined,
          run: async (context) => {
            await fireEvent(command.event, {
              extension: contribution.extension,
              command: command.id,
              workspace: context.workspace,
            });
          },
        }),
      );
    }

    // registerMenu is per-location, so group contributed items by location.
    const menusByLocation = new Map<string, typeof contribution.menus>();
    for (const menu of contribution.menus) {
      const entries = menusByLocation.get(menu.location) ?? [];
      entries.push(menu);
      menusByLocation.set(menu.location, entries);
    }
    for (const [location, entries] of menusByLocation) {
      disposers.push(
        registerMenu(
          location,
          entries.map((menu) => ({
            commandId: menu.commandId,
            group: menu.group ?? undefined,
            order: menu.order ?? undefined,
          })),
        ),
      );
    }

    for (const tool of contribution.toolWindows) {
      const icon = await iconComponent(contribution, tool.icon);
      const component = defineComponent({
        name: `ExtensionTool_${tool.id}`,
        setup: () => () => h(ExtensionFrame, { contribution, tool }),
      });

      disposers.push(
        registerAction({
          id: `view.toggle.${tool.id}`,
          label: `Toggle ${tool.title} Tool Window`,
          category: "View",
          keys: tool.hotkey ?? undefined,
          shortcut: tool.hotkey ?? undefined,
          when: (context) => !!context.workspace,
          run: () => {
            useToolWindows().toggle(tool.id);
          },
        }),
      );
      disposers.push(
        registerMenu(Menus.titlebarMain, [
          { commandId: `view.toggle.${tool.id}`, group: "04_view", order: tool.order ?? undefined },
        ]),
      );
      registerToolWindow({
        id: tool.id,
        title: tool.title,
        position: tool.position,
        icon,
        component,
        order: tool.order ?? undefined,
        hotkey: tool.hotkey ?? undefined,
        defaultOpen: tool.defaultOpen,
      });
      const toolId = tool.id;
      disposers.push(() => unregisterToolWindow(toolId));
    }
  }

  return () => {
    for (const dispose of disposers.splice(0).reverse()) dispose();
  };
}
