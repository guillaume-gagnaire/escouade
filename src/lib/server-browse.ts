/**
 * « Un dossier du serveur » and « Cloner un dépôt » (ESC-538, `docs/SERVER-UX.md` § 4 and 9): what the core of a
 * server says of one of its folders (`serverfolders::Listing`), and what the assistant « Nouveau projet » makes of it.
 */

import { fSize } from './format';
import { t } from './i18n';

/** A folder listed. */
export interface BrowseEntry {
  name: string;
  path: string;
  repo: boolean;
  branch?: string;
  project?: string;
}

/** A folder of the server. */
export interface BrowseListing {
  path: string;
  parent?: string;
  home: string;
  repo: boolean;
  branch?: string;
  project?: string;
  entries: BrowseEntry[];
  more: number;
  free?: number;
  total?: number;
}

/** What the line of a folder says on its right: a project already, a repository and its branch, a plain folder. */
export function entryNote(e: Pick<BrowseEntry, 'repo' | 'branch'>): string {
  if (!e.repo) return t('moves.browse.plain');
  return e.branch ? t('moves.browse.repo', { branch: e.branch }) : t('moves.browse.repoNoBranch');
}

/** « Le disque du serveur : 41 Go libres sur 80 Go », when the server says. */
export function diskText(l: Pick<BrowseListing, 'free' | 'total'>): string {
  if (l.free === undefined || l.total === undefined) return '';
  return t('moves.browse.disk', { free: fSize(l.free), total: fSize(l.total) });
}

/** The folder a clone of `url` goes in, as git names it (`serverfolders::folder_of`). */
export function cloneFolder(url: string): string {
  const last = url.trim().replace(/\/+$/, '').split(/[/:]/).pop() ?? '';
  return last.replace(/\.git$/, '');
}

/** The path of `name` in the folder `parent` of the server (a Linux server: `/`; a Windows one, its own separator). */
export function joinServer(parent: string, name: string): string {
  const sep = parent.includes('\\') && !parent.includes('/') ? '\\' : '/';
  return parent.endsWith(sep) ? `${parent}${name}` : `${parent}${sep}${name}`;
}
