// What the user asks of the branches of a project (switch, delete), and how the backend's refusals (`DIRTY`,
// `IN_WORKTREE:<agent id>:<agent name>`, `AGENT_WORKING:<agent id>:<agent name>`, `UNMERGED:<n>`; see
// `core::BranchRefusal`) become a question or a toast in the user's own words.

import { t } from './i18n';
import { api } from './ipc';
import { app } from './state.svelte';
import type { BranchInfo } from './types';

export type BranchRefusal =
  | { kind: 'dirty' }
  | { kind: 'inWorktree'; agentId: string; agent: string }
  | { kind: 'agentWorking'; agentId: string; agent: string }
  /** `commits` that no other branch has: 0 when the branch is not in the base, but its commits are in another branch. */
  | { kind: 'unmerged'; commits: number };

/**
 * The refusal the backend sent for a branch operation, null for any other error. The agent's id has no colon, its name
 * (the rest of the line) may have some.
 */
export function branchRefusal(error: unknown): BranchRefusal | null {
  if (typeof error !== 'string') return null;
  if (error === 'DIRTY') return { kind: 'dirty' };
  const worktree = /^IN_WORKTREE:([^:]*):(.*)$/s.exec(error);
  if (worktree) return { kind: 'inWorktree', agentId: worktree[1], agent: worktree[2] };
  const working = /^AGENT_WORKING:([^:]*):(.*)$/s.exec(error);
  if (working) return { kind: 'agentWorking', agentId: working[1], agent: working[2] };
  const unmerged = /^UNMERGED:(\d+)$/.exec(error);
  if (unmerged) return { kind: 'unmerged', commits: Number(unmerged[1]) };
  return null;
}

/** Why the backend refused an operation on `branch`: its code in a sentence, anything else as it came. */
export function refusalText(error: unknown, branch: string): string {
  const why = branchRefusal(error);
  switch (why?.kind) {
    case 'agentWorking':
      return t('branches.switch.agentWorking', { agent: why.agent });
    case 'inWorktree':
      return t('branches.picker.usedByWhy', { branch, agent: why.agent });
    case 'unmerged':
      return why.commits ? t('branches.delete.forceCommits', { count: why.commits }) : t('branches.delete.forceNoCommits');
    default:
      return String(error);
  }
}

/** Tells in a toast why the backend refused an operation on `branch`. */
export function tellRefusal(error: unknown, branch: string) {
  app.toast(refusalText(error, branch), 'error');
}

/** `switched`: the folder is on the branch; `asked`: a question was opened (stash the changes?); `refused`: told in a toast. */
export type SwitchResult = 'switched' | 'asked' | 'refused';

/**
 * Switches the project's folder to `name` (a remote branch: to the local one that tracks it). Uncommitted changes
 * refuse it, unless `stash`: the question comes first, and asking « Mettre de côté (stash) et changer » runs it again
 * with the stash. `release` runs before the question opens: what is on screen (the picker) gets out of its way, so
 * that the question takes the focus last and gives it back to where it came from.
 */
export async function switchBranch(
  projectId: string,
  name: string,
  stash = false,
  release?: () => void | Promise<void>,
): Promise<SwitchResult> {
  try {
    const stashed = await api.branchSwitch(projectId, name, stash);
    if (stashed) app.toast(t('branches.switch.stashed', { name: stashed }), 'ok');
    return 'switched';
  } catch (e) {
    if (branchRefusal(e)?.kind === 'dirty') {
      await release?.();
      askStash(t('branches.switch.title', { branch: name }), () => switchBranch(projectId, name, true));
      return 'asked';
    }
    tellRefusal(e, name);
    return 'refused';
  }
}

/** The question « stash the changes and switch? », titled `title`; `again` runs the operation with the stash. */
export function askStash(title: string, again: () => unknown, onCancel?: () => void) {
  app.modal = {
    kind: 'confirm',
    title,
    body: t('branches.switch.dirty'),
    confirm: t('branches.switch.stashAndSwitch'),
    onConfirm: async () => {
      await again();
    },
    onCancel,
  };
}

