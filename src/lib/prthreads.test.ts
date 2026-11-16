import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { extendPick, lineIndex, nextStop, openOrder, pickOf, placeThreads, threadCounts, threadKey, writeFailure } from './prthreads';
import type { PrContentsFile, PrThread } from './types';

// The threads of a pull request in the tab « Fichiers » (ESC-474): under their line, apart when their line is gone,
// « Suivante » from an open one to the next, the lines a comment is on, and a write's failure in words.

const FILE: PrContentsFile = {
  path: 'src/a.ts',
  status: 'modified',
  additions: 2,
  deletions: 1,
  lines: [
    { kind: '@', text: '@@ -1,3 +1,4 @@' },
    { kind: ' ', old: 1, new: 1, text: 'a' },
    { kind: '-', old: 2, text: 'b' },
    { kind: '+', new: 2, text: 'B' },
    { kind: '+', new: 3, text: 'C' },
    { kind: ' ', old: 3, new: 4, text: 'd' },
  ],
  binary: false,
  generated: false,
  cut: false,
};
const OTHER: PrContentsFile = { ...FILE, path: 'src/b.ts' };

const th = (id: string, over: Partial<PrThread>): PrThread => ({
  id,
  path: FILE.path,
  side: 'right',
  state: 'open',
  comments: [{ id: `${id}-1`, author: 'bob', body: 'x' }],
  ...over,
});

beforeEach(() => setLang('fr'));

describe('the threads of the tab « Fichiers » (ESC-474)', () => {
  it('goes under its line: the tip’s number on its side, the base’s for a line removed', () => {
    expect(lineIndex(FILE.lines, 2, 'right')).toBe(3);
    expect(lineIndex(FILE.lines, 2, 'left')).toBe(2);
    expect(lineIndex(FILE.lines, 4, 'right')).toBe(5);
    expect(lineIndex(FILE.lines, 9, 'right')).toBe(-1);
  });

  it('places each thread under its line, apart when its line is gone or it is outdated, apart from the files when its file is not changed', () => {
    const threads = [
      th('T1', { line: 2 }),
      th('T2', { line: 2, side: 'left' }),
      th('T3', { line: 40 }),
      th('T4', { line: 1, state: 'outdated' }),
      th('T5', { path: 'README.md', line: 1 }),
      th('T6', { path: undefined }),
      th('T7', { line: 4, state: 'resolved' }),
    ];
    const p = placeThreads([FILE, OTHER], threads);
    expect(p.under[FILE.path][3].map((t) => t.id)).toEqual(['T1']);
    expect(p.under[FILE.path][2].map((t) => t.id)).toEqual(['T2']);
    expect(p.under[FILE.path][5].map((t) => t.id)).toEqual(['T7']);
    expect(p.stale[FILE.path].map((t) => t.id)).toEqual(['T3', 'T4']);
    expect(p.elsewhere.map((t) => t.id)).toEqual(['T5']);
    expect(threadCounts(threads)).toEqual({ open: 5, resolved: 1 });
  });

  it('« Suivante » goes from an open thread to the next in the order of the tab, and from the last to the first', () => {
    const threads = [
      th('B', { path: OTHER.path, line: 1 }),
      th('A2', { line: 4 }),
      th('A1', { line: 1 }),
      th('A0', { line: 40 }),
      th('R', { line: 3, state: 'resolved' }),
      th('E', { path: 'README.md', line: 1 }),
    ];
    const stops = openOrder([FILE, OTHER], placeThreads([FILE, OTHER], threads));
    expect(stops.map((s) => s.key)).toEqual(['E', 'A0', 'A1', 'A2', 'B']);
    expect(nextStop(stops, null)?.key).toBe('E');
    expect(nextStop(stops, 'A1')?.key).toBe('A2');
    expect(nextStop(stops, 'B')?.key).toBe('E');
    expect(nextStop(stops, 'E', -1)?.key).toBe('B');
    expect(nextStop([], null)).toBeNull();
    expect(threadKey(th('', { line: 3 }))).toBe('src/a.ts:3:-1');
  });

  it('a comment goes on a line or a range of the same side', () => {
    const one = pickOf(FILE.path, FILE.lines[3], 3)!;
    expect(one).toEqual({ path: FILE.path, side: 'right', line: 2, index: 3 });
    expect(pickOf(FILE.path, FILE.lines[2], 2)).toMatchObject({ side: 'left', line: 2 });
    expect(pickOf(FILE.path, FILE.lines[0], 0)).toBeNull();
    const to = pickOf(FILE.path, FILE.lines[5], 5)!;
    expect(extendPick(one, to)).toEqual({ path: FILE.path, side: 'right', line: 4, startLine: 2, index: 5 });
    expect(extendPick(to, one)).toEqual({ path: FILE.path, side: 'right', line: 4, startLine: 2, index: 5 });
    expect(extendPick(pickOf(FILE.path, FILE.lines[2], 2), to)).toBe(to);
  });

  it('says why a write did not go, its draft kept', () => {
    expect(writeFailure({ kind: 'forbidden', message: '403' }, 'GitHub')).toBe(
      'Ton jeton GitHub n’a pas le droit d’écrire sur ce dépôt. Rien n’est parti, ton brouillon est gardé.',
    );
    expect(writeFailure({ kind: 'gone', message: '404' }, 'GitHub')).toContain('n’existe plus sur GitHub');
    expect(writeFailure({ kind: 'unreachable', message: '' }, 'GitHub', false)).toBe(
      'Hors ligne : rien n’est parti, ton brouillon est gardé.',
    );
    expect(writeFailure({ kind: 'unreachable', message: '' }, 'GitHub')).toBe(
      'GitHub injoignable : rien n’est parti, ton brouillon est gardé.',
    );
    expect(writeFailure({ kind: 'rateLimit', message: '', reset: 0 }, 'GitHub')).toContain('Limite d’API de GitHub');
    expect(writeFailure({ kind: 'other', message: 'Le commentaire est vide.' }, 'GitHub')).toBe('Le commentaire est vide.');
  });
});
