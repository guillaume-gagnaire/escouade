// The Claude accounts as the window shows them.

import { t, tIn } from './i18n';
import { app } from './state.svelte';
import type { Account } from './types';

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

/** Back to the « Comptes Claude » tab of the settings, their draft as it was. */
export function backToAccounts() {
  app.modal = { kind: 'settings', tab: 'accounts', resume: true };
}