/**
 * Starts an agent in a worktree on `branch`, which exists (a remote one: on the local branch that tracks it), and shows
 * it at once like a new agent. The branch of the project's folder and a branch another worktree has are refused by the
 * backend, and told in a toast.
 */
export async function startAgentOn(projectId: string, branch: string): Promise<boolean> {
  try {
    const a = await api.createAgentOnBranch(projectId, branch);
    app.agents[a.id] = a;
    // Its composer is what a new agent is for: closed first, the editor keeps the source it was on.
    app.closeEditor(projectId);
    app.selectAgent(a.id);
    return true;
  } catch (e) {
    tellRefusal(e, branch);
    return false;
  }
}

/**
 * Why a local branch can be neither switched to nor deleted: a worktree other than the project's folder holds it (an
 * agent's, named, or any other, by its folder). Null for the branch of the folder, and for a branch nothing holds.
 */
export function worktreeReason(b: BranchInfo): string | null {
  if (b.current || !b.worktree) return null;
  if (b.agent) return t('branches.picker.usedByWhy', { branch: b.name, agent: app.agents[b.agent]?.name ?? '?' });
  return t('branches.picker.otherWorktreeWhy', { branch: b.name, path: b.worktree });
}

/** A branch that can be deleted: neither the one the project's folder is on nor one a worktree holds. */
export const deletable = (b: BranchInfo) => b.remote || (!b.current && !b.worktree);

/**
 * The remote copy of a local branch that « Supprimer aussi » offers, as git names it (`origin/feat`): the branch it
 * tracks when that one still exists and bears the same name. A copy with another name is not the branch's own.
 */
export function remoteCopy(b: BranchInfo): string | null {
  if (b.remote || !b.upstream || b.upstreamGone) return null;
  return b.upstream.endsWith(`/${b.name}`) ? b.upstream : null;
}

/**
 * Deletes a branch of the project after asking. A branch whose work is not in the base is refused by the backend
 * (`UNMERGED:<n>`): the question comes again, with what would be lost, and only then is it forced.
 */
export function deleteBranch(projectId: string, b: BranchInfo) {
  const remote = remoteCopy(b);
  const run = async (alsoRemote: boolean, force: boolean) => {
    try {
      const note = await api.branchDelete(projectId, b.name, alsoRemote, force);
      // The sentence says why the remote copy stayed: it says the rest too.
      if (note) app.toast(note, 'info');
      else app.toast(t('branches.delete.done', { branch: b.name }), 'ok');
    } catch (e) {
      const why = branchRefusal(e);
      if (why?.kind === 'unmerged') askForce(why.commits, alsoRemote);
      else tellRefusal(e, b.name);
    }
  };
  const askForce = (commits: number, alsoRemote: boolean) => {
    app.modal = {
      kind: 'confirm',
      title: t('branches.delete.forceTitle', { branch: b.name }),
      body: commits ? t('branches.delete.forceCommits', { count: commits }) : t('branches.delete.forceNoCommits'),
      confirm: t('branches.delete.forceConfirm'),
      danger: true,
      onConfirm: () => run(alsoRemote, true),
    };
  };
  app.modal = b.remote
    ? {
        kind: 'confirm',
        title: t('branches.delete.remoteTitle', { branch: b.name }),
        body: t('branches.delete.remoteBody'),
        confirm: t('common.delete'),
        danger: true,
        onConfirm: () => run(false, false),
      }
    : {
        kind: 'confirm',
        title: t('branches.delete.title', { branch: b.name }),
        body: t('branches.delete.body', { branch: b.name }),
        confirm: t('common.delete'),
        option: remote ? { label: t('branches.delete.alsoRemote', { remote }), value: false } : undefined,
        onConfirm: (alsoRemote) => run(alsoRemote, false),
      };
}
