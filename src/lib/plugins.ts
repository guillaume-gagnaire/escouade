// The plugins (PLG-4, ESC-176; docs/PLUGINS-DESIGN.md §3): their views as the core gives them, the calls of the view Plugins,
// and what the view reads of them — what each may do in words, the strongest first, its category, its projects.
import { invoke } from '@tauri-apps/api/core';
import { t } from './i18n';
import type { StoreView } from './store';
import type { Project } from './types';

export type Origin =
  | { kind: 'folder'; path: string }
  | { kind: 'git'; url: string; tag: string }
  | { kind: 'dev'; path: string }
  /** A version of store.escouade.dev (STORE-9), pinned by its version.json's SHA-256. */
  | {
      kind: 'store';
      version: string;
      sha256: string;
      verified: boolean;
      repository: string;
      /** Its folder in the repository (ESC-313), absent at its root. */
      path?: string;
      tag: string;
      commit: string;
      /** Installed from the app's seed, with no network (ESC-338): the same bytes as the store's, a plugin like any other. */
      seed?: boolean;
    }
  /**
   * A module of the repository's workspace (`scripts/plugins-wasm.sh`): what a build of development runs of the first hand
   * where the seed has not its files; never in a release (ESC-355, the origin `builtin` gone). Not the same as a
   * `dev.escouade.*` of the store: the origin tells them apart, never the id.
   */
  | { kind: 'workspace' };

/** Whether this plugin is a module of the workspace of a build of development, by its origin alone. */
export const ofTheWorkspace = (p: Pick<PluginView, 'origin'>) => p.origin.kind === 'workspace';

/** The page of a plugin's source on GitHub: its repository, or its folder at its tag (`<repository>/tree/<tag>/<folder>`). */
export function sourceUrl(o: { repository: string; tag: string; path?: string }): string {
  return o.path && o.tag ? `${o.repository.replace(/\/$/, '')}/tree/${o.tag}/${o.path}` : o.repository;
}

export type PluginState =
  | { kind: 'ready' }
  | { kind: 'disabled'; reason: string }
  | { kind: 'invalid'; reason: string; problems: { path: string; code: string; message: string }[] };

/** A text of a manifest: one for every language, or one by language (`{ fr, en }`). */
export type ManifestText = string | Record<string, string>;

export interface ManifestSetting {
  key: string;
  /** `profile`: a profile of the kind `of` (ESC-530). */
  type: 'string' | 'number' | 'boolean' | 'select' | 'profile';
  /** `project`: set by each project, in its settings, not on the plugin's page (ESC-530). */
  scope?: 'app' | 'project';
  /** `false`: never from `.escouade.yml`. */
  repo?: boolean;
  /** The kind of profile of a `profile` setting. */
  of?: string;
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
  /** A command (a process), or a JS/TS bundle that Escouade runs in WASM (`js`, ESC-320). */
  run?: { command: string; js?: string; wasm?: string; args: string[] };
  activation: string[];
  permissions: {
    events: string[];
    network: string[];
    secrets: string[];
    commands: string[];
    /** The variables it may set on what it runs (ESC-424). */
    env?: string[];
    files?: { root: string; access: string }[];
    decide: boolean;
  };
  /** What it asks for beyond the host's defaults; `processes` and `reason`: its programs at once (ESC-529). */
  limits?: { processes?: number; reason?: ManifestText };
  contributes: Record<string, unknown>;
}

export interface Choices {
  enabled: boolean;
  projects: Record<string, boolean>;
  revoked: string[];
  settings: Record<string, unknown>;
  /** The fingerprint of the commands on facts the user agreed to (PLG-5); absent: none. */
  commandsApproved?: string;
  /** Its settings of each project set on this machine, by project then key (ESC-530); a profile's id, or null. */
  projectSettings?: Record<string, Record<string, unknown>>;
  /** Its profiles by kind (ESC-530): their ids and simple fields. */
  profiles?: Record<string, { id: string; values: Record<string, unknown> }[]>;
  /** The names of the secrets kept for it (never their values). */
  secretNames?: string[];
}

export interface PluginUpdate {
  version: string;
  tag?: string;
  /** For a plugin of the store: whether that version is verified. */
  verified?: boolean;
  /** That version is the one the app's seed holds: it installs from there, with no network. */
  seed?: boolean;
}

