import { describe, expect, it } from 'vitest';
import { ancestors, treeRows } from './tree';

const files = ['src/b.ts', 'README.md', 'src/a/x.ts', 'package.json', 'src/A.ts'];
const shape = (rows: ReturnType<typeof treeRows>) => rows.map((r) => `${r.depth}${r.kind[0]} ${r.name}`);

describe('treeRows', () => {
  it('lists folders first, then files, each sorted, folders closed', () => {
    expect(shape(treeRows(files, {}, {}))).toEqual(['0d src', '0f package.json', '0f README.md']);
  });

  it('shows what an open folder holds, one level deeper', () => {
    const rows = treeRows(files, { src: true, 'src/a': true }, {});
    expect(shape(rows)).toEqual(['0d src', '1d a', '2f x.ts', '1f A.ts', '1f b.ts', '0f package.json', '0f README.md']);
    expect(rows[0]).toMatchObject({ path: 'src', open: true });
    expect(rows[2]).toMatchObject({ path: 'src/a/x.ts', kind: 'file' });
  });

  it('marks changed files and the folders holding them', () => {
    const rows = treeRows(files, {}, { 'src/a/x.ts': 'M', 'README.md': 'A' });
    expect(rows.find((r) => r.name === 'src')).toMatchObject({ inside: 'M' });
    expect(rows.find((r) => r.name === 'README.md')).toMatchObject({ status: 'A' });
    expect(rows.find((r) => r.name === 'package.json')).toMatchObject({ status: null, inside: null });
  });

  it('gives a folder the strongest change it holds: modified, then added, then deleted', () => {
    const inside = (status: Record<string, 'M' | 'A' | 'D'>) => treeRows(['d/x', 'd/y', 'd/z'], {}, status)[0].inside;
    expect(inside({ 'd/x': 'D', 'd/y': 'A', 'd/z': 'M' })).toBe('M');
    expect(inside({ 'd/x': 'D', 'd/y': 'A' })).toBe('A');
    expect(inside({ 'd/x': 'D' })).toBe('D');
    expect(inside({ 'dx/x': 'M' })).toBe(null);
  });
});

describe('treeRows of folders holding a single folder', () => {
  const deep = ['src/lib/editor/a.ts', 'src/lib/editor/b.ts', 'README.md'];

  it('shows them as one row, opened and closed as the deepest one', () => {
    expect(shape(treeRows(deep, {}, {}))).toEqual(['0d src/lib/editor', '0f README.md']);
    const rows = treeRows(deep, { 'src/lib/editor': true }, { 'src/lib/editor/a.ts': 'M' });
    expect(rows[0]).toMatchObject({ kind: 'dir', path: 'src/lib/editor', open: true, inside: 'M' });
    expect(shape(rows)).toEqual(['0d src/lib/editor', '1f a.ts', '1f b.ts', '0f README.md']);
  });

  it('keeps a folder that holds files, or two folders, on a row of its own', () => {
    expect(shape(treeRows(['src/a.ts', 'src/lib/x.ts'], { src: true }, {}))).toEqual(['0d src', '1d lib', '1f a.ts']);
    expect(shape(treeRows(['src/a/x.ts', 'src/b/y.ts'], { src: true }, {}))).toEqual(['0d src', '1d a', '1d b']);
  });
});

describe('treeRows with a file being named', () => {
  const deep = ['src/lib/editor/a.ts', 'README.md'];

  it('puts the name field first in its folder, one level deeper', () => {
    const rows = treeRows(['src/a.ts', 'README.md'], { src: true }, {}, 'src');
    expect(shape(rows)).toEqual(['0d src', '1n ', '1f a.ts', '0f README.md']);
    expect(rows[1]).toMatchObject({ kind: 'new', path: 'src' });
  });

  it('puts it first of all at the root', () => {
    expect(shape(treeRows(['src/a.ts', 'README.md'], {}, {}, ''))).toEqual(['0n ', '0d src', '0f README.md']);
  });

  it('shows on its own row a folder of a single folder that gets the new file', () => {
    const rows = treeRows(deep, { src: true, 'src/lib': true }, {}, 'src');
    expect(shape(rows)).toEqual(['0d src', '1n ', '1d lib/editor', '0f README.md']);
  });

  it('shows no field in a closed folder', () => {
    expect(shape(treeRows(['src/a.ts'], {}, {}, 'src'))).toEqual(['0d src']);
  });
});

describe('treeRows of the files git ignores', () => {
  const rows = treeRows(['.env', 'api/.env', 'api/index.ts', 'README.md'], { api: true }, {}, null, ['.env', 'api/.env']);

  it('marks the ignored files, and no folder, whatever they hold', () => {
    expect(rows.filter((r) => r.ignored).map((r) => r.path)).toEqual(['api/.env', '.env']);
    expect(rows.find((r) => r.name === 'api')).toMatchObject({ kind: 'dir', ignored: false });
  });

  it('marks none without the list the backend gives', () => {
    expect(treeRows(['.env'], {}, {}).every((r) => !r.ignored)).toBe(true);
  });

  it('marks a file by its path, not by its name', () => {
    const named = treeRows(['.env', 'api/.env'], { api: true }, {}, null, ['api/.env']);
    expect(named.map((r) => [r.path, r.ignored])).toEqual([
      ['api', false],
      ['api/.env', true],
      ['.env', false],
    ]);
  });
});

describe('treeRows with names that Object has too', () => {
  it('keeps a folder named like an Object member closed and a file named so unchanged', () => {
    const rows = treeRows(['constructor/x.ts', 'toString'], {}, {});
    expect(rows.find((r) => r.name === 'constructor')).toMatchObject({ kind: 'dir', open: false });
    expect(rows.find((r) => r.name === 'toString')).toMatchObject({ kind: 'file', status: null });
    expect(shape(rows)).toEqual(['0d constructor', '0f toString']);
  });
});

describe('ancestors', () => {
  it('gives the folders above a file, outermost first', () => {
    expect(ancestors('src/a/x.ts')).toEqual(['src', 'src/a']);
    expect(ancestors('x.ts')).toEqual([]);
  });
});
