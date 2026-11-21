import { invoke } from './transport';
import type { FieldSetting } from './plugin-fields';
import { activeIn, textOf, type Manifest, type ManifestText, type PluginView, type SecretState } from './plugins';
import type { Project } from './types';

// A plugin's profiles and its settings of each project (ESC-530, issues #38 and #39; docs/PLUGINS-CONTRACT-STUDY.md §2.2 and
// §3.1). A profile (`contributes.profiles`) is made, changed and deleted on the plugin's page; each project chooses its own in
// Réglages › Projets › Plugins, where the plugin's settings of each project (`"scope": "project"`) are set too.
// What is read here: the kinds and their fields, a profile's name and summary, the projects that take it, the plugins a
// project's page shows, and where each of its values comes from (this machine, `.escouade.yml`, the default).

/** A field of a kind of profile: a simple one (a `type`), or a secret one (`secret: true`). */
export interface ProfileField {
  key: string;
  type?: 'string' | 'number' | 'boolean' | 'select';
  secret?: boolean;
  title?: ManifestText;
  description?: ManifestText;
  default?: unknown;
  options?: (string | { value: string; title?: ManifestText })[];
  multiline?: boolean;
  lines?: number;
  import?: string[];
}

/** A kind of profile (`contributes.profiles[]`). */
export interface ProfileKind {
  id: string;
  title?: ManifestText;
  description?: ManifestText;
  nameField: string;
  fields: ProfileField[];
}

/** A profile the user made: its id drawn by Escouade, its simple fields. */
export interface Profile {
  id: string;
  values: Record<string, unknown>;
}

/** The secrets of a profile, kept or not, and where. */
export interface ProfileSecrets {
  kind: string;
  profile: string;
  secrets: SecretState[];
}

export const profilesApi = {
  secrets: (id: string) => invoke<ProfileSecrets[]>('plugin_profile_secrets', { id }),
  /** « Enregistrer »: a new one when `profile` is null. The plugin's view and the profile's id. */
  save: (id: string, kind: string, profile: string | null, values: Record<string, unknown>) =>
    invoke<[PluginView, string]>('plugin_profile_save', { id, kind, profile, values }),
  /** « Garder » (empty: « Oublier ») of a secret field of a profile. Rejects with a `SecretRefused`. */
  setSecret: (id: string, kind: string, profile: string, key: string, value: string, inFile = false) =>
    invoke<void>('plugin_profile_secret', { id, kind, profile, key, value, inFile }),
  /** « Supprimer le profil »: the plugin's view, and the projects that took it. */
  remove: (id: string, kind: string, profile: string) =>
    invoke<[PluginView, { projects: string[] }]>('plugin_profile_delete', { id, kind, profile }),
  /** A setting of each project set on this machine; `clear`: forgotten (the file's or the default's again). */
  setProject: (id: string, project: string, key: string, value: unknown, clear = false) =>
    invoke<PluginView>('plugin_project_set', { id, project, key, value: value ?? null, clear }),
  /** « Écrire dans .escouade.yml » of a setting of each project: the plugin's view and the project. */
  writeProject: (id: string, project: string, key: string) => invoke<[PluginView, Project]>('plugin_project_write', { id, project, key }),
};

/** The kinds of profile of a manifest. */
export function kindsOf(m: Manifest | null): ProfileKind[] {
  const list = m?.contributes.profiles;
  return Array.isArray(list) ? (list as ProfileKind[]).filter((k) => k && typeof k.id === 'string' && Array.isArray(k.fields)) : [];
}

/** The profiles of a kind the user made, in their order. */
export const profilesOf = (p: PluginView, kind: string): Profile[] => p.choices.profiles?.[kind] ?? [];

/** A profile's name: its `nameField`, else its id. */
export function nameOf(kind: ProfileKind, p: Profile): string {
  const n = p.values[kind.nameField];
  return typeof n === 'string' && n.trim() ? n.trim() : p.id;
}

/** What a profile's row says under its name: the values of its other simple fields, the first three, « · » between. */
export function summaryOf(kind: ProfileKind, p: Profile): string {
  return kind.fields
    .filter((f) => !f.secret && f.key !== kind.nameField && !f.multiline)
    .map((f) => p.values[f.key])
    .filter((v) => v !== undefined && v !== null && v !== '' && typeof v !== 'boolean')
    .slice(0, 3)
    .map(String)
    .join(' · ');
}

