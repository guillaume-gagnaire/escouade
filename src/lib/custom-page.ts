// « Page à soi » of a project (AP-7, ESC-677; docs/AGENT-PAGES.md § 7.4), as Réglages › Projets › Lancement shows it:
// the page this machine chose comes first; else the one `pages.custom` of its .escouade.yml proposes, once the user
// trusted the file (with its commands); else none, and `show_page(template: "custom")` is refused, saying what to take.

import type { Project } from './types';

/** The key of the file. */
export const CUSTOM_PAGE_KEY = 'pages.custom';

/** Where the page of one's own of a project comes from. */
export type CustomPageState =
  /** None: `custom` is refused. */
  | { from: 'none' }
  /** This machine's; `file`, what the repository proposes and it comes before. */
  | { from: 'machine'; path: string; file: string | null }
  /** The repository's, trusted. */
  | { from: 'repo'; path: string }
  /** The repository's, waiting for the user's trust. */
  | { from: 'waiting'; path: string };

const text = (v: unknown): string => (typeof v === 'string' ? v.trim() : '');

/** Where the page of one's own of `p` comes from, as the core picks it (`pages::custom_page_of`). */
export function customPageOf(p: Project | undefined | null, mine = p?.customPage): CustomPageState {
  const file = p?.shared?.file;
  const proposed = file && !file.error ? text(file.values?.[CUSTOM_PAGE_KEY]) : '';
  const trusted = text(p?.fileCustomPage);
  const waiting = text(file?.pending?.commands?.[CUSTOM_PAGE_KEY]);
  const own = text(mine);
  if (own) return { from: 'machine', path: own, file: proposed || trusted || null };
  if (trusted) return { from: 'repo', path: trusted };
  if (waiting) return { from: 'waiting', path: waiting };
  return { from: 'none' };
}

/** Why `path` is no page of one's own (an .html document), or null; empty is none, which is fine. */
export function customPageProblem(path: string): 'notHtml' | null {
  const p = path.trim();
  if (!p) return null;
  return /\.html?$/i.test(p) ? null : 'notHtml';
}

/** The file a dialog chose, relative to the project's folder when it is in it (as the core reads it then), else as is. */
export function relativeToProject(file: string, folder: string): string {
  const norm = (s: string) => s.replace(/\\/g, '/').replace(/\/+$/, '');
  const f = norm(file);
  const d = norm(folder);
  const windows = /^[a-z]:\//i.test(d);
  const same = windows ? f.toLowerCase().startsWith(`${d.toLowerCase()}/`) : f.startsWith(`${d}/`);
  return d && same ? f.slice(d.length + 1) : file;
}
