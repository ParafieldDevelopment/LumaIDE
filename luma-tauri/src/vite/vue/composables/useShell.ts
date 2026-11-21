import { invoke } from "@tauri-apps/api/core";

/** Result of a finished process (see src/rust/shell.rs). */
export interface CommandOutput {
  /** Exit code, or null if the process was killed (e.g. timed out). */
  code: number | null;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

/** Runs `git <args>` in `cwd`. `args[0]` must be a subcommand (e.g. "status"). */
export function runGit(cwd: string, args: string[]) {
  return invoke<CommandOutput>("git_command", { cwd, args });
}

/** Runs a shell command line (PowerShell on Windows, sh elsewhere) in `cwd`. */
export function runTerminalCommand(cwd: string, command: string) {
  return invoke<CommandOutput>("run_terminal_command", { cwd, command });
}

/** Resolves a `cd` target relative to `cwd`. Rejects if it isn't a directory. */
export function changeDirectory(cwd: string, target: string) {
  return invoke<string>("terminal_change_dir", { cwd, target });
}
