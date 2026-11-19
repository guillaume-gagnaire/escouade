import { openUrl } from './opener';
import { t, tIn } from './i18n';
import { api } from './ipc';
import { can } from './engines';
import { app } from './state.svelte';
import type { Agent, Project } from './types';

// The messages asked of an agent: in the language of the texts for Claude (« Langue des textes rédigés par Claude »),
// whatever the interface's, as everything Escouade tells an agent.
export const commitAgentPrompt = () => tIn(app.lang.claude, 'git.agent.commitPrompt');

export const commitAllPrompt = () => tIn(app.lang.claude, 'git.agent.commitAllPrompt');

export async function commitViaAgent(agent: Agent, scope: 'agent' | 'project' = 'agent') {
  const ok = await app.run(api.sendMessage(agent.id, scope === 'agent' ? commitAgentPrompt() : commitAllPrompt()));
  if (ok !== undefined) app.toast(t('git.agent.commitRequested', { name: agent.name }), 'ok');
}

/** « Commit… » (an agent's changes) needs its agent; « Commit tout… » needs one only for the agent to write it. */
export function canCommit(project: Project, agent: Agent | null, scope: 'agent' | 'project'): boolean {
  return !!agent || (scope === 'project' && project.commitMode === 'direct');
}

/**
 * « Commit… » (the agent's changes) or « Commit tout… » (the project's), as the project's « Commit » setting says:
 * the agent is asked to commit, or the direct commit opens, its message proposed, never committed unread.
 */
export function askCommit(project: Project, agent: Agent | null, scope: 'agent' | 'project') {
  if (!canCommit(project, agent, scope)) return;
  if (project.commitMode === 'direct') {
    app.modal = { kind: 'commit', projectId: project.id, agentId: scope === 'agent' ? agent!.id : null };
  } else if (agent) {
    commitViaAgent(agent, scope);
  }
}

/**
 * The branch the project is on ('' on a detached HEAD) when the backend refused to merge because
 * it is not the agent's base branch; null for any other error.
 */
function branchInTheWay(error: unknown): string | null {
  return /^NOT_ON_BASE:([^:]*):/.exec(String(error))?.[1] ?? null;
}

export function mergeAgent(agent: Agent) {
  if (!agent.worktree) return;
  const wt = agent.worktree;
  // Of several repositories (ESC-169): each one checked in its window, all or nothing.
  if (wt.repos?.length) {
    app.modal = { kind: 'mergeRepos', agentId: agent.id };
    return;
  }
  const merge = async (squash: boolean, switchToBase: boolean) => {
    try {
      const out = await api.mergeAgent(agent.id, squash, switchToBase);
      app.toast(out || t('git.agent.merged'), 'ok');
    } catch (e) {
      const current = branchInTheWay(e);
      // Asked once: a refusal of the switch itself (git's) is an error like any other.
      if (current === null || switchToBase) {
        app.toast(String(e), 'error');
        return;
      }
      app.modal = {
        kind: 'confirm',
        title: t('git.agent.switchTitle', { base: wt.baseBranch }),
        // The branch the project is on, or none: each told in a whole sentence.
        body: current
          ? t('git.agent.switchBodyOnBranch', { current, base: wt.baseBranch, branch: wt.branch })
          : t('git.agent.switchBodyDetached', { base: wt.baseBranch, branch: wt.branch }),
        confirm: t('git.agent.switchConfirm'),
        onConfirm: () => merge(squash, true),
      };
    }
  };
  app.modal = {
    kind: 'confirm',
    title: t('git.agent.mergeTitle', { branch: wt.branch, base: wt.baseBranch }),
    body: t('git.agent.mergeBody', { name: agent.name, base: wt.baseBranch }),
    confirm: t('git.agent.mergeConfirm'),
    option: { label: t('git.agent.squash'), value: false },
    onConfirm: (squash) => merge(squash, false),
  };
}

/**
 * « Intégrer <base> »: the agent's base branch goes into its branch, as the project's strategy says. A conflict is
 * handed to the agent by the backend; the toast says so, and the refusals (the agent works, it has changes) are told as
 * they come.
 */
export async function integrateBase(agent: Agent) {
  const wt = agent.worktree;
  if (!wt) return;
  const [base, branch] = [wt.baseBranch, wt.branch];
  try {
    const done = await api.integrateBase(agent.id);
    if (done.kind === 'done') app.toast(t('branches.integrate.done', { base, branch }), 'ok');
    else if (done.kind === 'upToDate') app.toast(t('branches.integrate.upToDate', { base, branch }), 'info');
    else if (done.rebase) app.toast(t('branches.integrate.conflictRebase', { base, agent: agent.name }), 'info');
    else app.toast(t('branches.integrate.conflictMerge', { base, branch, agent: agent.name, count: done.files.length }), 'info');
  } catch (e) {
    app.toast(String(e), 'error');
  }
}

/**
 * « Vue Claude Code » / « Vue Escouade », from the header of its conversation or the menu of its card: never for a
 * ticket's agent, which its board drives over the stream, nor for an archived one, nor for a harness without a terminal view.
 */
export const canSwitchView = (agent: Agent) => !agent.ticketId && !agent.archived && can(agent, 'terminalView');

/** In its harness's own interface, either in place of its process (Claude Code, Codex) or joined to it (OpenCode). */
export const inHarnessView = (agent: Agent) => agent.terminal || agent.attached;

/** Into the other view; the backend's refusals (a turn under way, remote control on) are told. */
export const switchView = (agent: Agent) => app.run(api.setAgentView(agent.id, !inHarnessView(agent)));

/**
 * Remote Control on / off by hand (off always sets the agent apart, a setting covering it or not): the agent becomes
 * reachable from claude.ai and the Claude app. The backend answers whether the change waits for the next launch of an
 * interface of Claude Code that is open (it keeps the remote control it was started with, and its own /remote-control
 * does it until then): told, rather than that it is reachable. Otherwise the backend starts the process in the
 * background when the remote control comes on, and stops it when it goes off: reachable once on, and nothing to tell
 * once off.
 */
export async function toggleRemote(agent: Agent) {
  const on = !agent.remoteControl;
  const later = await app.run(api.setRemoteControl(agent.id, on));
  if (later === undefined) return;
  if (later) app.toast(t('git.agent.remoteNextLaunch'), 'ok');
  else if (on) app.toast(t('git.agent.remoteOn', { name: agent.name }), 'ok');
}

/**
 * In the Escouade view, an agent that holds Remote Control with no process (stopped for being idle, over the cap of the
 * processes kept, the app restarted) is asleep: claude.ai sees its session disconnected until its next process
 * (ESC-544). Its next message wakes it, or « Réveiller ».
 */
export const remoteAsleep = (agent: Agent) => !agent.terminal && !agent.archived && agent.remoteControl && !agent.alive;

/** « Réveiller »: its process starts and takes its remote session back. */
export async function wakeRemote(agent: Agent) {
  await app.run(api.wakeRemote(agent.id));
}

export function openRemote(agent: Agent) {
  if (agent.remoteUrl) openUrl(agent.remoteUrl).catch((e) => app.toast(String(e), 'error'));
}

export async function copyRemoteLink(agent: Agent) {
  if (!agent.remoteUrl) return;
  try {
    await navigator.clipboard.writeText(agent.remoteUrl);
    app.toast(t('git.agent.linkCopied'), 'ok');
  } catch (e) {
    app.toast(String(e), 'error');
  }
}