/** What the revocation list says of the version a plugin runs (docs/STORE.md §4.6). */
export interface Withdrawn {
  action: 'disable' | 'unverify';
  version: string;
  reason: string;
  date: string;
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
  /** « Vérifié par Escouade »: a version of the store whose signature holds here, now. */
  verified: boolean;
  /** The key that made that seal, and the role it sealed in (ESC-592). */
  seal: SealKey | null;
  withdrawn: Withdrawn | null;
  /** Not from the store, but the store has its id: the version it would install in its place. */
  inStore: { version: string; verified: boolean } | null;
}

/** What a key of the store signs (crates/escouade-plugins/src/store_keys.rs, `Role`). */
export type KeyRole = 'first-hand' | 'third-party' | 'revocation' | 'backup';

/** The i18n key of a role's name (`plugins.seal.role.*`, in camelCase). */
export const roleKey = (role: KeyRole) =>
  (({ 'first-hand': 'firstHand', 'third-party': 'thirdParty', revocation: 'revocation', backup: 'backup' }) as const)[role];

/** The key that made a seal: its minisign id and its role. */
export interface SealKey {
  id: string;
  role: KeyRole;
}

/** A plugin of the app's seed that is not installed (ESC-338): uninstalled by the user, or refused by the seed. */
export interface SeedOffer {
  id: string;
  name: string;
  version: string;
  /** The version the user uninstalled. */
  removed: string | null;
  /** Why the seed did not install it. */
  refused: string | null;
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
  /** A version of the store whose signature holds with the app's keys. */
  verified: boolean;
  /** Why the revocation list took its seal off. */
  unverified: string | null;
  /**
   * A folder or a repository read under « N'installer que des versions vérifiées » (SEC-01, ESC-589): the store's verified
   * version whose files are its own.
   */
  matchesStore?: string;
  /**
   * The module a `run.wasm` plugin of a folder will run, by its path in the folder: `packs/<id>-<version>.wasm` or
   * `plugin.wasm` (issue #43, ESC-652). The fingerprint covers it.
   */
  module?: string;
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
    stop?: { kind: 'asked' | 'shutdown' | 'crashed' | 'refused'; times?: number; reason?: string };
    restarts: number;
    pid?: number;
  };
  journal: JournalEntry[];
}

export type JournalKind =
  'start' | 'ready' | 'stop' | 'crash' | 'stderr' | 'message' | 'timeout' | 'refused' | 'dropped' | 'webhook' | 'command' | 'decision';

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
  /** Downloads a version of the store and checks it (the last one this Escouade runs when none is named). */
  inspectStore: (id: string, version: string | null = null) => invoke<PluginPreview>('plugin_inspect_store', { id, version }),
  /** Reads the seed's version of a plugin and checks it, with no network (ESC-338). */
  inspectSeed: (id: string) => invoke<PluginPreview>('plugin_inspect_seed', { id }),
  seedOffers: () => invoke<SeedOffer[]>('plugin_seed_offers'),
  /** « Installer » of a plugin the seed proposes: installed at once from it, with no network (ESC-373). */
  installSeed: (id: string) => invoke<PluginView | null>('plugin_install_seed', { id }),
  install: (token: string) => invoke<PluginView>('plugin_install', { token }),
  discard: (token: string) => invoke<void>('plugin_discard', { token }),
  remove: (id: string, keep: boolean) => invoke<boolean>('plugin_remove', { id, keep }),
  setEnabled: (id: string, on: boolean) => invoke<PluginView>('plugin_set_enabled', { id, on }),
  setProject: (id: string, project: string, on: boolean) => invoke<PluginView>('plugin_set_project', { id, project, on }),
  setPermission: (id: string, word: string, granted: boolean) => invoke<PluginView>('plugin_set_permission', { id, word, granted }),
  setSetting: (id: string, key: string, value: unknown) => invoke<PluginView>('plugin_set_setting', { id, key, value }),
  /** Agrees to the commands it runs on facts as they are now, or takes the agreement back (PLG-5). */
  approveCommands: (id: string, on: boolean) => invoke<PluginView>('plugin_approve_commands', { id, on }),
  secrets: (id: string) => invoke<SecretState[]>('plugin_secrets', { id }),
  /** « Garder » (empty: « Oublier »); `inFile`: the user chose the file, where the keychain refused (ESC-499). Rejects with a
   * `SecretRefused`. */
  setSecret: (id: string, key: string, value: string, inFile = false) => invoke<void>('plugin_set_secret', { id, key, value, inFile }),
  /** « Déplacer vers le trousseau »: a secret kept in the file goes to the keychain that is there now. */
  moveSecret: (id: string, key: string) => invoke<void>('plugin_secret_move', { id, key }),
  /** The file of the plugins' secrets on this machine (Réglages › Plugins). */
  secretsFile: () => invoke<SecretsFile>('secrets_file'),
  removeSecretsFile: () => invoke<void>('secrets_file_remove'),
  run: (id: string) => invoke<PluginRun>('plugin_run', { id }),
  start: (id: string) => invoke<PluginRun>('plugin_start', { id }),
  /** « Rechercher des mises à jour »: the store read again first (ESC-405), the plugins then, and the store as it was read. */
  checkUpdates: () => invoke<{ plugins: PluginView[]; store: StoreView }>('plugins_check_updates'),
  /** The icon of an installed plugin (PLG-13), a `data:image/png` address; null: its letters. */
  icon: (id: string) => invoke<string | null>('plugin_icon', { id }),
};

