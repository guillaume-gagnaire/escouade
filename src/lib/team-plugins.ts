// The team's private store (TS-14, `src-tauri/src/teamplugins.rs`): its key and its namespace, the plugins it
// published, the proposals waiting for an administrator's signature. Installing a team's plugin is the member's gesture:
// `inspect` reads it, the window of its permissions shows it, `plugin_install` installs it. Pure functions here, the
// screens in `components/team/TeamPlugins.svelte` and `components/plugins/TeamStore.svelte`.

import { invoke } from './transport';
import type { PluginPreview } from './plugins';

export interface KeyInfo {
  namespace: string;
  /** The ids of the key of every day and of the backup key, as minisign prints them. */
  ids: string[];
  /** The ids the team's revocation list revoked. */
  revoked: string[];
  by: string;
  at: number;
  /** This device keeps the sealed key: it can sign. */
  here: boolean;
}

export interface TeamPluginRow {
  id: string;
  name: string;
  description: string;
  version: string;
  by: string;
  signedAt: number;
  digest: string;
  /** The version installed here from this team. */
  installed: string | null;
  /** Installed here from another origin: never replaced by the team's. */
  elsewhere: boolean;
  /** Why the team's list withdraws this version. */
  withdrawn: string | null;
}

export interface Checked {
  key: string;
  ok: boolean;
  detail: string;
}

export interface ProposalRow {
  id: string;
  plugin: string;
  name: string;
  version: string;
  tag: string;
  by: string;
  mine: boolean;
  at: number;
  digest: string;
  checks: Checked[];
}

export interface TeamPluginsView {
  team: string;
  name: string;
  admin: boolean;
  key: KeyInfo | null;
  /** A plugin installed here from another origin, in the team's namespace: the team's store is refused here. */
  conflict: string | null;
  plugins: TeamPluginRow[];
  proposed: ProposalRow[];
  serial: number | null;
  refused: number;
  alarm: boolean;
}

export interface Report {
  id: string;
  version: string;
  archiveSha256: string;
  moduleSha256: string | null;
  digest: string;
  checks: Checked[];
  ok: boolean;
}

export interface Submitted {
  report: Report;
  proposal: string | null;
  view: TeamPluginsView | null;
}

export const teamPluginsApi = {
  get: (team: string) => invoke<TeamPluginsView>('team_plugins', { team }),
  all: () => invoke<TeamPluginsView[]>('team_plugins_all'),
  createKey: (team: string, namespace: string, password: string, backupDir: string) =>
    invoke<{ view: TeamPluginsView; backup: string }>('team_plugin_key_create', { team, namespace, password, backupDir }),
  importKey: (team: string, file: string, password: string) => invoke<TeamPluginsView>('team_plugin_key_import', { team, file, password }),
  submit: (team: string, folder: string, tag: string) => invoke<Submitted>('team_plugin_submit', { team, folder, tag }),
  sign: (team: string, proposal: string, password: string, compileHere: boolean) =>
    invoke<{ report: Report; view: TeamPluginsView }>('team_plugin_sign', { team, proposal, password, compileHere }),
  revoke: (team: string, plugin: string, reason: string, password: string, versions: string[] | null = null) =>
    invoke<TeamPluginsView>('team_plugin_revoke', { team, plugin, versions, reason, password, revokeKey: null }),
  dismiss: (team: string, proposal: string) => invoke<TeamPluginsView>('team_plugin_dismiss', { team, proposal }),
  retry: (team: string) => invoke<TeamPluginsView>('team_plugins_retry', { team }),
  inspect: (team: string, id: string, version: string | null = null) => invoke<PluginPreview>('team_plugin_inspect', { team, id, version }),
};

/** A namespace as the app takes it: a reversed domain of two parts at least, then `.*`, never Escouade's own. */
export function namespaceOk(raw: string): 'ok' | 'form' | 'firstParty' {
  const ns = raw.trim().toLowerCase();
  if (!ns.endsWith('.*')) return 'form';
  const labels = ns.slice(0, -2).split('.');
  if (!labels.every((l) => /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(l))) return 'form';
  const prefix = labels.join('.');
  const under = (a: string, b: string) => a === b || a.startsWith(`${b}.`);
  if (under(prefix, 'dev.escouade') || under('dev.escouade', prefix)) return 'firstParty';
  return labels.length >= 2 ? 'ok' : 'form';
}

/** What a plugin row offers the member: install it, update it, nothing (installed, withdrawn, or of another origin). */
export function rowAction(r: TeamPluginRow): 'install' | 'update' | 'installed' | 'withdrawn' | 'elsewhere' {
  if (r.elsewhere) return 'elsewhere';
  if (r.withdrawn) return 'withdrawn';
  if (!r.installed) return 'install';
  return r.installed.localeCompare(r.version, undefined, { numeric: true }) < 0 ? 'update' : 'installed';
}

/** « Acme · privé »: how a team's plugin says where it comes from. */
export function privateLabel(name: string, word: string): string {
  return `${name} · ${word}`;
}
