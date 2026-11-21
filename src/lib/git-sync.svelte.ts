// The sync of a project's checkout, or of the agent open's own folder, with its remote (pull, push, fetch), asked from
// the branch picker and shown by the status bar's branch button while it runs.

import { api } from './ipc';
import { app } from './state.svelte';
import type { FetchFailure, GitInfo } from './types';

export type SyncOp = 'pull' | 'push' | 'fetch';

const CALLS: Record<SyncOp, (projectId: string, repo: string | null) => Promise<string>> = {
  pull: (id, repo) => api.gitPull(id, repo),
  push: (id, repo) => api.gitPush(id, repo),
  fetch: (id, repo) => api.gitFetch(id, repo),
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
  /** Why its last background fetch failed (ESC-561), and whether it goes through Windows' ssh. */
  fetchFailure: FetchFailure | null;
  fetchSshNote: boolean;
}

/** `syncInfo` of the repository `repo` of a project of several (ESC-169); of the project itself when null. */
export function repoSyncInfo(g: GitInfo | undefined, repo: string | null): SyncInfo | null {
  if (repo === null) return syncInfo(g);
  const r = g?.repos?.find((x) => x.repo === repo);
  if (!r || !r.hasRemote || !r.branch || r.branch === '(detached)') return null;
  return {
    branch: r.branch,
    upstream: r.upstream,
    tracked: !!r.upstream,
    upstreamGone: false,
    ahead: r.ahead,
    behind: r.behind,
    lastFetch: g?.lastFetch ?? null,
    fetchFailure: g?.fetchFailure ?? null,
    fetchSshNote: !!g?.fetchSshNote,
  };
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
    fetchFailure: g.fetchFailure ?? null,
    fetchSshNote: !!g.fetchSshNote,
  };
}

class GitSync {
  /** The sync running, by project. */
  running = $state<Record<string, SyncOp>>({});

  /** Runs `op` on a project's checkout (its repository `repo`, of several), then says what it did (or why it could not). */
  async run(op: SyncOp, projectId: string, repo: string | null = null) {
    if (this.running[projectId]) return;
    this.running[projectId] = op;
    const summary = await app.run(CALLS[op](projectId, repo));
    delete this.running[projectId];
    if (summary) app.toast(summary, 'ok');
  }

  /** Runs `op` on the agent's own folder (its worktree), shown by its project's branch button while it runs. */
  async runAgent(op: SyncOp, agentId: string, projectId: string) {
    if (this.running[projectId]) return;
    this.running[projectId] = op;
    const summary = await app.run(api.agentGitSync(agentId, op));
    delete this.running[projectId];
    if (summary) app.toast(summary, 'ok');
  }
}

export const gitSync = new GitSync();
