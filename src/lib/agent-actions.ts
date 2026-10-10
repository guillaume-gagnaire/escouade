import { openUrl } from '@tauri-apps/plugin-opener';
import { t, tIn } from './i18n';
import { api } from './ipc';
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

/** Remote Control on / off: the agent becomes reachable from claude.ai and the Claude app. */
export async function toggleRemote(agent: Agent) {
  const on = !agent.remoteControl;
  const ok = await app.run(api.setRemoteControl(agent.id, on));
  if (ok !== undefined && on) app.toast(t('git.agent.remoteOn', { name: agent.name }), 'ok');
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
