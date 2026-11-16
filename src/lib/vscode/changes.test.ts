import { describe, expect, it, vi } from 'vitest';
import type { FileBase, FileChange } from '../types';
import { BASE_SCHEME, BaseChanges, changedLines, filesOf, FOLDER_DOT, LETTER_COLORS, type ChangesUri, type CoreChanges } from './changes';
import type { Source, UriLike } from './fs-provider';
import { relativeTo } from './tree';

/** A URI as VS Code writes it: `scheme://authority/path?query`. */
const uri = (u: { scheme: string; authority: string; path: string; query?: string }): ChangesUri => {
  const q = u.query ?? '';
  return {
    scheme: u.scheme,
    authority: u.authority,
    path: u.path,
    query: q,
    toString: () => `${u.scheme}://${u.authority}${u.path}${q ? `?${q}` : ''}`,
  };
};
const file = (path: string) => uri({ scheme: 'file', authority: '', path });

const PROJECT: Source = { projectId: 'p1', agentId: null };
const AGENT: Source = { projectId: 'p1', agentId: 'a1' };
/** The project's checkout, an agent's worktree in it (its own source), as the file system provider serves them. */
const ROOTS: [Source, string][] = [
  [PROJECT, '/c:/code/demo'],
  [AGENT, '/c:/code/demo/.claude/worktrees/a1'],
];

const change = (path: string, status: FileChange['status'], agentId: string | null = null, inWorktree = false): FileChange => ({
  path,
  status,
  add: 1,
  del: 0,
  agentId,
  inWorktree,
});

/** The core: the bases by source and path, the files `git_files` lists for each. */
function fakeCore() {
  const bases = new Map<string, FileBase | null>();
  const lists = new Map<string, FileChange[]>();
  const key = (s: string | null, p = '') => `${s ?? 'project'}:${p}`;
  const core: CoreChanges = {
    fsBase: vi.fn(async (_p: string, agentId: string | null, path: string) => bases.get(key(agentId, path)) ?? null),
    gitFiles: vi.fn(async (_p: string, agentId: string | null) => lists.get(key(agentId)) ?? []),
  };
  return {
    core,
    base: (src: Source, path: string, b: FileBase | null) => bases.set(key(src.agentId, path), b),
    list: (src: Source, files: FileChange[]) => lists.set(key(src.agentId), files),
  };
}

function setup() {
  const fake = fakeCore();
  const sourceOf = (u: UriLike) => {
    if (u.scheme !== 'file') return null;
    let best: { source: Source; rel: string; len: number } | null = null;
    for (const [source, root] of ROOTS) {
      const rel = relativeTo(root, u.path, true);
      if (rel !== null && (!best || root.length > best.len)) best = { source, rel, len: root.length };
    }
    return best && { source: best.source, rel: best.rel };
  };
  const uriOf = (src: Source, rel: string) => {
    const root = ROOTS.find(([s]) => s.projectId === src.projectId && s.agentId === src.agentId)?.[1];
    return root ? file(`${root}/${rel}`) : null;
  };
  const changes = new BaseChanges<ChangesUri>(fake.core, { uri, sourceOf, uriOf }, true);
  return { ...fake, changes };
}

