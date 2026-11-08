// The search of the settings window (R8 of the 2.2 redesign, design/settings-2-2/PROPOSITION.md §2 « La recherche »):
// in the labels and the descriptions of each row of the register (settings-pages.ts), Escouade's and each project's,
// in French and in English whatever the language of the interface, case and accents left out (as the search in the
// conversations). A hit names its page; opening it unfolds « Avancé » when the row is there, and lights the row.

import { normalize } from './conversation';
import { tIn, type Lang } from './i18n';
import { pageInfo, SETTINGS_PAGES, SETTINGS_ROWS, type RowInfo, type SettingsPage } from './settings-pages';

const LANGS: Lang[] = ['fr', 'en'];

/** A result: a row, or a page itself (its name or what it holds matches). */
export interface SearchHit {
  /** Unique among the hits: what a list keys on. */
  key: string;
  page: SettingsPage;
  /** The project whose page it is, for a page of a project. */
  projectId: string | null;
  /** The row found; none for a page. */
  row: RowInfo | null;
}

interface Doc {
  page: SettingsPage;
  row: RowInfo | null;
  labels: string[];
  descs: string[];
}

/** Every row, and every page, with its texts in both languages folded. Read at each search: the catalogs are loaded. */
function docs(): Doc[] {
  const pages: Doc[] = SETTINGS_PAGES.map((p) => ({
    page: p.id,
    row: null,
    labels: LANGS.map((l) => normalize(tIn(l, `settings.pages.${p.id}.label`))),
    descs: LANGS.map((l) => normalize(tIn(l, `settings.pages.${p.id}.desc`))),
  }));
  const rows: Doc[] = SETTINGS_ROWS.map((r) => ({
    page: r.page,
    row: r,
    labels: LANGS.map((l) => normalize(r.labelIn(l))),
    descs: LANGS.map((l) => normalize(r.descIn(l))),
  }));
  return [...pages, ...rows];
}

/**
 * How well a text answers the query, the lower the better; null when it does not. A label that starts with the query,
 * then one that holds it, then one that holds each of its words, then a description that holds each of them.
 */
function score(d: Doc, q: string, words: string[]): number | null {
  const has = (text: string) => words.every((w) => text.includes(w));
  if (d.labels.some((l) => l.startsWith(q))) return 0;
  if (d.labels.some((l) => l.includes(q))) return 1;
  if (d.labels.some(has)) return 2;
  if (d.descs.some(has)) return 3;
  return null;
}

/**
 * The rows and pages that answer `query`, the best first, then in the order of the register; a page of a project
 * once for each project, the one chosen (`current`) first. At most `limit`.
 */
export function searchSettings(query: string, projects: { id: string }[], current: string | null = null, limit = 40): SearchHit[] {
  const q = normalize(query);
  if (!q) return [];
  const words = q.split(' ');
  const order = [...projects].sort((a, b) => Number(b.id === current) - Number(a.id === current));
  const found: (SearchHit & { score: number; at: number })[] = [];
  docs().forEach((d, at) => {
    const s = score(d, q, words);
    if (s === null) return;
    const id = d.row ? `row:${d.row.id}` : `page:${d.page}`;
    if (pageInfo(d.page).scope === 'app') found.push({ key: id, page: d.page, projectId: null, row: d.row, score: s, at });
    else
      order.forEach((p, i) =>
        found.push({ key: `${p.id}:${id}`, page: d.page, projectId: p.id, row: d.row, score: s, at: at + (i + 1) * 10_000 }),
      );
  });
  return found
    .sort((a, b) => a.score - b.score || a.at - b.at)
    .slice(0, limit)
    .map(({ key, page, projectId, row }) => ({ key, page, projectId, row }));
}

/**
 * The element of the page on screen that shows the row: the setting whose label is the row's (`.sr .l`), else the
 * group or « Avancé » that bears its name (a row that is a whole group: « Hébergeurs de code », « Recherche par le
 * sens »). Null when nothing on the page bears it.
 */
export function rowElement(panel: ParentNode, row: RowInfo): HTMLElement | null {
  const label = normalize(row.label);
  // Its label alone, else its label and the hint after it (« Proxy HTTP(S) ex. http://… »).
  const named = (sel: string) => {
    const all = [...panel.querySelectorAll<HTMLElement>(sel)].map((el) => ({ el, text: normalize(el.textContent ?? '') }));
    return (all.find((x) => x.text === label) ?? all.find((x) => x.text.startsWith(`${label} `)))?.el ?? null;
  };
  const setting = named('.sr .l');
  if (setting) return setting.closest<HTMLElement>('.sr');
  const group = named('.sg h2, .sg .ft, .sg .l');
  return group?.closest<HTMLElement>('.sg') ?? null;
}
