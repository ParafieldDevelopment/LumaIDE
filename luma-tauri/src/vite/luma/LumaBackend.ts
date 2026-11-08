import { getVersion } from "@tauri-apps/api/app";

export async function getAppVersion() {
    return await getVersion();
}