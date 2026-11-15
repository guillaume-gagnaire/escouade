import { describe, expect, it } from 'vitest';
import {
  LARGE_LINES,
  VIEWED_KEPT,
  chunks,
  editorLine,
  fileMark,
  fileTree,
  firstLine,
  foldReason,
  isViewed,
  keptViewed,
  langOf,
  matches,
  setViewed,
  sides,
  treeOrder,
  viewedCount,
  viewedKey,
} from './prfiles';
import type { PrContentsFile, PrLine } from './types';

const file = (path: string, over: Partial<PrContentsFile> = {}): PrContentsFile => ({
  path,
  status: 'modified',
  additions: 1,
  deletions: 1,
  lines: [
    { kind: '@', text: '@@ -1,3 +1,3 @@' },
    { kind: ' ', old: 1, new: 1, text: 'const a = 1;' },
    { kind: '-', old: 2, text: 'const b = 2;' },
    { kind: '+', new: 2, text: 'const b = 3;' },
    { kind: ' ', old: 3, new: 3, text: 'const c = 4;' },
  ],
  binary: false,
  generated: false,
  cut: false,
  ...over,
});

describe('the tree of the files changed (ESC-467)', () => {
  const files = [
    file('src/auth/magic-link.ts'),
    file('src/auth/session.ts', { status: 'added' }),
    file('README.md'),
    file('src/lib/a/b/deep.ts'),
    file('package-lock.json', { generated: true }),
  ];

  it('puts the folders first, then the files, and says a folder of a single folder with it', () => {
    const rows = fileTree(files).map((r) => `${'  '.repeat(r.depth)}${r.kind === 'dir' ? `${r.name}/` : r.name}`);
    expect(rows).toEqual([
      'src/',
      '  auth/',
      '    magic-link.ts',
      '    session.ts',
      '  lib/a/b/',
      '    deep.ts',
      'package-lock.json',
      'README.md',
    ]);
    expect(treeOrder(files).map((f) => f.path)).toEqual([
      'src/auth/magic-link.ts',
      'src/auth/session.ts',
      'src/lib/a/b/deep.ts',
      'package-lock.json',
      'README.md',
    ]);
  });

  it('filters by every word of the filter, without case, and leaves a closed folder’s files out', () => {
    expect(matches('src/auth/Session.ts', 'AUTH sess')).toBe(true);
    expect(matches('src/auth/session.ts', 'auth lock')).toBe(false);
    expect(treeOrder(files, 'auth').map((f) => f.path)).toEqual(['src/auth/magic-link.ts', 'src/auth/session.ts']);
    const closed = fileTree(files, '', { 'src/auth': true }).map((r) => r.path);
    expect(closed).toContain('src/auth');
    expect(closed).not.toContain('src/auth/session.ts');
  });
});

describe('« Vu », by pull request and by commit', () => {
  it('a file marked in the whole pull request is no longer once a commit changes it; another file stays', () => {
    const a = file('a.ts');
    const b = file('b.ts');
    let v = setViewed(undefined, 'pr', a, true, 1);
    v = setViewed(v, 'pr', b, true, 2);
    expect(viewedCount(v, 'pr', [a, b])).toBe(2);
    // A new commit changes a.ts: its lines, hence its mark, are not the same.
    const a2 = file('a.ts', { lines: [...a.lines, { kind: '+', new: 4, text: 'const d = 5;' }], additions: 2 });
    expect(fileMark(a2)).not.toBe(fileMark(a));
    expect(isViewed(v, 'pr', a2)).toBe(false);
    expect(isViewed(v, 'pr', b)).toBe(true);
    // Unmarked.
    v = setViewed(v, 'pr', b, false, 3);
    expect(isViewed(v, 'pr', b)).toBe(false);
  });

  it('a commit keeps its own: marking in it leaves the pull request’s as it was', () => {
    const a = file('a.ts');
    let v = setViewed(undefined, { sha: 'abc' }, a, true, 1);
    expect(isViewed(v, { sha: 'abc' }, a)).toBe(true);
    expect(isViewed(v, { sha: 'def' }, a)).toBe(false);
    expect(isViewed(v, 'pr', a)).toBe(false);
    v = setViewed(v, { sha: 'abc' }, a, false, 2);
    expect(v.commits).toEqual({});
  });

  it('keeps the last marked pull requests, and drops what is not one', () => {
    expect(viewedKey('p1', 'Acme/Demo', 12)).toBe('p1#acme/demo#12');
    const all: Record<string, unknown> = { bad: { at: 'x' }, worse: null };
    for (let i = 0; i < VIEWED_KEPT + 5; i++) all[`k${i}`] = { at: i, pr: {}, commits: {} };
    const kept = keptViewed(all);
    expect(Object.keys(kept)).toHaveLength(VIEWED_KEPT);
    expect(kept.k0).toBeUndefined();
    expect(kept[`k${VIEWED_KEPT + 4}`]).toBeDefined();
    expect(keptViewed(undefined)).toEqual({});
  });
});

describe('folded files, lines and languages', () => {
  it('folds a binary, a generated and a large file, not the others', () => {
    const many: PrLine[] = Array.from({ length: LARGE_LINES + 1 }, (_, i) => ({ kind: '+', new: i + 1, text: 'x' }));
    expect(foldReason(file('logo.png', { binary: true, lines: [] }))).toBe('binary');
    expect(foldReason(file('yarn.lock', { generated: true }))).toBe('generated');
    expect(foldReason(file('big.md', { lines: many }))).toBe('large');
    expect(foldReason(file('a.ts'))).toBeNull();
    expect(chunks(many, 100).map((c) => [c.from, c.lines.length])).toEqual([
      [0, 100],
      [100, 100],
      [200, 100],
      [300, 100],
      [400, 1],
    ]);
  });

  it('opens a line at its number in the tip, a removed one where it stood, nothing for a removed file', () => {
    const f = file('a.ts');
    expect(editorLine(f, 0)).toBeNull();
    expect(editorLine(f, 1)).toBe(1);
    expect(editorLine(f, 2)).toBe(2);
    expect(editorLine(f, 3)).toBe(2);
    expect(firstLine(f)).toBe(2);
    expect(editorLine(file('gone.ts', { status: 'removed' }), 1)).toBeNull();
    expect(firstLine(file('gone.ts', { status: 'removed' }))).toBeNull();
  });

  it('colors each side of a file as one text, and knows a file’s language by its name', () => {
    const s = sides(file('a.ts').lines);
    expect(s.tip).toEqual(['const a = 1;', 'const b = 3;', 'const c = 4;']);
    expect(s.base).toEqual(['const a = 1;', 'const b = 2;', 'const c = 4;']);
    expect(s.at).toEqual([
      { side: 'tip', i: -1 },
      { side: 'tip', i: 0 },
      { side: 'base', i: 1 },
      { side: 'tip', i: 1 },
      { side: 'tip', i: 2 },
    ]);
    expect(langOf('src/a.ts')).toBe('ts');
    expect(langOf('src-tauri/src/lib.rs')).toBe('rust');
    expect(langOf('Dockerfile')).toBe('dockerfile');
    expect(langOf('LICENSE')).toBeNull();
    expect(langOf('logo.png')).toBeNull();
  });
});
