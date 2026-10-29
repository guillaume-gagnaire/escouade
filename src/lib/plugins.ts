// The plugins (PLG-4, ESC-176; docs/PLUGINS.md §3): their views as the core gives them, the calls of the view Plugins,
// and what the view reads of them — what each may do in words, the strongest first, its category, its projects.
import { invoke } from '@tauri-apps/api/core';
import { t } from './i18n';
import type { Project } from './types';

export type Origin = { kind: 'folder'; path: string } | { kind: 'git'; url: string; tag: string } | { kind: 'dev'; path: string };

export type PluginState =
  | { kind: 'ready' }
  | { kind: 'disabled'; reason: string }
  | { kind: 'invalid'; reason: string; problems: { path: string; code: string; message: string }[] };

/** A text of a manifest: one for every language, or one by language (`{ fr, en }`). */
export type ManifestText = string | Record<string, string>;

export interface ManifestSetting {
  key: string;
  type: 'string' | 'number' | 'boolean' | 'select';
  title?: ManifestText;
  description?: ManifestText;
  default?: unknown;
  options?: (string | { value: string; title?: ManifestText })[];
}

export interface Manifest {
  id: string;
  name: ManifestText;
  version: string;
  description?: ManifestText;
  author?: string;
  license?: string;
  repository?: string;
  escouade: { api: number; min: string; max?: string };
  run?: { command: string; args: string[] };
  activation: string[];
  permissions: { events: string[]; network: string[]; secrets: string[]; commands: string[]; decide: boolean };
  contributes: Record<string, unknown>;
}

export interface Choices {
  enabled: boolean;
  projects: Record<string, boolean>;
  revoked: string[];
  settings: Record<string, unknown>;
  /** The fingerprint of the commands on facts the user agreed to (PLG-5); absent: none. */
  commandsApproved?: string;
}

export interface PluginUpdate {
  version: string;
  tag?: string;
}

export interface PluginView {
  id: string;
  name: string;
  version: string;
  description: string;
  origin: Origin;
  fingerprint: string;
  folder: string;
  declarative: boolean;
  state: PluginState;
  ignored: string[];
  manifest: Manifest | null;
  installedAt: string;
  choices: Choices;
  asks: string[];
  secretKeys: string[];
  update: PluginUpdate | null;
  /** The command lines it runs on facts (`contributes.commands`, PLG-5). */
  commands: string[];
  /** Whether they were agreed to as they are now (true without any): none runs before. */
  commandsApproved: boolean;
}

/** A plugin read before it is installed: the install window's. */
export interface PluginPreview {
  token: string;
  /** Its icon (PLG-13), a `data:image/png` address read in what was set aside; null: its letters. */
  icon: string | null;
  id: string;
  name: string;
  version: string;
  description: string;
  origin: Origin;
  fingerprint: string;
  declarative: boolean;
  refusal: string | null;
  asks: string[];
  previous: { version: string; fingerprint: string; asks: string[] } | null;
  grown: string[];
  manifest: Manifest;
}

/** Why a plugin was not installed. */
export interface PluginRefusal {
  message: string;
  problems: { path: string; code: string; message: string }[];
}

export type Phase = 'idle' | 'starting' | 'running' | 'restarting' | 'stopped';

export interface PluginRun {
  status: {
    phase: Phase;
    stop?: { kind: 'asked' | 'shutdown' | 'crashed' | 'memory' | 'refused'; times?: number; reason?: string };
    restarts: number;
    pid?: number;
  };
  journal: JournalEntry[];
}

export type JournalKind = 'start' | 'ready' | 'stop' | 'crash' | 'stderr' | 'message' | 'timeout' | 'refused' | 'dropped';

export interface JournalEntry {
  at: string;
  kind: JournalKind;
  text: string;
}

