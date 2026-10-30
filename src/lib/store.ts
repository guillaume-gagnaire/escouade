// The store in the app (STORE-9, docs/STORE.md §5): the catalogue of store.escouade.dev as the core keeps it, searched on
// this machine (name, description, keywords, author; case and accents ignored), its categories, what a version asks for
// in one line, and the calls of the view Plugins.
import { invoke } from '@tauri-apps/api/core';
import { t } from './i18n';
import { textOf, type Category, type ManifestText } from './plugins';

export interface CatalogVersion {
  version: string;
  published_at: string;
  sha256: string;
  archive: { sha256: string; size: number };
  platforms: string[];
  escouade: { api: string | number; min: string; max?: string };
  verified: boolean;
  review?: { signed_at: string; summary: { fr: string; en: string } | null };
  yanked: boolean;
  unverified?: { fr: string; en: string };
  permissions: { events?: string[]; network?: string[]; secrets?: string[]; commands?: string[]; decide?: boolean };
  run: boolean;
  contributes: string[];
  files: { version_json: string; archive: string; binaries: Record<string, string>; icon?: Record<string, string>; signature?: string };
}

export interface CatalogPlugin {
  id: string;
  name: ManifestText;
  description: ManifestText;
  author: string | null;
  repository: string;
  license: string | null;
  categories: string[];
  keywords: string[];
  installs: number;
  latest: string | null;
  transferred_at?: string;
  versions: CatalogVersion[];
}

export interface Catalog {
  format: number;
  updated_at: string;
  installs_until: string;
  plugins: CatalogPlugin[];
}

export interface StoreView {
  url: string;
  catalog: Catalog | null;
  readAt: string | null;
  error: string | null;
  /** By id, the version this Escouade would install: the last it can run. */
  best: Record<string, string>;
  revocations: { serial: number | null; readAt: string | null; from: string | null; error: string | null } | null;
  platform: string;
}

export interface Suggestion {
  id: string;
  reason: string;
}

export const storeApi = {
  view: () => invoke<StoreView>('store_view'),
  refresh: () => invoke<StoreView>('store_refresh'),
  suggestions: (project: string | null) => invoke<Suggestion[]>('store_suggestions', { project }),
};

/** The store's categories (store/server/utils/submissions.ts) as the view's (choice 214). */
const CATEGORY_OF: Record<string, Category> = {
  harness: 'harnesses',
  'ticket-sources': 'kanban',
  forges: 'forges',
  themes: 'themes',
  notifications: 'notifications',
  permissions: 'permissions',
  tools: 'tools',
  languages: 'languages',
};

export const categoriesOfCatalog = (p: CatalogPlugin): Category[] =>
  p.categories.map((c) => CATEGORY_OF[c]).filter((c): c is Category => !!c);

/** A text folded for the search: lower case, without accents. */
export const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

/** Whether the plugin `p` answers the search `q`: every word in its name, description, keywords, author or id. */
export function matches(p: CatalogPlugin, q: string, lang: string): boolean {
  const words = fold(q).split(/\s+/).filter(Boolean);
  if (!words.length) return true;
  const hay = fold(
    [textOf(p.name, lang), textOf(p.description, 'fr'), textOf(p.description, 'en'), p.keywords.join(' '), p.author ?? '', p.id].join(' '),
  );
  return words.every((w) => hay.includes(w));
}

/** The version of `p` this Escouade would install, if any. */
export const bestOf = (view: StoreView, p: CatalogPlugin): CatalogVersion | null =>
  p.versions.find((v) => v.version === view.best[p.id]) ?? null;

/** Whether the seal of `v` holds as the catalogue says (the app checks it again at the install). */
export const sealed = (v: CatalogVersion | null) => !!v && v.verified && !v.unverified;

/** What a version asks for, in one line, the strongest first (choice 166). */
export interface Asked {
  text: string;
  kind: 'run' | 'decide' | 'host' | 'secret' | 'none';
}
export function askedOf(v: CatalogVersion | null): Asked[] {
  if (!v) return [];
  const out: Asked[] = [];
  const p = v.permissions ?? {};
  if (v.run) out.push({ text: t('plugins.store.runs'), kind: 'run' });
  if (p.decide) out.push({ text: t('plugins.store.decides'), kind: 'decide' });
  for (const h of p.network ?? []) out.push({ text: h, kind: 'host' });
  const secrets = (p.secrets ?? []).length;
  if (secrets) out.push({ text: t('plugins.store.secrets', { count: secrets }), kind: 'secret' });
  if (!v.run && !out.length) out.push({ text: t('plugins.store.noProgram'), kind: 'none' });
  return out;
}

/** A number as the interface's language writes it (2 310). */
export const count = (n: number, lang: string) => new Intl.NumberFormat(lang === 'fr' ? 'fr-FR' : 'en-US').format(n);
