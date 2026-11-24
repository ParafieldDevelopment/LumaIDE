/** Payload emitted by the Rust per-window filesystem watcher. */
export const FILES_CHANGED_EVENT = "workspace-files-changed";

export interface FileChange {
  path: string;
  kind: "create" | "modify" | "remove" | "other";
  isDir: boolean;
}

export interface FilesChangedPayload {
  changes: FileChange[];
}