/** A plugin's secret, as its settings show it: kept or not, and in the file the user chose rather than the keychain. */
export interface SecretState {
  key: string;
  kept: boolean;
  inFile: boolean;
  /** A secret on several lines, kept: its lines and bytes, never its text (ESC-526). */
  size?: { lines: number; bytes: number };
}

/** Why « Garder » kept nothing: its cause, said simply; `keychainRefused`: the keychain was the cause, the file is offered. */
export interface SecretRefused {
  message: string;
  keychainRefused: boolean;
}

/** The file of the plugins' secrets on this machine (ESC-499): chosen or not, where, whether a keychain is there, what it holds. */
export interface SecretsFile {
  chosen: boolean;
  path: string;
  keychain: boolean;
  secrets: { plugin: string; pluginName: string; key: string }[];
}

/** What a rejected `setSecret` says: a `SecretRefused`, or a plain text. */
export function refusedOf(e: unknown): SecretRefused {
  if (e && typeof e === 'object' && 'message' in e) {
    const r = e as Partial<SecretRefused>;
    return { message: String(r.message), keychainRefused: !!r.keychainRefused };
  }
  return { message: String(e), keychainRefused: false };
}

/** A text of a manifest in the interface's language: English by default (§3.1). */
export function textOf(text: ManifestText | undefined, lang: string): string {
  if (text === undefined) return '';
  if (typeof text === 'string') return text;
  return text[lang] ?? text.en ?? Object.values(text)[0] ?? '';
}

/** A permission in words: its icon, what it says, its detail (a host, an event…), and how strong it is. */
export interface Ask {
  word: string;
  icon: 'term' | 'shield' | 'bash' | 'spark' | 'globe' | 'lock' | 'folder' | 'overview';
  text: string;
  detail: string;
  strong: boolean;
  red: boolean;
  /** What writes on the user's disk, in orange (§8.4.6). */
  orange?: boolean;
  /** What it says under it: whether Escouade enforces it (a host of a plugin in WASM) or only shows it. */
  note?: string;
  /** A command with jokers (ESC-528): its words, each joker said in words, in a pill. */
  pieces?: CommandPiece[];
}

/** A word of a command as the window shows it: as written, or a joker (`joker`: `{pid}`…) said in words. */
export interface CommandPiece {
  text: string;
  joker?: string;
}

/** The jokers of `permissions.commands` (§5.3 of the contract's study), by what the window says of each. */
const JOKERS = {
  '{int}': 'plugins.ask.jokers.int',
  '{port}': 'plugins.ask.jokers.port',
  '{name}': 'plugins.ask.jokers.name',
  '{pid}': 'plugins.ask.jokers.pid',
  '{path:temp}': 'plugins.ask.jokers.temp',
  '{path:plugin}': 'plugins.ask.jokers.plugin',
} as const;

