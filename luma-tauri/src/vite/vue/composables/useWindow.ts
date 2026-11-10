import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { reactive, readonly } from "vue";
import {waitUntilReady} from "~/luma/LumaWindow.ts";

const appWindow = getCurrentWindow();

const state = reactive({
    isMacOS: platform() === "macos",
    isMaximized: false,
    isFocused: true,
});

let initialization: Promise<void> | undefined;

let unlisteners: (() => void)[] = [];

async function initializeWindow() {
    if (initialization) return initialization;

    initialization = (async () => {
        await waitUntilReady()

        state.isMaximized = await appWindow.isMaximized();
        state.isFocused = await appWindow.isFocused();

        unlisteners.push(
            await appWindow.onResized(async () => {
                state.isMaximized = await appWindow.isMaximized();
            }),

            await appWindow.onFocusChanged(({ payload }) => {
                state.isFocused = payload;
            }),
        );
    })();

    try {
        await initialization;
    } catch (error) {
        initialization = undefined;
        throw error;
    }
}

export function useWindow() {

    // Make sure the native event listeners are initialized.
    void initializeWindow().catch(console.error);

    return {
        // Reactive state (shared across components)
        state: readonly(state),

        // Window actions
        minimize: () => appWindow.minimize(),
        maximize: () => appWindow.maximize(),
        unmaximize: () => appWindow.unmaximize(),
        toggleMaximize: () => appWindow.toggleMaximize(),
        close: () => appWindow.close(),
        setFocus: () => appWindow.setFocus(),

        // Direct access if needed
        appWindow,
    };
}