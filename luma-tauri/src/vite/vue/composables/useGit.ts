/** Typed Git operations built on the generic `git_command` IPC bridge. */
import { invoke } from "@tauri-apps/api/core";
import type { CommandOutput } from "./useShell";

export interface GitFileChange {
  path: string;
  /** Porcelain X (index) status. */
  index: string;
  /** Porcelain Y (worktree) status. */
  worktree: string;
}

export interface GitBranch {
  name: string;
  current: boolean;
  upstream: string;
}

export interface GitLogEntry {
  hash: string;
  message: string;
}

export interface GitStatus {
  branch: string;
  changes: GitFileChange[];
}

async function git(cwd: string, args: string[]): Promise<CommandOutput> {
  return invoke<CommandOutput>("git_command", { cwd, args });
}

function ok(output: CommandOutput, fallback: string): string {
  if (output.code !== 0) {
    const message = output.stderr.trim() || output.stdout.trim();
    if (output.timedOut) throw new Error("Git command timed out");
    throw new Error(message || fallback);
  }
  return output.stdout;
}

export async function gitStatus(cwd: string): Promise<GitStatus> {
  const stdout = ok(
    await git(cwd, ["status", "--porcelain=v1", "--branch"]),
    "git status failed",
  );
  const changes: GitFileChange[] = [];
  let branch = "";
  for (const line of stdout.split(/\r?\n/)) {
    if (!line) continue;
    if (line.startsWith("## ")) {
      branch = line.slice(3).split("...")[0].split(" ")[0];
      continue;
    }
    const index = line[0] ?? " ";
    const worktree = line[1] ?? " ";
    let path = line.slice(3);
    if (path.includes(" -> ")) path = path.split(" -> ").pop() ?? path;
    changes.push({ path, index, worktree });
  }
  return { branch, changes };
}

export async function gitBranches(cwd: string): Promise<GitBranch[]> {
  const stdout = ok(
    await git(cwd, ["branch", "--format=%(refname:short)%09%(HEAD)%09%(upstream:short)"]),
    "git branch failed",
  );
  return stdout
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => {
      const [name, head, upstream] = line.split("\t");
      return { name, current: head === "*", upstream: upstream ?? "" };
    });
}

export async function gitLog(cwd: string, limit = 30): Promise<GitLogEntry[]> {
  const stdout = ok(
    await git(cwd, ["log", `--max-count=${limit}`, "--pretty=format:%h%x09%s"]),
    "git log failed",
  );
  return stdout
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => {
      const [hash, ...rest] = line.split("\t");
      return { hash, message: rest.join("\t") };
    });
}

export async function gitDiff(cwd: string, path: string, staged = false): Promise<string> {
  const args = ["diff"];
  if (staged) args.push("--cached");
  args.push("--", path);
  return ok(await git(cwd, args), "git diff failed");
}

export async function gitInit(cwd: string): Promise<void> {
  ok(await git(cwd, ["init"]), "git init failed");
}

export async function gitStage(cwd: string, paths: string[]): Promise<void> {
  if (!paths.length) return;
  ok(await git(cwd, ["add", "--", ...paths]), "git add failed");
}

export async function gitUnstage(cwd: string, paths: string[]): Promise<void> {
  if (!paths.length) return;
  ok(await git(cwd, ["restore", "--staged", "--", ...paths]), "git restore --staged failed");
}

export async function gitCommit(cwd: string, message: string): Promise<string> {
  return ok(await git(cwd, ["commit", "-m", message]), "git commit failed");
}

export async function gitCheckout(cwd: string, branch: string): Promise<void> {
  ok(await git(cwd, ["checkout", branch]), `git checkout ${branch} failed`);
}

export async function gitCreateBranch(cwd: string, name: string): Promise<void> {
  ok(await git(cwd, ["checkout", "-b", name]), `git checkout -b ${name} failed`);
}

export async function gitFetch(cwd: string): Promise<void> {
  ok(await git(cwd, ["fetch", "--all", "--prune"]), "git fetch failed");
}

export async function gitPull(cwd: string): Promise<void> {
  ok(await git(cwd, ["pull", "--no-edit"]), "git pull failed");
}

export async function gitPush(cwd: string, branch?: string): Promise<void> {
  const output = await git(cwd, ["push"]);
  if (output.code === 0) return;
  // No upstream yet: publish the current branch explicitly.
  if (branch && /no upstream|no upstream branch/i.test(output.stderr)) {
    ok(await git(cwd, ["push", "--set-upstream", "origin", branch]), "git push --set-upstream failed");
    return;
  }
  ok(output, "git push failed");
}
