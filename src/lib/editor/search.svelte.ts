// « Rechercher dans les fichiers » (Ctrl+Maj+F, Maj+F12) in the editor's left column: what is looked for in the files
// of the source shown, sent to `code_search` 250 ms after the last change, and the answer of the last search only
// (the backend does not stop a search given up: its answer is dropped when it comes).

import { escapeRegExp } from '../format';
import { t } from '../i18n';
import { api } from '../ipc';
import type { SearchMatch, SearchQuery, SearchResult } from '../types';
import { sourceAgent } from './buffers.svelte';

/** Lines asked of a search; the panel says when there were more. */
export const SEARCH_MAX = 2000;
/** The pause after the last key or option, in milliseconds, before a search starts. */
export const SEARCH_PAUSE_MS = 250;

export type SearchOption = 'caseSensitive' | 'wholeWord' | 'regex';

/** A search of a source ('project' or an agent's id). */
export type SearchRun = (source: string, q: SearchQuery) => Promise<SearchResult>;

/** Whether JavaScript reads `pattern` as an expression: one it does not is not sent. */
function valid(pattern: string): boolean {
  try {
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

/** The search of a project's editor, kept while the app runs: the editor closed and opened again finds it as it was. */
export class FileSearch {
  text = $state('');
  caseSensitive = $state(false);
  wholeWord = $state(false);
  regex = $state(false);
  /** The left column shows the search rather than the files. */
  shown = $state(false);
  /** The answer of the last search, for the text and options it was sent with; null before one, or with none to show. */
  result = $state.raw<SearchResult | null>(null);
  /** Why there are no results: the expression is invalid, or git refused it (its message). */
  error = $state<string | null>(null);
  /** A search waits for its pause or its answer: the results shown are those of the one before. */
  pending = $state(false);

  private source = 'project';
  /** The number of the last search sent: only its answer is taken. */
  private seq = 0;
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** What the search under way was sent with: the same is not sent again before it answers. */
  private flying: string | null = null;
  /** What the results shown were found with: a change undone within the pause sends nothing. */
  private answered: string | null = null;

  constructor(
    private readonly run: SearchRun,
    private readonly pause = SEARCH_PAUSE_MS,
  ) {}

  /** The field typed in: searched after the pause, or emptied right away. */
  type(text: string) {
    this.text = text;
    this.changed();
  }

  toggle(o: SearchOption) {
    this[o] = !this[o];
    this.changed();
  }

  /** Looks for `text` with these options right away (Maj+F12, a selection). */
  ask(q: Partial<Pick<FileSearch, 'text' | SearchOption>>) {
    if (q.text !== undefined) this.text = q.text;
    if (q.caseSensitive !== undefined) this.caseSensitive = q.caseSensitive;
    if (q.wholeWord !== undefined) this.wholeWord = q.wholeWord;
    if (q.regex !== undefined) this.regex = q.regex;
    this.now();
  }

  /** Looks for the text `selected` in the code as it is written, even when the field reads an expression. */
  seed(selected: string) {
    this.ask({ text: this.regex ? escapeRegExp(selected) : selected });
  }

  /** The source shown: another one is searched again, the results of the previous one gone at once. */
  setSource(source: string) {
    if (source === this.source) return;
    this.source = source;
    this.result = null;
    this.error = null;
    this.answered = null;
    this.now();
  }

  /** Searches right away (Entrée), even what the results show already: the files may have changed since. */
  now() {
    this.send(true);
  }

  /** Stops: no search starts any more, no answer is taken. */
  dispose() {
    clearTimeout(this.timer);
    this.seq++;
    this.flying = null;
    this.pending = false;
  }

  private changed() {
    clearTimeout(this.timer);
    // Nothing to wait for: an empty field clears the results now.
    if (!this.text) return this.send(true);
    this.pending = true;
    this.timer = setTimeout(() => this.send(false), this.pause);
  }

  private send(again: boolean) {
    clearTimeout(this.timer);
    const q: SearchQuery = {
      pattern: this.text,
      regex: this.regex,
      caseSensitive: this.caseSensitive,
      wholeWord: this.wholeWord,
      maxResults: SEARCH_MAX,
    };
    const key = JSON.stringify([this.source, q]);
    // Its answer is coming.
    if (key === this.flying) return;
    const n = ++this.seq;
    this.flying = null;
    if (!q.pattern || (q.regex && !valid(q.pattern))) {
      this.result = null;
      this.answered = null;
      this.error = q.pattern ? t('editor.search.invalidRegex') : null;
      this.pending = false;
      return;
    }
    // Back to what the results show (the search under way, if another, is dropped).
    if (!again && key === this.answered) {
      this.pending = false;
      return;
    }
    this.flying = key;
    this.pending = true;
    const done = (result: SearchResult | null, error: string | null) => {
      if (n !== this.seq) return;
      this.result = result;
      this.error = error;
      this.answered = result ? key : null;
      this.flying = null;
      this.pending = false;
    };
    this.run(this.source, q).then(
      (r) => done(r, null),
      (e) => done(null, String(e)),
    );
  }
}

class FileSearches {
  private all = new Map<string, FileSearch>();

  of(projectId: string): FileSearch {
    let s = this.all.get(projectId);
    if (!s) this.all.set(projectId, (s = new FileSearch((source, q) => api.codeSearch(projectId, sourceAgent(source), q))));
    return s;
  }

  /** Forgets the search of a closed project. */
  closeProject(projectId: string) {
    this.all.get(projectId)?.dispose();
    this.all.delete(projectId);
  }

  reset() {
    for (const s of this.all.values()) s.dispose();
    this.all.clear();
  }
}

export const fileSearches = new FileSearches();

let finder: (() => void) | null = null;

/** What Ctrl+Maj+F does, set by the editor on screen; the function given back unsets it. */
export function setFindInFiles(f: () => void): () => void {
  finder = f;
  return () => {
    if (finder === f) finder = null;
  };
}

/** Ctrl+Maj+F: false without an editor on screen to search from. */
export function findInFiles(): boolean {
  if (!finder) return false;
  finder();
  return true;
}

/** A stretch of a line found: a match, or the text around. */
export interface Piece {
  text: string;
  hit: boolean;
}

/** Characters shown before a line's first match: the match must show in a column a few dozen characters wide. */
const LEAD = 16;

/**
 * The text of a line found, in pieces, its matches marked; without its indentation, nor what comes long before its
 * first match (`…` then says the line goes on before, as it does when git sent a stretch of a long line). The ranges
 * count characters (code points): split into them, the text never cuts an emoji in two.
 */
export function pieces(m: Pick<SearchMatch, 'text' | 'offset' | 'ranges'>): Piece[] {
  const chars = [...m.text];
  const spans = m.ranges
    .map(([s, e]) => [Math.max(0, s), Math.min(chars.length, e)])
    .filter(([s, e]) => e > s)
    .sort((a, b) => a[0] - b[0]);
  const joined: number[][] = [];
  for (const [s, e] of spans) {
    const last = joined.at(-1);
    if (last && s <= last[1]) last[1] = Math.max(last[1], e);
    else joined.push([s, e]);
  }
  const indent = chars.findIndex((c) => !/\s/.test(c));
  let start = indent < 0 ? chars.length : indent;
  // Without a match (none marked by git), there is nothing to bring into view: the line shows from its indentation.
  const first = joined[0]?.[0];
  if (first !== undefined) start = Math.min(start, first);
  const cut = first !== undefined && first - start > LEAD;
  if (cut) start = first - LEAD;
  const out: Piece[] = m.offset > 0 || cut ? [{ text: '…', hit: false }] : [];
  const add = (from: number, to: number, hit: boolean) => to > from && out.push({ text: chars.slice(from, to).join(''), hit });
  let at = start;
  for (const [s, e] of joined) {
    add(at, s, false);
    add(s, e, true);
    at = e;
  }
  add(at, chars.length, false);
  return out;
}
