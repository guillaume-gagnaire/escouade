// The Claude accounts as the window shows them.

import { quotaUntil } from './board';
import { fList } from './format';
import { t, tIn } from './i18n';
import { app } from './state.svelte';
import type { Account, Agent, AutopilotPause, Project } from './types';

/** The id of the user's own account. */
export const PRINCIPAL = 'principal';

/** The files, then the folders, of Principal's that a new account may share (`accounts::SHARED_FILES`, `SHARED_DIRS`). */
export const SHARED_FILES = ['settings.json', 'CLAUDE.md'];

/**
 * The account's name as shown. Principal is named in the language of its first save: while the user has not renamed it,
 * it is shown in the interface's.
 */
export function accountName(a: Pick<Account, 'id' | 'name'>): string {
  const name = a.name.trim();
  const defaults = [tIn('fr', 'accounts.principal'), tIn('en', 'accounts.principal')];
  return a.id === PRINCIPAL && (!name || defaults.includes(name)) ? t('accounts.principal') : a.name;
}

/** The agents not archived that run on the account: while there are some, it is not removed. */
export function agentsOn(id: string): number {
  return Object.values(app.agents).filter((a) => a.account === id && !a.archived).length;
}

/** The active account the project prefers (« Compte préféré »), if it names one: a switched off or unknown one is none. */
function preferred(project: Pick<Project, 'account'> | undefined): Account | undefined {
  const id = project?.account;
  return id ? app.settings.accounts.find((a) => a.id === id && a.active) : undefined;
}

/**
 * The account a new agent of the project would go to now (« Automatique »): the one it prefers, else the current account
 * (the first active one under the pause threshold, as the backend says), else the first active one.
 */
export function newAgentAccount(project: Pick<Project, 'account'> | undefined): Account | undefined {
  const { accounts } = app.settings;
  return preferred(project) ?? accounts.find((a) => a.id === app.usage.current) ?? accounts.find((a) => a.active);
}

/** Why no ticket of the project starts: the pause of its preferred account alone, else of the accounts new agents may go to. */
export function pauseOf(project: Pick<Project, 'id' | 'account'>): AutopilotPause | null {
  return preferred(project) ? (app.projectPauses[project.id] ?? null) : app.autopilotPause;
}

/** The agent's account as the backend takes it: one the window does not know is Principal's. */
function accountOf(a: Agent): string {
  return app.settings.accounts.some((x) => x.id === a.account) ? a.account : PRINCIPAL;
}

/**
 * When the first agent waiting for its quota resumes, if every account a ticket of the project could start on has one:
 * nothing starts before. While one has none, a ticket starts on it.
 */
export function quotaOf(project: Pick<Project, 'account'>): number | null {
  const pick = preferred(project);
  const ids = pick ? [pick.id] : app.settings.accounts.filter((a) => a.active).map((a) => a.id);
  const agents = Object.values(app.agents);
  const times = ids.map((id) => quotaUntil(agents.filter((a) => accountOf(a) === id)));
  return ids.length && times.every((x) => x !== null) ? Math.min(...(times as number[])) : null;
}

/** « Les comptes Principal et Pro ont passé le seuil. », under the pause: nothing after a usage limit, nor with a single account. */
export function pausedAccounts(p: AutopilotPause): string {
  if (p.reason === 'limit' || !p.accounts?.length) return '';
  const names = p.accounts.map((id) => {
    const a = app.settings.accounts.find((x) => x.id === id);
    return a ? accountName(a) : id;
  });
  return t('accounts.pause.over', { count: names.length, accounts: fList(names) });
}

/** Back to the « Comptes Claude » tab of the settings, their draft as it was. */
export function backToAccounts() {
  app.modal = { kind: 'settings', tab: 'accounts', resume: true };
}