export const pluginsApi = {
  list: () => invoke<PluginView[]>('plugins_list'),
  inspectFolder: (path: string) => invoke<PluginPreview>('plugin_inspect_folder', { path }),
  inspectGit: (url: string, tag: string) => invoke<PluginPreview>('plugin_inspect_git', { url, tag }),
  inspectDev: (path: string) => invoke<PluginPreview>('plugin_inspect_dev', { path }),
  install: (token: string) => invoke<PluginView>('plugin_install', { token }),
  discard: (token: string) => invoke<void>('plugin_discard', { token }),
  remove: (id: string, keep: boolean) => invoke<boolean>('plugin_remove', { id, keep }),
  setEnabled: (id: string, on: boolean) => invoke<PluginView>('plugin_set_enabled', { id, on }),
  setProject: (id: string, project: string, on: boolean) => invoke<PluginView>('plugin_set_project', { id, project, on }),
  setPermission: (id: string, word: string, granted: boolean) => invoke<PluginView>('plugin_set_permission', { id, word, granted }),
  setSetting: (id: string, key: string, value: unknown) => invoke<PluginView>('plugin_set_setting', { id, key, value }),
  /** Agrees to the commands it runs on facts as they are now, or takes the agreement back (PLG-5). */
  approveCommands: (id: string, on: boolean) => invoke<PluginView>('plugin_approve_commands', { id, on }),
  secrets: (id: string) => invoke<[string, boolean][]>('plugin_secrets', { id }),
  setSecret: (id: string, key: string, value: string) => invoke<void>('plugin_set_secret', { id, key, value }),
  run: (id: string) => invoke<PluginRun>('plugin_run', { id }),
  start: (id: string) => invoke<PluginRun>('plugin_start', { id }),
  checkUpdates: () => invoke<PluginView[]>('plugins_check_updates'),
  /** The icon of an installed plugin (PLG-13), a `data:image/png` address; null: its letters. */
  icon: (id: string) => invoke<string | null>('plugin_icon', { id }),
};

/** A text of a manifest in the interface's language: English by default (§3.1). */
export function textOf(text: ManifestText | undefined, lang: string): string {
  if (text === undefined) return '';
  if (typeof text === 'string') return text;
  return text[lang] ?? text.en ?? Object.values(text)[0] ?? '';
}

/** A permission in words: its icon, what it says, its detail (a host, an event…), and how strong it is. */
export interface Ask {
  word: string;
  icon: 'term' | 'shield' | 'bash' | 'spark' | 'globe' | 'lock';
  text: string;
  detail: string;
  strong: boolean;
  red: boolean;
}

/** The title of a secret's field, or its key. */
function secretTitle(m: Manifest | null, key: string, lang: string) {
  const fields = (m?.contributes.secrets as { key: string; title?: ManifestText }[] | undefined) ?? [];
  return textOf(fields.find((f) => f.key === key)?.title, lang) || key;
}

/** What the permission `word` lets the plugin do, in words (§3.4). */
export function askOf(word: string, m: Manifest | null, lang: string): Ask {
  const [kind, ...rest] = word.split(':');
  const what = rest.join(':');
  switch (kind) {
    case 'run':
      return { word, icon: 'term', text: t('plugins.ask.run'), detail: m?.run?.command ?? '', strong: true, red: false };
    case 'decide':
      return { word, icon: 'shield', text: t('plugins.ask.decide'), detail: '', strong: true, red: true };
    case 'commands':
      return { word, icon: 'bash', text: t('plugins.ask.commands'), detail: what, strong: false, red: false };
    case 'events':
      return { word, icon: 'spark', text: t('plugins.ask.events'), detail: what, strong: false, red: false };
    case 'network':
      return { word, icon: 'globe', text: t('plugins.ask.network'), detail: what, strong: false, red: false };
    default:
      return {
        word,
        icon: 'lock',
        text: t('plugins.ask.secret', { name: secretTitle(m, what, lang) }),
        detail: '',
        strong: false,
        red: false,
      };
  }
}

/** The permissions of `words`, in words, the strongest first (they come in that order). */
export const asksOf = (words: string[], m: Manifest | null, lang: string) => words.map((w) => askOf(w, m, lang));

