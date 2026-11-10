import {getVersion} from "@tauri-apps/api/app";
import {invoke} from '@tauri-apps/api/core';
import {ref, watch} from "vue";

let windowReady = ref<boolean>(false);
export async function getAppVersion() {
    return await getVersion();
}

export async function waitUntilReady(): Promise<void> {
    return new Promise((resolve) => {
        if (windowReady.value) {
            resolve();
            return;
        }

        const stop = watch(windowReady, (value) => {
            if (value) {
                stop();
                resolve();
            }
        });
    });
}
export async function webviewReady() {
    await invoke("webview_ready");

    await new Promise(settled => setTimeout(settled, 100));

    windowReady.value = true;
}

export async function loadTheme(theme_id?: string) {
    // TODO: Load Theme

    console.log("TODO: Implement theme loader {}", theme_id);
}

export async function openSettings() {
    await invoke("open_settings_window");
}