/** The settings of each project a manifest declares (`"scope": "project"`), in its order. */
export function projectFields(m: Manifest | null): FieldSetting[] {
  const s = m?.contributes.settings;
  return Array.isArray(s) ? (s as FieldSetting[]).filter((f) => f?.scope === 'project') : [];
}

/** The ids of the projects that chose the profile `profile` (a `profile` setting of each project of its kind). */
export function usedBy(p: PluginView, kind: string, profile: string): string[] {
  const keys = projectFields(p.manifest)
    .filter((f) => f.type === 'profile' && f.of === kind)
    .map((f) => f.key);
  return Object.entries(p.choices.projectSettings ?? {})
    .filter(([, values]) => keys.some((k) => values[k] === profile))
    .map(([project]) => project);
}

/** The plugins the page Plugins of a project shows: installed, on in this project, with a setting of each project (§3.1). */
export function projectPlugins(list: PluginView[], project: Pick<Project, 'id' | 'private'>): PluginView[] {
  return list.filter((p) => p.state.kind === 'ready' && activeIn(p, project) && projectFields(p.manifest).length > 0);
}

/** What the project's `.escouade.yml` gives the plugin (`plugins.<id>`). */
export function fileValues(project: Project | undefined | null, id: string): Record<string, unknown> {
  const file = project?.shared?.file;
  if (!file || file.error) return {};
  const v = file.values?.[`plugins.${id}`];
  return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
}

/** Whether a setting may come from `.escouade.yml` and be written there: a simple one, not `repo: false`, never imported. */
export const fromRepo = (f: FieldSetting) => f.type !== 'profile' && f.repo !== false && !f.import;

/** Whether `v` is a value of the setting's type (a `select` among its options). */
export function fits(f: FieldSetting, v: unknown): boolean {
  if (f.type === 'string') return typeof v === 'string';
  if (f.type === 'number') return typeof v === 'number';
  if (f.type === 'boolean') return typeof v === 'boolean';
  if (f.type === 'select')
    return typeof v === 'string' && (!f.options || f.options.some((o) => (typeof o === 'string' ? o : o.value) === v));
  return false;
}

/** Where a value of a setting of each project comes from: imposed by the team (rank 5), this machine (4, ahead of the
 *  file), the file (3), recommended by the team (2), or its default (1). */
export type Source = 'enforced' | 'local' | 'file' | 'recommended' | 'default';

/** A setting of each project, as the page of a project shows it: its value, where it comes from, what the file says. */
export interface ProjectValue {
  value: unknown;
  source: Source;
  /** The file's value, when the file gives one that may hold (a simple setting, not `repo: false`). */
  file?: unknown;
  /** What the team recommends, when it does with a value that holds. */
  recommended?: unknown;
  /** The team's name, when it imposes or recommends the value. */
  team?: string;
}

/** Why a team's key of a plugin is refused (`profiles::team_refusal`). */
export type TeamRefusal = 'unknown' | 'secret' | 'profile' | 'imported' | 'keptOff' | 'appScope' | 'type';

/** Whether the project's team may set the setting `key` of the plugin to `value` (TS-13, ESC-532): a simple setting of each
 *  project, a value of its type; never a secret, a profile, a value imported, what is kept off the files. Null: it may. */
export function teamRefusal(p: PluginView, key: string, value: unknown): TeamRefusal | null {
  const c = p.manifest?.contributes;
  const secrets = Array.isArray(c?.secrets) ? (c.secrets as { key?: string }[]).map((s) => s?.key) : [];
  if (secrets.includes(key) || (p.manifest?.permissions?.secrets ?? []).includes(key)) return 'secret';
  const s = Array.isArray(c?.settings) ? (c.settings as FieldSetting[]) : [];
  const f = s.find((x) => x?.key === key);
  if (!f) return 'unknown';
  if (f.type === 'profile') return 'profile';
  if (f.import) return 'imported';
  if (f.repo === false) return 'keptOff';
  if (f.scope !== 'project') return 'appScope';
  if (!fits(f, value)) return 'type';
  return null;
}

