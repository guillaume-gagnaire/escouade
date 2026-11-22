// The administrator's circuit breaker (TS-15, `src-tauri/src/teams_breakers.rs`): what the band and the team's window
// read. Pure functions here, the screens in `components/team`.

import { invoke } from './transport';
import { fDateTime } from './format';
import { t } from './i18n';

export type Scope = { kind: 'team' } | { kind: 'project'; project: string; name: string };

export type What =
  { kind: 'plugin'; id: string; uninstall: boolean } | { kind: 'harness'; id: string } | { kind: 'model'; id: string } | { kind: 'agents' };

export interface Breaker {
  id: string;
  scope: Scope;
  what: What;
  reason: string;
  by: string;
  member: string;
  /** Seconds since the epoch. */
  at: number;
  serial: number;
  seq: number;
}

export interface Lifted {
  breaker: Breaker;
  by: string;
  member: string;
  at: number;
}

export type Refused =
  | { kind: 'notAdmin' | 'unsigned' | 'unknown' }
  | { kind: 'replayed'; serial: number; highest: number }
  | { kind: 'bad' | 'unreadable'; why: string };

export interface BreakerState {
  active: Breaker[];
  lifted: Lifted[];
  refusals: { seq: number; reason: Refused }[];
  alarm: 'rollback' | 'rewritten' | 'server' | 'fork' | null;
  serial: number;
}

export interface BreakersView {
  team: string;
  name: string;
  admin: boolean;
  view: BreakerState;
  signerName: string;
  /** When the team's server last answered this device (seconds); null when it never did. */
  lastContact: number | null;
  /** The plugins the team's revocation list withdraws in every version (TS-14): the circuit breaker reads them. */
  withdrawn: { id: string; reason: string }[];
}

export interface BreakerAsk {
  scope: Scope;
  what: What;
  reason: string;
  by: string;
}

export const teamBreakersApi = {
  get: (team: string) => invoke<BreakersView>('team_breakers', { team }),
  online: (team: string) => invoke<number>('team_breaker_online', { team }),
  set: (team: string, ask: BreakerAsk) => invoke<BreakersView>('team_breaker_set', { team, ask }),
  lift: (team: string, breaker: string, by: string) => invoke<BreakersView>('team_breaker_lift', { team, breaker, by }),
  retry: (team: string) => invoke<BreakersView>('team_breakers_retry', { team }),
};

/** « le harness Codex », « le modèle gpt-5.5 », « le plugin acme.rules », « tous les agents ». */
export function whatText(what: What, names: (engine: string) => string = (e) => e): string {
  switch (what.kind) {
    case 'harness':
      return t('teamBreakers.what.harnessOf', { name: names(what.id) });
    case 'model':
      return t('teamBreakers.what.modelOf', { name: what.id });
    case 'plugin':
      return what.uninstall
        ? t('teamBreakers.what.pluginUninstalledOf', { name: what.id })
        : t('teamBreakers.what.pluginOf', { name: what.id });
    case 'agents':
      return t('teamBreakers.what.agentsOf');
  }
}

/** « toute l'équipe Acme », « le projet API de l'équipe Acme ». */
export function scopeText(scope: Scope, team: string): string {
  return scope.kind === 'team'
    ? t('teamBreakers.scope.teamOf', { team })
    : t('teamBreakers.scope.projectOf', { project: scope.name || scope.project.slice(0, 8), team });
}

/** What stops when the breaker is set, as the confirmation says it. */
export function stopsText(what: What, names: (engine: string) => string = (e) => e): string {
  switch (what.kind) {
    case 'harness':
      return t('teamBreakers.stops.harness', { name: names(what.id) });
    case 'model':
      return t('teamBreakers.stops.model', { name: what.id });
    case 'plugin':
      return what.uninstall
        ? t('teamBreakers.stops.pluginUninstall', { name: what.id })
        : t('teamBreakers.stops.plugin', { name: what.id });
    case 'agents':
      return t('teamBreakers.stops.agents');
  }
}

/** What the band says of a team: its breakers, or the policy « sans contact depuis N heures » that holds its agents. */
export interface Band {
  team: string;
  /** « Arrêt demandé par Claire Martin : fuite de clé chez le fournisseur X » */
  text: string;
  /** « Le harness Codex est retiré · toute l'équipe Acme » (and « · et 2 autres coupe-circuits »). */
  detail: string;
  kind: 'breaker' | 'contact';
}

/** The band of the first team with a breaker in force (the most recent first), else of a team whose server has been
 *  silent past the policy `maxOfflineHours` (« sans contact depuis N heures »); null when none. `now` in milliseconds. */
export function bandOf(
  views: BreakersView[],
  maxOfflineHours: number | null,
  now: number,
  names: (engine: string) => string = (e) => e,
): Band | null {
  for (const v of views) {
    const [first, ...rest] = v.view.active;
    if (!first) continue;
    const detail = [
      t('teamBreakers.band.cut', { what: whatText(first.what, names) }),
      scopeText(first.scope, v.name),
      ...(rest.length ? [t('teamBreakers.band.more', { count: rest.length })] : []),
    ].join(' · ');
    return {
      team: v.team,
      text: t('teamBreakers.band.asked', { by: first.by, reason: first.reason }),
      detail,
      kind: 'breaker',
    };
  }
  if (maxOfflineHours) {
    for (const v of views) {
      const last = v.lastContact;
      if (last !== null && now - last * 1000 <= maxOfflineHours * 3600_000) continue;
      return {
        team: v.team,
        text: t('teamBreakers.band.contact', { team: v.name, hours: maxOfflineHours }),
        detail: last === null ? t('teamBreakers.band.never') : t('teamBreakers.band.lastContact', { at: fDateTime(last * 1000) }),
        kind: 'contact',
      };
    }
  }
  return null;
}
