// « Les docs d'un projet d'équipe » (TS-9, `src-tauri/src/teamdocs.rs`): the spec, the decisions and the documents of
// a team's project on the team server, encrypted; the window reads this machine's copy, offline too. What is shown
// above the Docs: a page written on both sides that did not merge (« Garder la mienne », « Prendre la sienne », « Donner
// à l'agent rédacteur »), a decision's field that lost to a teammate's, the repository's files left after the import,
// the documents too heavy for the server, the numbers that wait. Pure functions here, the screens in `components`.

import { invoke } from '@tauri-apps/api/core';

/** A page written on both sides that did not merge three ways. */
export interface PageConflict {
  path: string;
  title: string;
  mine: string;
  theirs: string;
  base: string;
  agentId: string | null;
}

/** A field of a decision written here that lost to a teammate's at the same moment. */
export interface DecisionConflict {
  id: string;
  title: string;
  field: string;
  mine: unknown;
  theirs: unknown;
  stamp: string;
}

export interface TeamDocsView {
  canChoose: boolean;
  onServer: boolean;
  journal: string | null;
  online: boolean | null;
  pending: number;
  conflicts: PageConflict[];
  decisionConflicts: DecisionConflict[];
  repoFiles: string[];
  tooLarge: string[];
  waitingNumbers: string[];
  maxDocument: number;
}

export type Settle = 'mine' | 'theirs' | 'agent';

export const teamDocsApi = {
  status: (id: string) => invoke<TeamDocsView>('team_docs_status', { id }),
  choose: (id: string, server: boolean, journal: string | null = null) => invoke<TeamDocsView>('team_docs_choose', { id, server, journal }),
  settle: (id: string, path: string, choice: Settle) => invoke<TeamDocsView>('team_docs_settle', { id, path, choice }),
  dismiss: (id: string, stamp: string) => invoke<TeamDocsView>('team_docs_dismiss', { id, stamp }),
  removeRepoFiles: (id: string) => invoke<TeamDocsView>('team_docs_remove_repo_files', { id }),
};

/** What a band above the Docs says, in the order shown: the conflicts first, what waits last. */
export type Band =
  | { kind: 'conflict'; conflict: PageConflict }
  | { kind: 'lost'; lost: DecisionConflict }
  | { kind: 'repoFiles'; files: string[] }
  | { kind: 'tooLarge'; files: string[]; max: number }
  | { kind: 'waiting'; ids: string[] };

export function bands(view: TeamDocsView | null | undefined): Band[] {
  if (!view?.onServer) return [];
  const out: Band[] = [];
  for (const conflict of view.conflicts) out.push({ kind: 'conflict', conflict });
  for (const lost of view.decisionConflicts) out.push({ kind: 'lost', lost });
  if (view.repoFiles.length) out.push({ kind: 'repoFiles', files: view.repoFiles });
  if (view.tooLarge.length) out.push({ kind: 'tooLarge', files: view.tooLarge, max: view.maxDocument });
  if (view.waitingNumbers.length) out.push({ kind: 'waiting', ids: view.waitingNumbers });
  return out;
}

/** The state of the server as the line above the Docs says it: offline, changes waiting, or in step. */
export function syncState(view: TeamDocsView | null | undefined): 'offline' | 'pending' | 'synced' | null {
  if (!view?.onServer) return null;
  if (view.online === false) return 'offline';
  if (view.pending > 0) return 'pending';
  return 'synced';
}

/** A value of a decision's field as one line: a list joined, an object as JSON. */
export function fieldText(v: unknown): string {
  if (v === null || v === undefined) return '';
  if (typeof v === 'string') return v;
  if (Array.isArray(v)) return v.map(fieldText).join(', ');
  return JSON.stringify(v);
}

/** The repository's files as the band lists them: the first few, then how many more. */
export function someFiles(files: string[], shown = 3): { list: string[]; more: number } {
  return { list: files.slice(0, shown), more: Math.max(0, files.length - shown) };
}

/** A page's text without its header (`---` … `---`): what « Comparer » shows of each version. */
export function bodyOf(text: string): string {
  const m = /^---\r?\n[\s\S]*?\r?\n---\r?\n?/.exec(text);
  return (m ? text.slice(m[0].length) : text).replace(/^\s+/, '');
}

/** Megabytes, as the band says the bound. */
export function megabytes(bytes: number): number {
  return Math.round(bytes / (1024 * 1024));
}
