import { getVersion } from "@tauri-apps/api/app";
import { invoke } from '@tauri-apps/api/core';
import {getCurrentWindow} from "@tauri-apps/api/window";
export async function getAppVersion() {
    return await getVersion();
}

export async function webviewReady() {
    await invoke("webview_ready");

    await new Promise(settled => setTimeout(settled, 100));

    await getCurrentWindow().setFocus();
}

export async function loadTheme(theme_id?: string) {
    // TODO: Load Theme

    console.log("TODO: Implement theme loader {}", theme_id);
}

export async function openSettings() {
    await invoke("open_settings_window");
}