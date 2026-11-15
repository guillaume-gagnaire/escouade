// The team's signed `.escouade.yml` on this device (TS-13, `src-tauri/src/teams_settings.rs`): what the band, the
// history, the consents and the administrator's screen read. Pure functions here, the screens in `components/team`.

import { invoke } from '@tauri-apps/api/core';
import { fAgo } from './format';
import { t } from './i18n';

export type Part = 'defaults' | 'enforced' | 'proposed';
export type Sort = 'restrict' | 'value' | 'sensitive' | 'executable';

export interface TeamFile {
  version: number;
  defaults: Record<string, unknown>;
  enforced: Record<string, unknown>;
  proposed: Record<string, unknown>;
  reasons: Record<string, string>;
  quorum: boolean | null;
  signers: string[];
}

export type Refused =
  | { kind: 'unsigned' | 'notAdmin' | 'removedAdmin' | 'badChain' | 'behind' | 'replayed' | 'lost' }
  | { kind: 'never'; hit: { rule: string; part: Part; key: string } }
  | { kind: 'unreadable'; why: string };

export type Status = { kind: 'applied' | 'superseded' | 'proposed' } | { kind: 'refused'; reason: Refused };

export interface VersionView {
  number: number;
  digest: string;
  by: string;
  author: string;
  /** Seconds since the epoch. */
  at: number;
  note: string;
  file: TeamFile;
  signers: string[];
  need: number;
  signedByMe: boolean;
  status: Status;
}

export interface Pending {
  part: Part;
  key: string;
  value: unknown;
  sort: Sort;
  fingerprint: string;
}

export interface Effective {
  defaults: Record<string, unknown>;
  enforced: Record<string, unknown>;
  accepted: Record<string, unknown>;
  pending: Pending[];
  unknown: string[];
  reasons: Record<string, string>;
}

export interface SettingsView {
  applied: number | null;
  versions: VersionView[];
  refusals: { seq: number; reason: Refused }[];
  alarm: 'rollback' | 'rewritten' | 'server' | 'fork' | null;
  quorum: boolean;
  admins: number;
  effective: Effective;
}

export interface TeamSettingsView {
  team: string;
  name: string;
  admin: boolean;
  me: string;
  view: SettingsView;
  yaml: string;
  seen: number | null;
  signerName: string;
}

export interface SettingsDraft {
  file: TeamFile | null;
  warnings: string[];
  error: string | null;
  need: number;
}

export const teamSettingsApi = {
  get: (team: string) => invoke<TeamSettingsView>('team_settings', { team }),
  check: (team: string, yaml: string) => invoke<SettingsDraft>('team_settings_check', { team, yaml }),
  propose: (team: string, yaml: string, note: string, by: string) =>
    invoke<TeamSettingsView>('team_settings_propose', { team, yaml, note, by }),
  cosign: (team: string, digest: string) => invoke<TeamSettingsView>('team_settings_cosign', { team, digest }),
  consent: (team: string, part: Part, key: string, fingerprint: string) =>
    invoke<TeamSettingsView>('team_settings_consent', { team, part, key, fingerprint }),
  withdraw: (team: string, part: Part, key: string) => invoke<TeamSettingsView>('team_settings_withdraw', { team, part, key }),
  seen: (team: string, number: number) => invoke<void>('team_settings_seen', { team, number }),
  retry: (team: string) => invoke<TeamSettingsView>('team_settings_retry', { team }),
  setProjectTeam: (project: string, team: string | null) => invoke<unknown>('project_set_team', { project, team }),
};

// ---------- the diff ----------

