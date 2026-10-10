// « Ouvrir un fichier » (Ctrl+P): the line typed after a name, the files ranked for what is typed, the files opened
// last, and the editor's answer to a file picked.

import type { NavTarget } from './goto';

/** Files the palette shows. */
export const QUICK_OPEN_MAX = 50;
/** Files remembered per source. */
const RECENT_MAX = 30;

export interface QuickQuery {
  /** What is looked for in the paths. */
  name: string;
  /** `name:42` or `name:42:7`: the line, and the column in characters, to open the file at. */
  line?: number;
  col?: number;
}

/**
 * The name typed and the line after it, as an error of a stack trace gives them (`src/app.ts:42:7`). The colon of a
 * Windows path (`C:\code\app.ts:3`) belongs to the name; a colon not followed by digits yet is waited for.
 */
export function parseQuickQuery(text: string): QuickQuery {
  const typed = text.trim();
  const m = /^(.+?):(\d*)(?::(\d*))?$/.exec(typed);
  if (!m) return { name: typed };
  const line = Number(m[2]);
  if (!line) return { name: m[1] };
  const col = Number(m[3]);
  return col ? { name: m[1], line, col } : { name: m[1], line };
}

const WORD_START = new Set(['/', '-', '_', '.', ' ']);
const isWordStart = (lower: string, at: number) => at === 0 || WORD_START.has(lower[at - 1]);

/**
 * How well `q` (lowercase) names `path` (`lower` is its lowercase), or null when it does not. In decreasing order: the
 * name starts with it (all of it: the most), the name holds it, the path holds it, its letters are in the name in
 * order, in the path in order. Where it starts a word (after a slash, a dash, a dot…) counts for more; the shorter
 * path comes first among equals.
 */
function score(lower: string, q: string): number | null {
  const nameAt = lower.lastIndexOf('/') + 1;
  const name = lower.slice(nameAt);
  let s: number;
  if (name.startsWith(q)) {
    s = name === q ? 1200 : 1000;
  } else if (name.includes(q)) {
    s = 700 + (isWordStart(lower, nameAt + name.indexOf(q)) ? 50 : 0);
  } else if (lower.includes(q)) {
    s = 400 + (isWordStart(lower, lower.indexOf(q)) ? 50 : 0);
  } else {
    const inName = letters(lower, q, nameAt);
    const found = inName ?? letters(lower, q, 0);
    if (found === null) return null;
    s = (inName === null ? 100 : 250) + found;
  }
  return s - lower.length;
}

/** The letters of `q` found in order in `lower` from `from` on: the bonus of those starting a word, null when one is missing. */
function letters(lower: string, q: string, from: number): number | null {
  let at = from;
  let bonus = 0;
  for (const c of q) {
    at = lower.indexOf(c, at);
    if (at < 0) return null;
    // The start of a name or of a folder counts for more than a word inside a name.
    if (isWordStart(lower, at)) bonus += at === 0 || lower[at - 1] === '/' ? 20 : 10;
    at += c.length;
  }
  return bonus;
}

/**
 * The `limit` files of `files` best named by `query`, best first. With nothing typed, the `recent` files (those still
 * in `files`, the latest first) and then the others in their order.
 */
export function rankFiles(files: readonly string[], query: string, recent: readonly string[] = [], limit = QUICK_OPEN_MAX): string[] {
  const q = query.trim().toLowerCase().replaceAll('\\', '/');
  if (!q) {
    const known = new Set(files);
    const first = [...new Set(recent)].filter((f) => known.has(f)).slice(0, limit);
    const taken = new Set(first);
    const out = first;
    for (let i = 0; i < files.length && out.length < limit; i++) if (!taken.has(files[i])) out.push(files[i]);
    return out;
  }
  const scored: { file: string; score: number }[] = [];
  for (const file of files) {
    const s = score(file.toLowerCase(), q);
    if (s !== null) scored.push({ file, score: s });
  }
  // Stable: the files it cannot tell apart stay in the order of the tree.
  return scored
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((x) => x.file);
}

/** The files opened last in each source of each project, for the palette to show first. Kept while the app runs. */
class RecentFiles {
  private all = new Map<string, string[]>();

  private key = (projectId: string, source: string) => `${projectId}|${source}`;

  /** `path` was opened: it comes first. */
  note(projectId: string, source: string, path: string) {
    const k = this.key(projectId, source);
    const list = [path, ...(this.all.get(k) ?? []).filter((p) => p !== path)];
    this.all.set(k, list.slice(0, RECENT_MAX));
  }

  /** The files, the latest first. */
  list(projectId: string, source: string): string[] {
    return [...(this.all.get(this.key(projectId, source)) ?? [])];
  }

  /** Forgets a source that is gone (a deleted agent's worktree). */
  closeSource(projectId: string, source: string) {
    this.all.delete(this.key(projectId, source));
  }

  /** Forgets a closed project. */
  closeProject(projectId: string) {
    for (const k of [...this.all.keys()]) if (k.startsWith(`${projectId}|`)) this.all.delete(k);
  }

  reset() {
    this.all.clear();
  }
}

export const recentFiles = new RecentFiles();

let jumper: ((t: NavTarget) => void) | null = null;

/** What opening a file does in the editor on screen (it keeps the place left in its history); the function given back unsets it. */
export function setEditorJump(f: (t: NavTarget) => void): () => void {
  jumper = f;
  return () => {
    if (jumper === f) jumper = null;
  };
}

/** Opens `t` in the editor on screen, false when there is none to do it. */
export function editorJump(t: NavTarget): boolean {
  if (!jumper) return false;
  jumper(t);
  return true;
}
