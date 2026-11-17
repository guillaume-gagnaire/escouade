// The extensions of Open VSX in the VS Code view (ESC-481, A5), as the page sees them: everything comes from the core
// (`src-tauri/src/vsx.rs`), which searches, downloads, checks (sum, signature, archive) and serves; the page never
// reaches Open VSX nor the disk. Only declarative extensions run (themes, grammars, snippets, languages, icons): an
// extension with code waits for the extension host of its own origin (A6), one for Node for the Node host (N2).
// Pure: the panel (`ExtensionsPanel.svelte`) and the workbench (`workbench.ts::useExtensions`) read it.

import { t } from '../i18n';

/** Why the core refused (`escouade_vsx::Code`). */
export type RefusalCode =
  | 'badId'
  | 'notAllowed'
  | 'untrusted'
  | 'unsigned'
  | 'badSum'
  | 'badSignature'
  | 'trapped'
  | 'mismatch'
  | 'private'
  | 'notFound'
  | 'network'
  | 'io';

export interface Refusal {
  code: RefusalCode;
  /** Said by the core, in the interface's language. */
  message: string;
}

/** A result of the core, or its refusal. */
export interface Outcome<T> {
  value: T | null;
  refusal: Refusal | null;
}

/** A result of a search (a third party's data, as all of Open VSX's). */
export interface Summary {
  id: string;
  namespace: string;
  name: string;
  version: string;
  displayName: string;
  description: string;
  /** Open VSX verified the namespace: its owner is who publishes there. */
  verified: boolean;
  icon: string | null;
  downloads: number;
  deprecated: boolean;
}

export interface SearchPage {
  extensions: Summary[];
  total: number;
}

export interface Detail extends Summary {
  publisher: string;
  license: string;
  repository: string;
  homepage: string;
  preRelease: boolean;
  downloadable: boolean;
  timestamp: string;
  versions: string[];
  categories: string[];
}

/** A card: its README (rendered sanitized, nothing remote without a click), what the allowed list says, the version installed. */
export interface Card {
  detail: Detail;
  readme: string | null;
  refused: string | null;
  installed: string | null;
}

export type Kind = 'declarative' | 'web' | 'node';

export interface Contributes {
  /** The settings ids of its colour themes. */
  themes: string[];
  iconThemes: string[];
  languages: string[];
  grammars: number;
  snippets: number;
}

/** An installed extension, checked by the core, with what holds now. */
export interface ExtState {
  id: string;
  key: string;
  namespace: string;
  name: string;
  version: string;
  displayName: string;
  description: string;
  publisher: string;
  kind: Kind;
  enabled: boolean;
  verified: boolean;
  preRelease: boolean;
  sha256: string;
  signed: boolean;
  installedAt: number;
  files: number;
  bytes: number;
  contributes: Contributes;
  dir: string;
  /** The allowed list leaves it out now: not loaded. */
  refused: string | null;
  /** What its code waits for. */
  waits: string | null;
}

/** A layer of the allowed list, and who sets it. */
export interface Layer {
  by: string;
  imposed: boolean;
  rules: Record<string, unknown>;
}

export interface Installed {
  extensions: ExtState[];
  /** `<key>/<theme id>`: the theme of an extension chosen in the view; null: Escouade's. */
  theme: string | null;
  layers: Layer[];
  warnings: string[];
  /** A private project of the view: nothing is asked of Open VSX. */
  private: string | null;
}

export interface Update {
  id: string;
  from: string;
  to: string;
  refused: string | null;
}

export interface FileInfo {
  path: string;
  mime: string;
  size: number;
}

/** What the view says of an installed extension. */
export type Status = 'active' | 'off' | 'refused' | 'waitsHost' | 'needsNode';

export function status(e: ExtState): Status {
  if (e.refused) return 'refused';
  if (!e.enabled) return 'off';
  if (e.kind === 'web') return 'waitsHost';
  if (e.kind === 'node') return 'needsNode';
  return 'active';
}

/** Whether the view lays it: declarative, on, allowed. */
export const runs = (e: ExtState) => status(e) === 'active';

/** The label of a status. */
export function statusLabel(s: Status): string {
  const labels: Record<Status, () => string> = {
    active: () => t('editor.ext.status.active'),
    off: () => t('editor.ext.status.off'),
    refused: () => t('editor.ext.status.refused'),
    waitsHost: () => t('editor.ext.status.waitsHost'),
    needsNode: () => t('editor.ext.status.needsNode'),
  };
  return labels[s]();
}

/** What an extension brings, in a few words (« 2 thèmes, 1 grammaire »). */
export function brings(c: Contributes): string {
  const parts: string[] = [];
  if (c.themes.length) parts.push(t('editor.ext.brings.themes', { count: c.themes.length }));
  if (c.iconThemes.length) parts.push(t('editor.ext.brings.iconThemes', { count: c.iconThemes.length }));
  if (c.grammars) parts.push(t('editor.ext.brings.grammars', { count: c.grammars }));
  if (c.snippets) parts.push(t('editor.ext.brings.snippets', { count: c.snippets }));
  if (c.languages.length) parts.push(t('editor.ext.brings.languages', { count: c.languages.length }));
  return parts.join(', ');
}

/** `<key>/<theme id>`, as the core keeps the theme chosen. */
export const themeKey = (e: Pick<ExtState, 'key'>, theme: string) => `${e.key}/${theme}`;

/** The theme id of `theme` (`<key>/<id>`) when its extension runs; null: Escouade's theme. */
export function chosenTheme(list: ExtState[], theme: string | null): string | null {
  if (!theme) return null;
  const at = theme.indexOf('/');
  const key = theme.slice(0, at);
  const id = theme.slice(at + 1);
  const e = list.find((x) => x.key === key);
  return e && runs(e) && e.contributes.themes.includes(id) ? id : null;
}

/** How many downloads, short (« 1,2 k »). */
export function downloads(n: number, lang: string): string {
  return new Intl.NumberFormat(lang, { notation: 'compact', maximumFractionDigits: 1 }).format(n);
}

/** The files of an extension the view registers: what its contributions read, bounded (the rest is left). */
export function filesToLay(files: FileInfo[], max = 32 << 20): FileInfo[] {
  const useful = /\.(json|jsonc|tmlanguage|tmtheme|plist|xml|code-snippets|svg|png|jpe?g|gif|webp|woff2?|ttf|otf|css)$/i;
  const out: FileInfo[] = [];
  let total = 0;
  for (const f of files) {
    if (!useful.test(f.path) || f.path.includes('node_modules/')) continue;
    if (total + f.size > max) continue;
    total += f.size;
    out.push(f);
  }
  return out;
}