describe('the base of a file (the original resource of the quick diff)', () => {
  it('is the file under escouade-base:, for a file of a source served only', () => {
    const { changes } = setup();
    const base = changes.originalOf(file('/c:/code/demo/src/app.ts'));
    expect(base?.toString()).toBe(`${BASE_SCHEME}:///c:/code/demo/src/app.ts`);
    expect(changes.fileOf(base!)?.toString()).toBe('file:///c:/code/demo/src/app.ts');
    expect(changes.originalOf(file('/c:/elsewhere/app.ts'))).toBeNull();
    expect(changes.originalOf(uri({ scheme: 'untitled', authority: '', path: '/Untitled-1' }))).toBeNull();
    // A base has no base.
    expect(changes.originalOf(base!)).toBeNull();
  });

  it('is read by the core for its own source: the project at HEAD, the worktree where it left main', async () => {
    const { changes, core, base } = setup();
    base(PROJECT, 'src/app.ts', { reference: 'HEAD', text: 'const a = 1;\n' });
    base(AGENT, 'src/app.ts', { reference: 'main', text: 'const a = 0;\n' });
    expect(await changes.text(changes.originalOf(file('/c:/code/demo/src/app.ts'))!)).toBe('const a = 1;\n');
    expect(core.fsBase).toHaveBeenLastCalledWith('p1', null, 'src/app.ts');
    expect(changes.reference(PROJECT)).toBe('HEAD');
    // A file of the worktree in the project's folder: its worktree's base, not the project's.
    expect(await changes.text(changes.originalOf(file('/c:/code/demo/.claude/worktrees/a1/src/app.ts'))!)).toBe('const a = 0;\n');
    expect(core.fsBase).toHaveBeenLastCalledWith('p1', 'a1', 'src/app.ts');
    expect(changes.reference(AGENT)).toBe('main');
  });

  it('is empty for a file new since its base (all its lines added), and none out of a repository', async () => {
    const { changes, base } = setup();
    base(AGENT, 'new.ts', { reference: 'main', text: null });
    const b = changes.originalOf(file('/c:/code/demo/.claude/worktrees/a1/new.ts'))!;
    expect(await changes.text(b)).toBe('');
    expect(changes.isNew(b)).toBe(true);
    // fs_base gives nothing out of a repository: no base, no marks.
    expect(await changes.text(changes.originalOf(file('/c:/code/demo/README.md'))!)).toBeNull();
    expect(changes.reference(PROJECT)).toBeNull();
    // A URI of no source, or of another scheme, is not read.
    expect(await changes.text(uri({ scheme: BASE_SCHEME, authority: '', path: '/c:/elsewhere/a.ts' }))).toBeNull();
    expect(await changes.text(file('/c:/code/demo/a.ts'))).toBeNull();
  });

  it('is told changed when the project changes (a file on disk, a commit, the base moved), its own project only', async () => {
    const { changes, base } = setup();
    base(PROJECT, 'a.ts', { reference: 'HEAD', text: 'x' });
    const b = changes.originalOf(file('/c:/code/demo/a.ts'))!;
    await changes.text(b);
    const told = vi.fn();
    changes.onDidChange(told);
    await changes.projectChanged('p2');
    expect(told).not.toHaveBeenCalled();
    await changes.projectChanged('p1');
    expect(told).toHaveBeenCalledWith(b);
    // Read again, it is the new base.
    base(PROJECT, 'a.ts', { reference: 'HEAD', text: 'y' });
    expect(await changes.text(b)).toBe('y');
  });

  it('is never written: the scheme has no write, and the comparison of a deleted file is with nothing', () => {
    const { changes } = setup();
    // The provider is a text document content provider's: it reads, and has nothing that writes.
    for (const name of ['writeFile', 'write', 'delete', 'rename', 'save'])
      expect((changes as unknown as Record<string, unknown>)[name]).toBeUndefined();
    const f = file('/c:/code/demo/src/app.ts');
    const pair = changes.comparison(f, false)!;
    expect([pair.left.scheme, pair.right.toString()]).toEqual([BASE_SCHEME, 'file:///c:/code/demo/src/app.ts']);
    const gone = changes.comparison(f, true)!;
    expect(gone.right.scheme).toBe(BASE_SCHEME);
    expect(changes.fileOf(gone.right)).toBeNull();
    expect(changes.comparison(file('/c:/elsewhere/a.ts'), false)).toBeNull();
  });

  it('of nothing is empty', async () => {
    const { changes } = setup();
    expect(await changes.text(changes.goneOf(file('/c:/code/demo/gone.ts')))).toBe('');
  });
});

