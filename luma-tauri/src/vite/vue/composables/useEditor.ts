import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface OpenFile {
  id: number;
  path: string;
  name: string;
  contents: string;
  dirty: boolean;
  language?: string;
}

let nextFileId = 1;

const openFiles = ref<OpenFile[]>([]);
const activeFileId = ref<number>(0);
const workspacePath = ref<string | null>(null);
const loadError = ref<string | null>(null);

export function useEditor() {
  const activeFile = computed<OpenFile | null>(() => {
    if (activeFileId.value === 0) return null;
    return openFiles.value.find((file) => file.id === activeFileId.value) ?? null;
  });

  const fileByPath = (path: string) =>
    openFiles.value.find((file) => file.path === path);

  function languageForPath(path: string): string | undefined {
    const ext = path.split(".").pop() ?? "";
    switch (ext.toLowerCase()) {
      case "ts":
      case "js":
      case "mjs":
      case "cjs":
        return "javascript";
      case "tsx":
      case "jsx":
        return "javascriptreact";
      case "json":
        return "json";
      case "html":
        return "html";
      case "css":
        return "css";
      case "scss":
      case "sass":
        return "scss";
      case "md":
        return "markdown";
      case "lua":
      case "luau":
        return "lua";
      case "rs":
        return "rust";
      case "py":
        return "python";
      case "toml":
        return "toml";
      case "txt":
        return "plaintext";
      default:
        return undefined;
    }
  }

  async function openFile(path: string, name?: string) {
    loadError.value = null;
    const existing = fileByPath(path);
    if (existing) {
      activeFileId.value = existing.id;
      return existing;
    }

    let contents: string;
    try {
      contents = await invoke<string>("read_file", { path });
    } catch (error) {
      loadError.value = `Unable to read "${name ?? path}": ${String(error)}`;
      return null;
    }

    const file: OpenFile = {
      id: nextFileId++,
      path,
      name: name ?? path.split(/[/\\]/).pop() ?? path,
      contents,
      dirty: false,
      language: languageForPath(path),
    };
    openFiles.value.push(file);
    activeFileId.value = file.id;
    return file;
  }

  async function saveFile(id: number): Promise<boolean> {
    const file = openFiles.value.find((f) => f.id === id);
    if (!file) return false;
    if (!file.dirty) return true;

    loadError.value = null;
    try {
      await invoke("write_file", { path: file.path, contents: file.contents });
      file.dirty = false;
      return true;
    } catch (error) {
      loadError.value = `Unable to save "${file.name}": ${String(error)}`;
      return false;
    }
  }

  async function saveAll(): Promise<boolean> {
    let ok = true;
    for (const file of openFiles.value) {
      if (file.dirty) {
        ok = (await saveFile(file.id)) && ok;
      }
    }
    return ok;
  }

  function closeFile(id: number) {
    const index = openFiles.value.findIndex((f) => f.id === id);
    if (index === -1) return;
    const file = openFiles.value[index];
    openFiles.value.splice(index, 1);
    if (activeFileId.value === id) {
      activeFileId.value = openFiles.value.length > 0 ? openFiles.value[0].id : 0;
    }
    return file;
  }

  function setContents(id: number, contents: string) {
    const file = openFiles.value.find((f) => f.id === id);
    if (!file) return;
    if (file.contents !== contents) {
      file.contents = contents;
      file.dirty = true;
    }
  }

  function setClean(id: number) {
    const file = openFiles.value.find((f) => f.id === id);
    if (file) file.dirty = false;
  }

  function setWorkspacePath(path: string | null) {
    workspacePath.value = path;
  }

  return {
    openFiles,
    activeFileId,
    activeFile,
    workspacePath,
    loadError,
    openFile,
    saveFile,
    saveAll,
    closeFile,
    setContents,
    setClean,
    setWorkspacePath,
    languageForPath,
  };
}