export interface Change {
  /** A part of the file, or `team` for the quorum and the signers. */
  part: Part | 'team';
  key: string;
  kind: 'added' | 'removed' | 'changed';
  before?: unknown;
  after?: unknown;
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** What changes from `before` (none: the first version) to `after`, part by part, key by key, in order. */
export function diff(before: TeamFile | null, after: TeamFile): Change[] {
  const out: Change[] = [];
  for (const part of ['enforced', 'defaults', 'proposed'] as const) {
    const a = before?.[part] ?? {};
    const b = after[part] ?? {};
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
    for (const key of keys) {
      const had = key in a;
      const has = key in b;
      if (had && !has) out.push({ part, key, kind: 'removed', before: a[key] });
      else if (!had && has) out.push({ part, key, kind: 'added', after: b[key] });
      else if (!same(a[key], b[key])) out.push({ part, key, kind: 'changed', before: a[key], after: b[key] });
    }
  }
  if ((before?.quorum ?? null) !== (after.quorum ?? null))
    out.push({
      part: 'team',
      key: 'quorum',
      kind: before ? 'changed' : 'added',
      before: before?.quorum ?? null,
      after: after.quorum ?? null,
    });
  if (!same(before?.signers ?? [], after.signers ?? []))
    out.push({ part: 'team', key: 'signers', kind: before ? 'changed' : 'added', before: before?.signers ?? [], after: after.signers });
  return out;
}

/** A value as the window writes it: JSON, short. */
export function shown(v: unknown): string {
  if (v === undefined) return '—';
  return typeof v === 'string' ? v : JSON.stringify(v);
}

// ---------- the history ----------

/** The versions of the chain (in force, or before it), the newest first. */
export function chain(v: SettingsView): VersionView[] {
  return v.versions.filter((x) => x.status.kind === 'applied' || x.status.kind === 'superseded').sort((a, b) => b.number - a.number);
}

/** The version of the chain at `number`. */
export function inChain(v: SettingsView, number: number | null): VersionView | null {
  if (number === null) return null;
  return chain(v).find((x) => x.number === number) ?? null;
}

/** The version in force. */
export function applied(v: SettingsView): VersionView | null {
  return inChain(v, v.applied);
}

/** What a version changed from the one before it in the chain (none before: everything it sets). */
export function changesOf(v: SettingsView, version: VersionView): Change[] {
  return diff(inChain(v, version.number - 1)?.file ?? null, version.file);
}

/** « En vigueur », « Remplacée », « En attente d'une seconde signature (1/2) », « Refusée : … ». */
export function statusLabel(version: VersionView): string {
  const s = version.status;
  if (s.kind === 'refused') return t('team.settings.status.refused', { why: refusedText(s.reason) });
  if (s.kind === 'applied') return t('team.settings.status.applied');
  if (s.kind === 'superseded') return t('team.settings.status.superseded');
  return t('team.settings.status.proposed', { have: version.signers.length, need: version.need });
}

export function refusedText(r: Refused): string {
  if (r.kind === 'never') return t('team.settings.refused.never', { key: `${r.hit.part}.${r.hit.key}` });
  if (r.kind === 'unreadable') return t('team.settings.refused.unreadable');
  return t(`team.settings.refused.${r.kind}`);
}

// ---------- the band ----------

export interface Band {
  team: string;
  name: string;
  /** The version the band shows: « Voir » marks it seen. */
  number: number;
  changes: number;
  by: string;
  at: number;
  /** What waits for this device's consent. */
  pending: number;
}

/** The band of a team: a version in force the member has not seen yet, with what it changed since the one seen; or
 *  what waits for this device's consent. None when there is nothing new. */
export function bandOf(s: TeamSettingsView): Band | null {
  const now = applied(s.view);
  const pending = s.view.effective.pending.length;
  if (!now) return null;
  const fresh = now.number > (s.seen ?? 0);
  const changes = fresh ? diff(inChain(s.view, s.seen)?.file ?? null, now.file).length : 0;
  if (changes === 0 && pending === 0) return null;
  return { team: s.team, name: s.name, number: now.number, changes, by: now.by, at: now.at, pending };
}

/** « L'équipe Acme a changé 2 réglages · Claire Martin, il y a 5 min ». */
export function bandText(b: Band, nowMs: number): string {
  const parts: string[] = [];
  if (b.changes > 0) {
    parts.push(t('team.settings.band.changed', { team: b.name, count: b.changes }));
    parts.push(t('team.settings.band.by', { by: b.by, ago: fAgo(b.at, nowMs) }));
  }
  if (b.pending > 0) parts.push(t('team.settings.band.pending', { team: b.name, count: b.pending }));
  return parts.join(' · ');
}

// ---------- what a consent allows ----------

/** What accepting a key allows, in plain words: « Ce certificat permet de lire le trafic TLS d'Escouade ». */
export function allows(p: Pick<Pending, 'key' | 'sort'>): string {
  switch (p.key) {
    case 'proxy.url':
      return t('team.settings.allows.proxyUrl');
    case 'proxy.authorities':
      return t('team.settings.allows.authorities');
    case 'proxy.exclusions':
      return t('team.settings.allows.exclusions');
  }
  return p.sort === 'executable' ? t('team.settings.allows.executable') : t('team.settings.allows.sensitive');
}