describe('the files changed (the letters of the explorer)', () => {
  const words = (letter: string, reference: string | null) => `${letter} vs ${reference ?? 'la base'}`;

  it('are those of the source among the files the tab « Fichiers » lists', () => {
    const list = [change('a.ts', 'M'), change('b.ts', 'A', 'a1', true), change('c.ts', 'D', 'a2', true), change('d.ts', 'M', 'a1')];
    expect(filesOf(list, PROJECT).map((f) => f.rel)).toEqual(['a.ts', 'd.ts']);
    expect(filesOf(list, AGENT)).toEqual([{ rel: 'b.ts', letter: 'A' }]);
  });

  it('give each file its letter and git’s colour, and mark its folders (propagate)', async () => {
    const { changes, list } = setup();
    list(PROJECT, [change('src/app.ts', 'M'), change('src/new.ts', 'A'), change('old.ts', 'D')]);
    await changes.addSource(PROJECT);
    const d = changes.decoration(file('/c:/code/demo/src/app.ts'), words);
    expect(d).toEqual({ badge: 'M', color: LETTER_COLORS.M, tooltip: 'M vs la base', propagate: false });
    expect(changes.decoration(file('/c:/code/demo/src/new.ts'), words)?.badge).toBe('A');
    expect(changes.decoration(file('/c:/code/demo/old.ts'), words)?.color).toBe('gitDecoration.deletedResourceForeground');
    // The case of the path ignored where the system ignores it.
    expect(changes.decoration(file('/c:/code/demo/SRC/App.ts'), words)?.badge).toBe('M');
    // A folder holding files changed: a dot in the colour of a change, whatever its files; not the root, nor a folder
    // of files unchanged, nor a file unchanged.
    expect(changes.decoration(file('/c:/code/demo/src'), words)).toEqual({
      badge: FOLDER_DOT,
      color: LETTER_COLORS.M,
      tooltip: 'folder vs la base',
      propagate: false,
    });
    expect(changes.decoration(file('/c:/code/demo'), words)).toBeUndefined();
    expect(changes.decoration(file('/c:/code/demo/sr'), words)).toBeUndefined();
    expect(changes.decoration(file('/c:/code/demo/same.ts'), words)).toBeUndefined();
    expect(changes.changed(PROJECT).map((f) => `${f.letter} ${f.rel}`)).toEqual(['D old.ts', 'M src/app.ts', 'A src/new.ts']);
  });

  it('are each source’s own: the worktree’s files are not the project’s', async () => {
    const { changes, list } = setup();
    list(PROJECT, [change('a.ts', 'M'), change('b.ts', 'A', 'a1', true)]);
    list(AGENT, [change('b.ts', 'A', 'a1', true)]);
    await changes.addSource(PROJECT);
    await changes.addSource(AGENT);
    expect(changes.decoration(file('/c:/code/demo/b.ts'), words)).toBeUndefined();
    expect(changes.decoration(file('/c:/code/demo/.claude/worktrees/a1/b.ts'), words)?.badge).toBe('A');
    expect(changes.followed()).toEqual([PROJECT, AGENT]);
  });

  it('are read again when the project changes, the files whose letter changed told', async () => {
    const { changes, list } = setup();
    list(PROJECT, [change('a.ts', 'M'), change('b.ts', 'A')]);
    await changes.addSource(PROJECT);
    const told = vi.fn();
    changes.onDidChangeFiles(told);
    list(PROJECT, [change('a.ts', 'M'), change('c.ts', 'M')]);
    await changes.projectChanged('p1');
    expect(told).toHaveBeenCalledTimes(1);
    expect(told.mock.calls[0][0].map(String).sort()).toEqual(['file:///c:/code/demo/b.ts', 'file:///c:/code/demo/c.ts']);
    // A file deep in folders: its folders told too, their dot changing with it.
    list(PROJECT, [change('a.ts', 'M'), change('c.ts', 'M'), change('src/db/d.ts', 'A')]);
    await changes.projectChanged('p1');
    expect(told.mock.calls[1][0].map(String)).toEqual([
      'file:///c:/code/demo/src/db/d.ts',
      'file:///c:/code/demo/src/db',
      'file:///c:/code/demo/src',
    ]);
    expect(changes.decoration(file('/c:/code/demo/b.ts'), words)).toBeUndefined();
    // Nothing changed: nothing told.
    await changes.projectChanged('p1');
    expect(told).toHaveBeenCalledTimes(2);
  });

  it('name their base in their tooltip once a base was read', async () => {
    const { changes, list, base } = setup();
    list(AGENT, [change('a.ts', 'M', 'a1', true)]);
    base(AGENT, 'a.ts', { reference: 'main', text: '' });
    await changes.addSource(AGENT);
    await changes.text(changes.originalOf(file('/c:/code/demo/.claude/worktrees/a1/a.ts'))!);
    expect(changes.decoration(file('/c:/code/demo/.claude/worktrees/a1/a.ts'), words)?.tooltip).toBe('M vs main');
  });

  it('of a project closed are forgotten', async () => {
    const { changes, list } = setup();
    list(PROJECT, [change('a.ts', 'M')]);
    await changes.addSource(PROJECT);
    changes.removeProject('p1');
    expect(changes.decoration(file('/c:/code/demo/a.ts'), words)).toBeUndefined();
    expect(changes.followed()).toEqual([]);
  });

  it('keep the last list when the core fails to read', async () => {
    const { changes, list, core } = setup();
    list(PROJECT, [change('a.ts', 'M')]);
    await changes.addSource(PROJECT);
    vi.mocked(core.gitFiles).mockRejectedValueOnce('git');
    await changes.projectChanged('p1');
    expect(changes.decoration(file('/c:/code/demo/a.ts'), words)?.badge).toBe('M');
  });
});

describe('the lines changed', () => {
  it('are the lines of the modified side of VS Code’s changes, a deletion counting none', () => {
    const r = (s: number, e: number) => ({ modified: { startLineNumber: s, endLineNumberExclusive: e } });
    expect(changedLines([])).toBe(0);
    expect(changedLines([r(3, 4)])).toBe(1);
    expect(changedLines([r(1, 6), r(10, 10), r(12, 14)])).toBe(7);
  });
});
