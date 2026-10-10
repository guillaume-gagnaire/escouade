// The sync of a project's checkout with its remote (pull, push, fetch), asked from the branch picker and shown by the
// status bar's branch button while it runs.

import { api } from './ipc';
import { app } from './state.svelte';
import type { GitInfo } from './types';

export type SyncOp = 'pull' | 'push' | 'fetch';

const CALLS: Record<SyncOp, (projectId: string) => Promise<string>> = {
  pull: (id) => api.gitPull(id),
  push: (id) => api.gitPush(id),
  fetch: (id) => api.gitFetch(id),
};

/** What a checkout can sync with: a branch checked out and a remote. Null for a repository that has none of them. */
export interface SyncInfo {
  branch: string;
  upstream: string | null;
  /** Tracks a remote branch that still exists: pull and push have something to talk to. Else the branch is (re)published. */
  tracked: boolean;
  upstreamGone: boolean;
  ahead: number;
  behind: number;
  lastFetch: number | null;
}

export function syncInfo(g: GitInfo | undefined): SyncInfo | null {
  if (!g?.isRepo || !g.hasRemote || !g.branch || g.branch === '(detached)') return null;
  return {
    branch: g.branch,
    upstream: g.upstream,
    tracked: !!g.upstream && !g.upstreamGone,
    upstreamGone: g.upstreamGone,
    ahead: g.ahead,
    behind: g.behind,
    lastFetch: g.lastFetch,
  };
}

class GitSync {
  /** The sync running, by project. */
  running = $state<Record<string, SyncOp>>({});

  /** Runs `op` on a project's checkout, then says what it did (or why it could not). */
  async run(op: SyncOp, projectId: string) {
    if (this.running[projectId]) return;
    this.running[projectId] = op;
    const summary = await app.run(CALLS[op](projectId));
    delete this.running[projectId];
    if (summary) app.toast(summary, 'ok');
  }
}

export const gitSync = new GitSync();
