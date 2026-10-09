import { openUrl } from '@tauri-apps/plugin-opener';
import { api } from './ipc';
import { app } from './state.svelte';
import type { Agent } from './types';

export const COMMIT_AGENT_PROMPT =
  'Commite les modifications que tu as faites dans ce dépôt, avec un message clair au format Conventional Commits. ' +
  "N'inclus que les fichiers que tu as modifiés ; s'il y a plusieurs sujets distincts, fais plusieurs commits.";

export const COMMIT_ALL_PROMPT =
  'Commite toutes les modifications en cours du dépôt, regroupées en commits cohérents, avec des messages clairs au format Conventional Commits.';

export async function commitViaAgent(agent: Agent, scope: 'agent' | 'project' = 'agent') {
  const ok = await app.run(api.sendMessage(agent.id, scope === 'agent' ? COMMIT_AGENT_PROMPT : COMMIT_ALL_PROMPT));
  if (ok !== undefined) app.toast(`Demande de commit envoyée à ${agent.name}`, 'ok');
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
      app.toast(out || 'Merge effectué', 'ok');
    } catch (e) {
      const current = branchInTheWay(e);
      // Asked once: a refusal of the switch itself (git's) is an error like any other.
      if (current === null || switchToBase) {
        app.toast(String(e), 'error');
        return;
      }
      app.modal = {
        kind: 'confirm',
        title: `Basculer sur « ${wt.baseBranch} » ?`,
        body:
          (current ? `Le projet est sur la branche « ${current} ».` : 'Le projet n’est sur aucune branche (HEAD détachée).') +
          ` Escouade bascule sur « ${wt.baseBranch} » puis merge « ${wt.branch} ».`,
        confirm: 'Basculer et merger',
        onConfirm: () => merge(squash, true),
      };
    }
  };
  app.modal = {
    kind: 'confirm',
    title: `Merger ${wt.branch} dans ${wt.baseBranch} ?`,
    body: `Les commits de l'agent « ${agent.name} » sont intégrés dans la branche « ${wt.baseBranch} » du projet.`,
    confirm: 'Merger',
    option: { label: 'Squash (un seul commit)', value: false },
    onConfirm: (squash) => merge(squash, false),
  };
}

/** Remote Control on / off: the agent becomes reachable from claude.ai and the Claude app. */
export async function toggleRemote(agent: Agent) {
  const on = !agent.remoteControl;
  const ok = await app.run(api.setRemoteControl(agent.id, on));
  if (ok !== undefined && on) app.toast(`${agent.name} est accessible depuis claude.ai et l’app Claude`, 'ok');
}

export function openRemote(agent: Agent) {
  if (agent.remoteUrl) openUrl(agent.remoteUrl).catch((e) => app.toast(String(e), 'error'));
}

export async function copyRemoteLink(agent: Agent) {
  if (!agent.remoteUrl) return;
  try {
    await navigator.clipboard.writeText(agent.remoteUrl);
    app.toast('Lien claude.ai copié', 'ok');
  } catch (e) {
    app.toast(String(e), 'error');
  }
}