/** The white space a command is cut on, as Rust's `split_whitespace` (never `\s`, which differs on U+FEFF and U+0085). */
const SPACE = /[\t\n\v\f\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+/;

/** The words of `command`, each joker said in words; null when it has none (the command shows as written). */
export function piecesOf(command: string): CommandPiece[] | null {
  const words = command.split(SPACE).filter(Boolean);
  if (!words.some((w) => w in JOKERS)) return null;
  return words.map((w) => (w in JOKERS ? { text: t(JOKERS[w as keyof typeof JOKERS]), joker: w } : { text: w }));
}

/** The title of a secret's field, or its key. */
function secretTitle(m: Manifest | null, key: string, lang: string) {
  const fields = (m?.contributes.secrets as { key: string; title?: ManifestText }[] | undefined) ?? [];
  return textOf(fields.find((f) => f.key === key)?.title, lang) || key;
}

/** A folder it asks for, `files:<root>:<access>` (§8.4.6): writing in the user's folders is in orange. */
function askOfFiles(word: string, root: string, writes: boolean): Ask {
  const note = t('plugins.perms.filesApplied');
  const line = (text: string, orange = false): Ask => ({
    word,
    icon: 'folder',
    text,
    detail: root,
    strong: orange,
    red: false,
    orange,
    note,
  });
  if (root === 'plugin') return { ...line(t('plugins.ask.filesPlugin')), note: undefined };
  if (root === 'temp') return { ...line(t('plugins.ask.filesTemp')), note: undefined };
  if (writes) return line(t('plugins.ask.filesWrite'), true);
  return line(t(root === 'agentHome' ? 'plugins.ask.filesHome' : 'plugins.ask.filesRead'));
}

/** What the permission `word` lets the plugin do, in words (§3.4). */
export function askOf(word: string, m: Manifest | null, lang: string): Ask {
  const [kind, ...rest] = word.split(':');
  const what = rest.join(':');
  switch (kind) {
    case 'run':
      // A JS/TS bundle or a Rust module runs inside Escouade, isolated (ESC-320, ESC-331): no program of the user's machine.
      if (m?.run?.js || m?.run?.wasm)
        return { word, icon: 'shield', text: t('plugins.ask.wasm'), detail: m.run.js ?? m.run.wasm ?? '', strong: false, red: false };
      return { word, icon: 'term', text: t('plugins.ask.run'), detail: m?.run?.command ?? '', strong: true, red: false };
    case 'decide':
      return { word, icon: 'shield', text: t('plugins.ask.decide'), detail: '', strong: true, red: true };
    case 'harness':
      // A harness it describes (PLG-6): the program of the user's machine its agents run, with its arguments.
      return { word, icon: 'term', text: t('plugins.ask.harness'), detail: what, strong: true, red: false };
    case 'harnessInstall':
    case 'harnessUpdate':
      // What its harness's manifest says installs or updates it (SEC-30): the app shows it, said to come from the plugin.
      return {
        word,
        icon: 'term',
        text: t(kind === 'harnessInstall' ? 'plugins.ask.harnessInstall' : 'plugins.ask.harnessUpdate'),
        detail: what,
        strong: false,
        red: false,
        note: t('plugins.perms.harnessCommandNote'),
      };
    case 'harnessUnset':
      // A variable its harness's launches take off: never the proxy nor the authorities Escouade gives (SEC-30).
      return {
        word,
        icon: 'bash',
        text: t('plugins.ask.harnessUnset'),
        detail: what,
        strong: false,
        red: false,
        note: t('plugins.perms.harnessUnsetApplied'),
      };
    case 'harnessBypass':
    case 'harnessNoMode':
      // A harness whose agents never ask before they act, or that says nothing of it (ESC-489): held for Bypass.
      return {
        word,
        icon: 'shield',
        text: t(kind === 'harnessBypass' ? 'plugins.ask.harnessBypass' : 'plugins.ask.harnessNoMode'),
        detail: what,
        strong: true,
        red: true,
        note: t('plugins.perms.harnessBypassApplied'),
      };
    case 'commands':
      return {
        word,
        icon: 'bash',
        text: t('plugins.ask.commands'),
        detail: what,
        pieces: piecesOf(what) ?? undefined,
        strong: true,
        red: true,
        // A program of a plugin in WASM goes through Escouade (`proc/spawn`); a process runs what it likes.
        note: t(m?.run?.js || m?.run?.wasm ? 'plugins.perms.commandsApplied' : 'plugins.perms.commandsNote'),
      };
    case 'processes': {
      // More programs at once than the 4 it has without asking (`limits.processes`, ESC-529): the core says it only
      // beyond 4, with the author's reason.
      // Its last full stop goes: the sentence that quotes it ends with its own.
      const reason = textOf(m?.limits?.reason, lang)
        .trim()
        .replace(/[.。]+$/, '');
      return {
        word,
        icon: 'overview',
        text: t('plugins.ask.processes', { count: Number(what) }),
        detail: '',
        strong: true,
        red: false,
        note: reason ? t('plugins.ask.processesWhy', { reason }) : t('plugins.ask.processesUnasked'),
      };
    }
    case 'env':
      // A variable it sets on what it runs (ESC-424): any other is refused, and the families that would change the program too.
      return {
        word,
        icon: 'bash',
        text: t('plugins.ask.env'),
        detail: what,
        strong: false,
        red: false,
        note: t('plugins.perms.envApplied'),
      };
    case 'files':
      return askOfFiles(word, rest[0] ?? '', rest[1] === 'write');
    case 'events':
      return { word, icon: 'spark', text: t('plugins.ask.events'), detail: what, strong: false, red: false };
    case 'network':
      // The hosts the user types for its accounts, and the local server of its harness: no host to name (ESC-332).
      if (what === '{account}' || what === 'local:shared')
        return {
          word,
          icon: 'globe',
          text: t(what === '{account}' ? 'plugins.ask.networkAccount' : 'plugins.ask.networkShared'),
          detail: '',
          strong: false,
          red: false,
        };
      return {
        word,
        icon: 'globe',
        text: t('plugins.ask.network'),
        detail: what,
        strong: false,
        red: false,
        // A bundle goes through Escouade's `net/fetch`, a declarative plugin's webhooks are sent by Escouade: only a
        // command keeps a network of its own.
        note: t(
          m?.run?.js || m?.run?.wasm
            ? 'plugins.perms.networkApplied'
            : m && !m.run
              ? 'plugins.perms.networkWebhooks'
              : 'plugins.perms.networkNote',
        ),
      };
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

/**
 * What a plugin adds, in words: a line per extension point it fills, with where to find it. The points the manifest
 * reserves (`menus`, `statusBar`, `badges`, `projectTemplates`) do nothing yet: never said (ESC-524).
 */
export function contributionsOf(m: Manifest | null, lang: string): string[] {
  if (!m) return [];
  const out: string[] = [];
  const list = (key: string) => (Array.isArray(m.contributes[key]) ? (m.contributes[key] as Record<string, unknown>[]) : []);
  // A webhook has no name of its own: what sets it off says it.
  const nameOf = (i: Record<string, unknown>) =>
    Array.isArray(i.events) && !i.name && !i.title && !i.id
      ? i.events.join(' ')
      : textOf((i.name ?? i.title ?? i.id) as ManifestText | undefined, lang);
  const names = (key: string) => list(key).map(nameOf).filter(Boolean).join(', ');
  const points: [
    string,
    (
      | 'themes'
      | 'harnesses'
      | 'webhooks'
      | 'commands'
      | 'ticketSources'
      | 'forges'
      | 'launchEnvironments'
      | 'accounts'
      | 'settings'
      | 'secrets'
    ),
  ][] = [
    ['themes', 'themes'],
    ['harnesses', 'harnesses'],
    ['ticketSources', 'ticketSources'],
    ['forges', 'forges'],
    ['launchEnvironments', 'launchEnvironments'],
    ['accounts', 'accounts'],
    ['webhooks', 'webhooks'],
    ['commands', 'commands'],
    ['settings', 'settings'],
    ['secrets', 'secrets'],
  ];
  for (const [key, label] of points) {
    const n = list(key).length;
    // An account of a Kanban source or a code host is added in Services connectés; a harness's, in Comptes et harness.
    const where = label === 'accounts' && (list('ticketSources').length || list('forges').length) ? 'accountsServices' : label;
    if (n) out.push(t(`plugins.adds.${where}`, { count: n, names: names(key) }));
  }
  return out;
}

/** The categories of the view's sidebar (choice 214), each with what puts a plugin in it. */
export const CATEGORIES = ['harnesses', 'kanban', 'forges', 'themes', 'notifications', 'permissions', 'tools', 'languages'] as const;
export type Category = (typeof CATEGORIES)[number];

/** The categories of a plugin, from what it adds and asks; never from a reserved point, which does nothing (ESC-524). */
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
  if (has('commands') || has('launchEnvironments')) out.push('tools');
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
    case 'store':
      return 'store.escouade.dev';
    case 'workspace':
      return 'plugins-wasm';
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