/** What a plugin adds, in words: a line per extension point it fills, with where to find it. */
export function contributionsOf(m: Manifest | null, lang: string): string[] {
  if (!m) return [];
  const out: string[] = [];
  const list = (key: string) => (Array.isArray(m.contributes[key]) ? (m.contributes[key] as Record<string, unknown>[]) : []);
  const names = (key: string) =>
    list(key)
      .map((i) => textOf((i.name ?? i.title ?? i.id) as ManifestText | undefined, lang))
      .filter(Boolean)
      .join(', ');
  const points: [
    string,
    (
      | 'themes'
      | 'harnesses'
      | 'webhooks'
      | 'commands'
      | 'ticketSources'
      | 'forges'
      | 'accounts'
      | 'menus'
      | 'statusBar'
      | 'badges'
      | 'projectTemplates'
      | 'settings'
      | 'secrets'
    ),
  ][] = [
    ['themes', 'themes'],
    ['harnesses', 'harnesses'],
    ['ticketSources', 'ticketSources'],
    ['forges', 'forges'],
    ['accounts', 'accounts'],
    ['webhooks', 'webhooks'],
    ['commands', 'commands'],
    ['menus', 'menus'],
    ['statusBar', 'statusBar'],
    ['badges', 'badges'],
    ['projectTemplates', 'projectTemplates'],
    ['settings', 'settings'],
    ['secrets', 'secrets'],
  ];
  for (const [key, label] of points) {
    const n = list(key).length;
    if (n) out.push(t(`plugins.adds.${label}`, { count: n, names: names(key) }));
  }
  return out;
}

/** The categories of the view's sidebar (choice 214), each with what puts a plugin in it. */
export const CATEGORIES = ['harnesses', 'kanban', 'forges', 'themes', 'notifications', 'permissions', 'tools', 'languages'] as const;
export type Category = (typeof CATEGORIES)[number];

/** The categories of a plugin, from what it adds and asks. */
export function categoriesOf(p: PluginView): Category[] {
  const m = p.manifest;
  if (!m) return [];
  const has = (k: string) => Array.isArray(m.contributes[k]) && (m.contributes[k] as unknown[]).length > 0;
  const out: Category[] = [];
  if (has('harnesses')) out.push('harnesses');
  if (has('ticketSources')) out.push('kanban');
  if (has('forges')) out.push('forges');
  if (has('themes')) out.push('themes');
  if (has('webhooks') || has('notifications')) out.push('notifications');
  if (m.permissions.decide || has('permissionRules')) out.push('permissions');
  if (has('commands') || has('menus') || has('statusBar') || has('badges')) out.push('tools');
  if (has('languages')) out.push('languages');
  return out;
}

/** Whether a plugin reaches `project`: where the user set it, as set; elsewhere unless it is private (§3.3). */
export const activeIn = (p: PluginView, project: Pick<Project, 'id' | 'private'>) =>
  p.choices.enabled && (p.choices.projects[project.id] ?? !project.private);

/** Where a plugin comes from, in a few words. */
export function originOf(o: Origin): string {
  switch (o.kind) {
    case 'git':
      return `${o.url.replace(/^https?:\/\//, '').replace(/\.git$/, '')} · ${o.tag}`;
    default:
      return o.path;
  }
}

/** The fingerprint shortened for a line: its kind and the ends of its hash. */
export function shortPrint(fp: string): string {
  const [kind, hash = ''] = fp.split(':');
  if (!hash) return fp;
  return kind === 'git' ? `${kind}:${hash.slice(0, 7)}` : `${kind}:${hash.slice(0, 4)}…${hash.slice(-4)}`;
}

/** The hue of a plugin's tile, drawn from its id: one of twelve, as the store's (`store/shared/tile.ts`). */
export function hueOf(id: string): number {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return (h % 12) * 30;
}

/** The two letters of a plugin's tile: its name's first two, as the store's. */
export const markOf = (name: string) => [...name.replace(/^[^\p{L}\p{N}]+/u, '')].slice(0, 2).join('') || '··';
