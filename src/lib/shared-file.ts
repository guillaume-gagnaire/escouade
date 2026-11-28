// A project's `.escouade.yml` (ESC-146), as the window shows it: where each setting comes from, and the commands of the
// file that wait for the user's approval.

import type { Project, SharedFile } from './types';

export const SHARED_FILE = '.escouade.yml';

/** Where a setting of the project comes from: the file, the file but waiting for approval, this machine ahead of the file. */
export type SharedState = 'file' | 'pending' | 'local' | null;

/** The file as the project last read it (none, or unreadable: no setting comes from it). */
export function sharedFile(p: Project | undefined | null): SharedFile | null {
  return p?.shared?.file ?? null;
}

/** Where the settings of `keys` come from (one row may show several: the first that says something). */
export function sharedState(p: Project | undefined | null, keys: string | string[]): SharedState {
  const file = sharedFile(p);
  if (!file || file.error) return null;
  for (const key of Array.isArray(keys) ? keys : [keys]) {
    if (!file.keys.includes(key)) continue;
    if (p?.shared?.local.includes(key)) return 'local';
    if (file.pending && key in file.pending.commands) return 'pending';
    return 'file';
  }
  return null;
}

/** The keys of `keys` the file sets but this machine keeps otherwise. */
export function keptKeys(p: Project | undefined | null, keys: string | string[]): string[] {
  const file = sharedFile(p);
  if (!file) return [];
  return (Array.isArray(keys) ? keys : [keys]).filter((k) => file.keys.includes(k) && p?.shared?.local.includes(k));
}

/** A command waiting, as the list shows it: what it runs, where, and its name for a launch command. */
export interface PendingCommand {
  name: string;
  command: string;
  cwd: string;
  shell: string;
}

/** The commands of the file waiting for approval, by group (worktree setup, teardown, launch, tests), in that order. */
export function pendingGroups(
  p: Project | undefined | null,
): { kind: 'setup' | 'teardown' | 'launch' | 'test' | 'page'; commands: PendingCommand[] }[] {
  const pending = sharedFile(p)?.pending;
  if (!pending) return [];
  const list = (v: unknown): PendingCommand[] =>
    (Array.isArray(v) ? v : []).map((x: Record<string, unknown>) => ({
      name: String(x.name ?? ''),
      command: String(x.command ?? ''),
      cwd: String(x.cwd ?? ''),
      shell: String(x.shell ?? ''),
    }));
  const groups = [
    { kind: 'setup' as const, key: 'worktrees.setup' },
    { kind: 'teardown' as const, key: 'worktrees.teardown' },
    { kind: 'launch' as const, key: 'launch' },
    { kind: 'test' as const, key: 'board.test_command' },
    // « Page à soi » (AP-7): no command, but the file's page waits for the same trust.
    { kind: 'page' as const, key: 'pages.custom' },
  ];
  return groups
    .filter((g) => g.key in pending.commands)
    .map((g) => {
      const v = pending.commands[g.key];
      const commands =
        g.kind === 'test' || g.kind === 'page'
          ? String(v ?? '').trim()
            ? [{ name: '', command: String(v), cwd: '', shell: '' }]
            : []
          : list(v);
      return { kind: g.kind, commands };
    });
}

/** Whether the project's file has commands waiting for the user's approval. */
export function hasPending(p: Project | undefined | null): boolean {
  return !!sharedFile(p)?.pending;
}
