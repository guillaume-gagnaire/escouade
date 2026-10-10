import { describe, expect, it, vi } from 'vitest';
import { editorJump, parseQuickQuery, QUICK_OPEN_MAX, rankFiles, recentFiles, setEditorJump } from './quick-open';

describe('parseQuickQuery', () => {
  it('reads a name, with the line and the column that follow it', () => {
    expect(parseQuickQuery('app.ts')).toEqual({ name: 'app.ts' });
    expect(parseQuickQuery('app.ts:42')).toEqual({ name: 'app.ts', line: 42 });
    expect(parseQuickQuery('src/app.ts:42:7')).toEqual({ name: 'src/app.ts', line: 42, col: 7 });
    expect(parseQuickQuery('  app:42  ')).toEqual({ name: 'app', line: 42 });
  });

  it('takes the colon of a Windows path for a part of the name', () => {
    expect(parseQuickQuery('C:\\code\\app.ts:3')).toEqual({ name: 'C:\\code\\app.ts', line: 3 });
  });

  it('waits for the digits: a colon alone, or a line 0, leaves the name', () => {
    expect(parseQuickQuery('app.ts:')).toEqual({ name: 'app.ts' });
    expect(parseQuickQuery('app.ts:0')).toEqual({ name: 'app.ts' });
    expect(parseQuickQuery('app.ts:12:')).toEqual({ name: 'app.ts', line: 12 });
    expect(parseQuickQuery('app.ts:12:0')).toEqual({ name: 'app.ts', line: 12 });
  });

  it('leaves what is no line as it is', () => {
    expect(parseQuickQuery('a:b')).toEqual({ name: 'a:b' });
    expect(parseQuickQuery(':42')).toEqual({ name: ':42' });
    expect(parseQuickQuery('')).toEqual({ name: '' });
  });
});

describe('rankFiles', () => {
  const FILES = [
    'README.md',
    'src/app.ts',
    'src/components/ConvSearch.svelte',
    'src/lib/editor/file-tree.ts',
    'src/lib/editor/subtree.ts',
    'src/lib/editor/tree.ts',
    'src/lib/application.ts',
    'src/lib/format.ts',
  ];

  it('lists, with nothing typed, the recent files first and the others in their order, without repeating any', () => {
    expect(rankFiles(FILES, '', ['src/lib/format.ts', 'gone.ts', 'README.md'], 4)).toEqual([
      'src/lib/format.ts',
      'README.md',
      'src/app.ts',
      'src/components/ConvSearch.svelte',
    ]);
    expect(rankFiles(FILES, '   ', [])).toEqual(FILES);
  });

  it('puts first the files whose name starts as typed, then those holding it, then those whose folders do', () => {
    expect(rankFiles(FILES, 'app')).toEqual(['src/app.ts', 'src/lib/application.ts']);
    expect(rankFiles(FILES, 'tree').slice(0, 3)).toEqual([
      'src/lib/editor/tree.ts',
      'src/lib/editor/file-tree.ts',
      'src/lib/editor/subtree.ts',
    ]);
    expect(rankFiles(['a/lib/x.ts', 'lib/y.ts', 'src/library.ts'], 'lib')).toEqual(['src/library.ts', 'lib/y.ts', 'a/lib/x.ts']);
  });

  it('prefers the exact name, then the shorter path', () => {
    expect(rankFiles(['src/index.tsx', 'lib/deep/er/index.ts', 'index.ts'], 'index.ts')).toEqual([
      'index.ts',
      'lib/deep/er/index.ts',
      'src/index.tsx',
    ]);
    expect(rankFiles(['aaaaa/b.ts', 'b.ts'], 'b')).toEqual(['b.ts', 'aaaaa/b.ts']);
  });

  it('gives a word start the lead: after a slash, a dash or a dot', () => {
    // `tree` starts a word in file-tree.ts, not in subtree.ts, which is shorter.
    expect(rankFiles(['src/lib/editor/subtree.ts', 'src/lib/editor/file-tree.ts'], 'tree')[0]).toBe('src/lib/editor/file-tree.ts');
    expect(rankFiles(['a/mylib/x.ts', 'a/lib/x.ts'], 'lib/x')).toEqual(['a/lib/x.ts', 'a/mylib/x.ts']);
  });

  it('finds the letters in order wherever they are, the file name first and the word starts ahead', () => {
    expect(rankFiles(FILES, 'cvsrch')).toEqual(['src/components/ConvSearch.svelte']);
    expect(rankFiles(['fabric.ts', 'foo-bar.ts', 'src/f/b.ts'], 'fb')).toEqual(['foo-bar.ts', 'fabric.ts', 'src/f/b.ts']);
    expect(rankFiles(FILES, 'zzz')).toEqual([]);
    // In order: `ppa` is not in `app.ts`.
    expect(rankFiles(['src/app.ts'], 'ppa')).toEqual([]);
  });

  it('ignores the case, and reads a backslash as a slash', () => {
    expect(rankFiles(FILES, 'CONVSEARCH')).toEqual(['src/components/ConvSearch.svelte']);
    expect(rankFiles(FILES, 'src\\lib\\for')).toEqual(['src/lib/format.ts']);
    expect(rankFiles(FILES, '  readme ')).toEqual(['README.md']);
  });

  it('shows 50 files at most', () => {
    const many = Array.from({ length: 80 }, (_, i) => `src/file${String(i).padStart(2, '0')}.ts`);
    const found = rankFiles(many, 'file');
    expect(found).toHaveLength(QUICK_OPEN_MAX);
    expect(QUICK_OPEN_MAX).toBe(50);
    expect(found[0]).toBe('src/file00.ts');
    expect(rankFiles(many, '')).toHaveLength(50);
    expect(rankFiles(many, 'file', [], 3)).toHaveLength(3);
  });

  it('keeps the order of the files it cannot tell apart', () => {
    expect(rankFiles(['b/x.ts', 'a/x.ts'], 'x.ts')).toEqual(['b/x.ts', 'a/x.ts']);
  });

  it('ranks 50 000 paths without a list of the whole', () => {
    const files = Array.from({ length: 50_000 }, (_, i) => `packages/p${i % 500}/src/module${i}.ts`);
    const found = rankFiles(files, 'module4999');
    expect(found[0]).toBe('packages/p499/src/module4999.ts');
    expect(found.length).toBeLessThanOrEqual(50);
  });
});

