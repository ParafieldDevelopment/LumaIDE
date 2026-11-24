import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { confirmDialog } from "./useConfirm";
import { pathKey } from "./paths";

export interface OpenFile {
  id: number;
  path: string;
  name: string;
  contents: string;
  dirty: boolean;
  language?: string;
  /** Hash of the content this editor session was last based on. */
  hash: string;
  /** Latest state read from disk (used for conflict prompts). */
  diskContents: string;
  diskHash: string;
  conflicted: boolean;
}

interface FileSnapshot {
  contents: string;
  hash: string;
}

interface WriteFileResult {
  saved: boolean;
  hash: string;
  diskContents?: string;
}

let nextFileId = 1;

const openFiles = ref<OpenFile[]>([]);
const activeFileId = ref<number>(0);
const workspacePath = ref<string | null>(null);
const loadError = ref<string | null>(null);
const conflictPrompts = new Set<number>();

export function useEditor() {
  const activeFile = computed<OpenFile | null>(() => {
    if (activeFileId.value === 0) return null;
    return openFiles.value.find((file) => file.id === activeFileId.value) ?? null;
  });

  const fileByPath = (path: string) =>
    openFiles.value.find((file) => pathKey(file.path) === pathKey(path));

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

    let snapshot: FileSnapshot;
    try {
      snapshot = await invoke<FileSnapshot>("read_file_snapshot", { path });
    } catch (error) {
      loadError.value = `Unable to read "${name ?? path}": ${String(error)}`;
      return null;
    }

    const file: OpenFile = {
      id: nextFileId++,
      path,
      name: name ?? path.split(/[/\\]/).pop() ?? path,
      contents: snapshot.contents,
      dirty: false,
      language: languageForPath(path),
      hash: snapshot.hash,
      diskContents: snapshot.contents,
      diskHash: snapshot.hash,
      conflicted: false,
    };
    openFiles.value.push(file);
    activeFileId.value = file.id;
    return file;
  }

  /** Ask the user what to do when disk and editor content have diverged. */
  async function resolveConflict(file: OpenFile, reason: string): Promise<boolean> {
    if (conflictPrompts.has(file.id)) return false;
    conflictPrompts.add(file.id);
    try {
      const answer = await confirmDialog({
        title: "File Changed on Disk",
        message: `${reason}\n\n"${file.name}" was modified by another program.\n\nOverwrite it with your changes, or reload the version from disk?`,
        buttons: [
          { label: "Overwrite", value: "overwrite", variant: "primary" },
          { label: "Reload from Disk", value: "reload" },
          { label: "Cancel", value: "cancel" },
        ],
      });

      if (answer.value === "overwrite") {
        const result = await invoke<WriteFileResult>("write_file", {
          path: file.path,
          contents: file.contents,
        });
        if (!result.saved) {
          loadError.value = `Unable to overwrite "${file.name}"`;
          return false;
        }
        file.hash = result.hash;
        file.diskHash = result.hash;
        file.diskContents = file.contents;
        file.dirty = false;
        file.conflicted = false;
        return true;
      }

      if (answer.value === "reload") {
        file.contents = file.diskContents;
        file.hash = file.diskHash;
        file.dirty = false;
        file.conflicted = false;
        return true;
      }

      // Cancel: keep the editor content and dirty flag, stay conflicted.
      return false;
    } catch (error) {
      loadError.value = `Unable to resolve conflict for "${file.name}": ${String(error)}`;
      return false;
    } finally {
      conflictPrompts.delete(file.id);
    }
  }

  async function saveFile(id: number, skipConflicted = false): Promise<boolean> {
    const file = openFiles.value.find((f) => f.id === id);
    if (!file) return false;
    if (!file.dirty) return true;
    if (file.conflicted && skipConflicted) return false;

    loadError.value = null;
    try {
      const result = await invoke<WriteFileResult>("write_file", {
        path: file.path,
        contents: file.contents,
        expectedHash: file.hash,
      });
      if (result.saved) {
        file.hash = result.hash;
        file.diskHash = result.hash;
        file.diskContents = file.contents;
        file.dirty = false;
        file.conflicted = false;
        return true;
      }

      // Disk changed underneath us — offer overwrite/reload.
      file.conflicted = true;
      file.diskContents = result.diskContents ?? "";
      file.diskHash = result.hash;
      return await resolveConflict(file, "Your changes could not be saved safely.");
    } catch (error) {
      loadError.value = `Unable to save "${file.name}": ${String(error)}`;
      return false;
    }
  }

  async function saveAll(skipConflicted = false): Promise<boolean> {
    let ok = true;
    for (const file of openFiles.value) {
      if (file.dirty) {
        ok = (await saveFile(file.id, skipConflicted)) && ok;
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

/** Called for watcher events. Clean tabs reload silently; dirty tabs prompt. */
export async function handleExternalChange(path: string) {
  const file = openFiles.value.find((f) => pathKey(f.path) === pathKey(path));
  if (!file) return;

  let snapshot: FileSnapshot;
  try {
    snapshot = await invoke<FileSnapshot>("read_file_snapshot", { path: file.path });
  } catch {
    // The file was deleted outside Luma.
    if (!file.dirty) {
      file.contents = "";
      file.diskContents = "";
      file.hash = "";
      file.diskHash = "";
      file.conflicted = false;
    } else {
      file.conflicted = true;
      loadError.value = `"${file.name}" was deleted outside Luma.`;
    }
    return;
  }

  if (snapshot.hash === file.hash && snapshot.hash === file.diskHash) return;

  file.diskContents = snapshot.contents;
  file.diskHash = snapshot.hash;

  if (!file.dirty && !file.conflicted) {
    file.contents = snapshot.contents;
    file.hash = snapshot.hash;
    return;
  }

  file.conflicted = true;
  await resolveExternalConflict(file);
}

async function resolveExternalConflict(file: OpenFile) {
  if (conflictPrompts.has(file.id)) return;
  conflictPrompts.add(file.id);
  try {
    const answer = await confirmDialog({
      title: "File Changed on Disk",
      message: `"${file.name}" was modified by another program while you were editing it.\n\nOverwrite it with your changes, or reload the version from disk?`,
      buttons: [
        { label: "Overwrite", value: "overwrite", variant: "primary" },
        { label: "Reload from Disk", value: "reload" },
        { label: "Cancel", value: "cancel" },
      ],
    });

    if (answer.value === "overwrite") {
      const result = await invoke<WriteFileResult>("write_file", {
        path: file.path,
        contents: file.contents,
      });
      if (!result.saved) {
        loadError.value = `Unable to overwrite "${file.name}"`;
        return;
      }
      file.hash = result.hash;
      file.diskHash = result.hash;
      file.diskContents = file.contents;
      file.dirty = false;
      file.conflicted = false;
    } else if (answer.value === "reload") {
      file.contents = file.diskContents;
      file.hash = file.diskHash;
      file.dirty = false;
      file.conflicted = false;
    }
    // Cancel keeps the local edits and the conflicted marker.
  } catch (error) {
    loadError.value = `Unable to resolve conflict for "${file.name}": ${String(error)}`;
  } finally {
    conflictPrompts.delete(file.id);
  }
}
