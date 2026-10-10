// Lines of the editor that differ from the reference version (HEAD, or where a worktree's branch
// left its base): drawn in the gutter and counted in the header.

import { diff } from '@codemirror/merge';

export interface LineChanges {
  /** Lines (1-based) added or modified. */
  changed: number[];
  /** Lines (1-based) above which lines were removed. */
  deleted: number[];
  count: number;
}

const NONE = (): LineChanges => ({ changed: [], deleted: [], count: 0 });

/** The longest the diff may run before it settles for a cruder answer, in ms. */
export const DIFF_TIMEOUT = 300;

// The diff runs on lines, not on characters: a character-level diff of two big files takes seconds. Each
// distinct line stands for one character (from U+0100, so never a line break or other control; surrogates
// skipped, for a pair would count as two positions), and the diff of those two short strings is the diff
// of the lines.
const FIRST_TOKEN = 0x100;
const LAST_TOKEN = 0xffff;
const SURROGATES_FROM = 0xd800;
const SURROGATES_TO = 0xdfff;

/** The lines as one string of tokens, or null when there are more distinct lines than tokens. */
function tokenize(lines: readonly string[], tokens: Map<string, string>, next: { code: number }): string | null {
  const out: string[] = new Array(lines.length);
  for (let i = 0; i < lines.length; i++) {
    let token = tokens.get(lines[i]);
    if (token === undefined) {
      if (next.code > LAST_TOKEN) return null;
      token = String.fromCharCode(next.code);
      tokens.set(lines[i], token);
      next.code = next.code + 1 === SURROGATES_FROM ? SURROGATES_TO + 1 : next.code + 1;
    }
    out[i] = token;
  }
  return out.join('');
}

/** The lines of two texts as strings of tokens, the same line the same token in both; null when there are too many. */
export function lineTokens(a: readonly string[], b: readonly string[]): [string, string] | null {
  const tokens = new Map<string, string>();
  const next = { code: FIRST_TOKEN };
  const ta = tokenize(a, tokens, next);
  const tb = ta === null ? null : tokenize(b, tokens, next);
  return ta === null || tb === null ? null : [ta, tb];
}

export function lineChanges(base: string | null, current: string): LineChanges {
  const lines = current.split('\n');
  if (base === null) {
    // A trailing line break opens an empty last line: not a line of the file.
    const last = lines[lines.length - 1] === '' ? lines.length - 1 : lines.length;
    const changed = Array.from({ length: last }, (_, i) => i + 1);
    return { changed, deleted: [], count: changed.length };
  }
  if (base === current) return NONE();
  const tokens = lineTokens(base.split('\n'), lines);
  if (!tokens) return NONE();
  const [a, b] = tokens;
  const changed = new Set<number>();
  const deleted = new Set<number>();
  for (const c of diff(a, b, { timeout: DIFF_TIMEOUT })) {
    if (c.toB > c.fromB) {
      for (let n = c.fromB + 1; n <= c.toB; n++) changed.add(n);
    } else if (c.toA > c.fromA) {
      deleted.add(Math.min(c.fromB + 1, lines.length));
    }
  }
  const sorted = (s: Set<number>) => [...s].sort((x, y) => x - y);
  return { changed: sorted(changed), deleted: sorted(deleted), count: changed.size };
}