/** What the project's team sets on the plugin `id`, by setting key; null when it sets nothing on it. */
export function teamValues(
  project: Project | undefined | null,
  id: string,
): { team: string; defaults: Record<string, unknown>; enforced: Record<string, unknown> } | null {
  const t = project?.shared?.teamPlugins;
  if (!t) return null;
  const prefix = `plugins.${id}.`;
  const pick = (m: Record<string, unknown>) =>
    Object.fromEntries(
      Object.entries(m ?? {}).flatMap(([k, v]) => (k.startsWith(prefix) && k.length > prefix.length ? [[k.slice(prefix.length), v]] : [])),
    );
  const defaults = pick(t.defaults);
  const enforced = pick(t.enforced);
  return Object.keys(defaults).length || Object.keys(enforced).length ? { team: t.team, defaults, enforced } : null;
}

/** The keys the project's team sets on the plugin that will not apply, with why. */
export function teamRefusals(p: PluginView, project: Project): { key: string; part: 'defaults' | 'enforced'; why: TeamRefusal }[] {
  const t = teamValues(project, p.id);
  if (!t) return [];
  return (['enforced', 'defaults'] as const).flatMap((part) =>
    Object.entries(t[part]).flatMap(([key, v]) => {
      const why = teamRefusal(p, key, v);
      return why ? [{ key, part, why }] : [];
    }),
  );
}

/** The value of a simple setting of each project in `project`, by its ranks: imposed by the team, this machine's, the
 *  file's, recommended by the team, the default. */
export function projectValue(p: PluginView, project: Project, f: FieldSetting): ProjectValue {
  const local = p.choices.projectSettings?.[project.id] ?? {};
  const file = fileValues(project, p.id);
  const team = teamValues(project, p.id);
  const byTeam = (part: 'defaults' | 'enforced') =>
    team && f.key in team[part] && teamRefusal(p, f.key, team[part][f.key]) === null ? team[part][f.key] : undefined;
  const enforced = byTeam('enforced');
  const recommended = byTeam('defaults');
  const fromFile = fromRepo(f) && f.key in file && fits(f, file[f.key]) ? file[f.key] : undefined;
  const extra = { ...(recommended !== undefined ? { recommended } : {}), ...(team ? { team: team.team } : {}) };
  if (enforced !== undefined) return { value: enforced, source: 'enforced', file: fromFile, ...extra };
  if (f.key in local && fits(f, local[f.key])) return { value: local[f.key], source: 'local', file: fromFile, ...extra };
  if (fromFile !== undefined) return { value: fromFile, source: 'file', file: fromFile, ...extra };
  if (recommended !== undefined) return { value: recommended, source: 'recommended', ...extra };
  return { value: f.default ?? (f.type === 'boolean' ? false : ''), source: 'default' };
}

/** The profile a project chose for a `profile` setting: its id, null for « Aucun profil », undefined when it chose none. */
export function chosenProfile(p: PluginView, project: string, key: string): string | null | undefined {
  const local = p.choices.projectSettings?.[project] ?? {};
  if (!(key in local)) return undefined;
  const v = local[key];
  return typeof v === 'string' ? v : null;
}

/** The profile of an app's `profile` setting of the kind `kind` (« Profil par défaut »), if any. */
export function defaultProfile(p: PluginView, kind: string): { key: string; id: string | null } | null {
  const s = p.manifest?.contributes.settings;
  const d = (Array.isArray(s) ? (s as FieldSetting[]) : []).find((f) => f.type === 'profile' && f.of === kind && f.scope !== 'project');
  if (!d) return null;
  const v = p.choices.settings[d.key];
  const id = typeof v === 'string' && profilesOf(p, kind).some((x) => x.id === v) ? v : null;
  return { key: d.key, id };
}

/** The name a project's `.escouade.yml` proposes for a `profile` setting (a proposal, never a choice), if any. */
export function proposedName(project: Project | undefined | null, id: string, f: FieldSetting): string | null {
  const v = fileValues(project, id)[f.key];
  return typeof v === 'string' && v.trim() ? v.trim() : null;
}

/** The profile of the kind whose name is the one proposed (case and spaces at its ends aside). */
export function proposedProfile(p: PluginView, kind: ProfileKind, name: string): Profile | null {
  const want = name.trim().toLowerCase();
  return profilesOf(p, kind.id).find((x) => nameOf(kind, x).toLowerCase() === want) ?? null;
}

/** A text of a manifest in a language, with a fallback. */
export const titleOf = (t: ManifestText | undefined, lang: string, fallback: string) => textOf(t, lang) || fallback;