describe('recentFiles', () => {
  it('keeps the files opened last first, once each, by project and source', () => {
    recentFiles.reset();
    recentFiles.note('p1', 'project', 'a.ts');
    recentFiles.note('p1', 'project', 'b.ts');
    recentFiles.note('p1', 'project', 'a.ts');
    recentFiles.note('p1', 'a1', 'c.ts');
    recentFiles.note('p2', 'project', 'd.ts');
    expect(recentFiles.list('p1', 'project')).toEqual(['a.ts', 'b.ts']);
    expect(recentFiles.list('p1', 'a1')).toEqual(['c.ts']);
    expect(recentFiles.list('p3', 'project')).toEqual([]);
  });

  it('forgets the oldest past thirty', () => {
    recentFiles.reset();
    for (let i = 0; i < 35; i++) recentFiles.note('p1', 'project', `f${i}.ts`);
    const list = recentFiles.list('p1', 'project');
    expect(list).toHaveLength(30);
    expect(list[0]).toBe('f34.ts');
    expect(list.at(-1)).toBe('f5.ts');
  });

  it('forgets a source that is gone, or a whole project', () => {
    recentFiles.reset();
    recentFiles.note('p1', 'project', 'a.ts');
    recentFiles.note('p1', 'a1', 'b.ts');
    recentFiles.note('p2', 'project', 'c.ts');
    recentFiles.closeSource('p1', 'a1');
    expect(recentFiles.list('p1', 'a1')).toEqual([]);
    expect(recentFiles.list('p1', 'project')).toEqual(['a.ts']);
    recentFiles.closeProject('p1');
    expect(recentFiles.list('p1', 'project')).toEqual([]);
    expect(recentFiles.list('p2', 'project')).toEqual(['c.ts']);
    // A project whose id starts like another's is not taken with it.
    recentFiles.note('p10', 'project', 'x.ts');
    recentFiles.closeProject('p1');
    expect(recentFiles.list('p10', 'project')).toEqual(['x.ts']);
  });

  it('is not a copy to write into', () => {
    recentFiles.reset();
    recentFiles.note('p1', 'project', 'a.ts');
    recentFiles.list('p1', 'project').push('b.ts');
    expect(recentFiles.list('p1', 'project')).toEqual(['a.ts']);
  });
});

describe('editorJump', () => {
  it('is for the editor on screen to answer, and false without one', () => {
    expect(editorJump({ path: 'a.ts' })).toBe(false);
    const jump = vi.fn();
    const unset = setEditorJump(jump);
    expect(editorJump({ path: 'a.ts', line: 3 })).toBe(true);
    expect(jump).toHaveBeenCalledWith({ path: 'a.ts', line: 3 });
    unset();
    expect(editorJump({ path: 'a.ts' })).toBe(false);
  });

  it('is not unset by an editor that was replaced', () => {
    const first = vi.fn();
    const second = vi.fn();
    const unsetFirst = setEditorJump(first);
    const unsetSecond = setEditorJump(second);
    unsetFirst();
    expect(editorJump({ path: 'a.ts' })).toBe(true);
    expect(second).toHaveBeenCalledTimes(1);
    unsetSecond();
  });
});
