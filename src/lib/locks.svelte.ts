// The settings imposed by the policies of this device (rank 6) or by a team (rank 5), as the core tells them (TS-11,
// `locks.rs`): a row of the register they lock is greyed, with a padlock and who imposes it; a choice a bound denies is
// greyed, with the reason. The core refuses the writes all the same: this only shows it.

import { t } from './i18n';
import type { LockedBy, Locks } from './types';

export const NO_LOCKS: Locks = { locks: {}, rows: {}, warnings: [] };

/** The locks the window knows: `InitialState.locks`, then each `locks` event; `teams`, those of each team's projects
 *  (TS-13), by team id. */
export const locks = $state<{ current: Locks; teams: Record<string, Locks> }>({ current: NO_LOCKS, teams: {} });

export function setLocks(l: Locks | undefined, teams?: Record<string, Locks>): void {
  locks.current = l ?? NO_LOCKS;
  locks.teams = teams ?? {};
}

/** The locks of a project: its team's when it is of one (what a team imposes holds for its projects alone), the app's
 *  otherwise. */
export function locksFor(project: { team?: string | null } | null | undefined): Locks {
  const team = project?.team;
  return (team && locks.teams[team]) || locks.current;
}

/** Who imposes the value of a row of `SETTINGS_ROWS` (its `locked_by`); null when nothing does. */
export function lockedBy(row: string, l: Locks = locks.current): LockedBy | null {
  for (const key of l.rows[row] ?? []) {
    const lock = l.locks[key];
    if (lock?.by && lock.value !== null && lock.value !== undefined) return lock.by;
  }
  return null;
}

/** The key of the file of the plugins' secrets (ESC-520): off, « Garder dans un fichier » is no longer offered. */
export const SECRETS_FILE_KEY = 'secrets.fileFallback';

/** Who imposes the key `key` of the core's table, for one that locks no row of the register (`secrets.fileFallback`);
 *  null when nothing does. */
export function keyLockedBy(key: string, l: Locks = locks.current): LockedBy | null {
  const lock = l.locks[key];
  return lock?.by && lock.value !== null && lock.value !== undefined ? lock.by : null;
}

/** « Imposé par la stratégie de ce poste », « Imposé par l'équipe Acme ». */
export function lockLine(by: LockedBy): string {
  return by.kind === 'team' ? t('settings.locks.byTeam', { team: by.team }) : t('settings.locks.byDevice');
}

/** Who denies `choice` of the bound `key` (the device first, then the team); null when every bound allows it. */
export function deniedBy(key: string, choice: string, l: Locks = locks.current): LockedBy | null {
  const bounds = [...(l.locks[key]?.bounds ?? [])].sort((a, b) => Number(b.by.kind === 'device') - Number(a.by.kind === 'device'));
  return bounds.find((b) => !b.allowed.includes(choice))?.by ?? null;
}

/** « non permis par la stratégie de ce poste »: why a choice is greyed; null when it is allowed. */
export function denial(key: string, choice: string, l: Locks = locks.current): string | null {
  const by = deniedBy(key, choice, l);
  if (!by) return null;
  return by.kind === 'team' ? t('settings.locks.deniedTeam', { team: by.team }) : t('settings.locks.deniedDevice');
}

/** The keys of the core's table the window reads (`locks::KEYS`). */
export const MODES_KEY = 'permissions.modes';
export const HARNESSES_KEY = 'harnesses.allowed';

/** The choices of a control bounded by `key`: a choice a bound denies greyed, with the reason in its tip (« GPT-5.5 ·
 *  non permis par l'équipe Acme »). The choice already set stays shown as it is. */
export function bounded<T extends { value: string; label: string; title?: string; disabled?: boolean }>(
  key: string,
  options: T[],
  l: Locks = locks.current,
): T[] {
  return options.map((o) => {
    const why = denial(key, o.value, l);
    return why ? { ...o, disabled: true, title: `${o.label} · ${why}` } : o;
  });
}
