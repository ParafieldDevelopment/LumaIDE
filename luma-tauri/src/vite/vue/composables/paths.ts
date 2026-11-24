/** Path helpers shared by the file watcher, explorer and editor. */

/** Case-insensitive, separator-neutral key for matching paths across IPC. */
export function pathKey(path: string): string {
  return path.replace(/\\/g, "/").toLowerCase();
}

/** Parent directory of a file path, preserving the original separators. */
export function parentPath(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const index = normalized.lastIndexOf("/");
  if (index < 0) return path;
  if (index === 0) return path.startsWith("/") ? "/" : path;
  // Windows drive root, e.g. C:/file.txt -> C:\
  if (index === 2 && /^[A-Za-z]:/.test(normalized)) return `${path.slice(0, 2)}\\`;
  return path.slice(0, index);
}
