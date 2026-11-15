import { describe, expect, it, vi } from 'vitest';
import type { FileStamp, FileStat } from '../types';
import { EscouadeFileSystem, FileChangeType, FileType, type CoreFiles, type ErrorCode, type UriLike } from './fs-provider';
import type { FileUri } from './tree';

/** VS Code's error, as the overlay tells it: its code. */
class FsError extends Error {
  constructor(
    message: string,
    readonly code: ErrorCode,
  ) {
    super(message);
  }
}

const kit = {
  error: (message: string, code: ErrorCode) => new FsError(message, code),
  uri: (u: FileUri): UriLike => ({ scheme: 'file', ...u }),
};
const file = (path: string, authority = ''): UriLike => ({ scheme: 'file', authority, path });

interface Entry {
  bytes: Uint8Array;
  mtime: number;
}

/**
 * The core, as far as the provider sees it: the sources' folders (by their absolute path), each file's bytes and time,
 * and the commands of the editor over them, refused as the core refuses (`NOT_FOUND:<path>`, `changed`).
 */
function fakeCore(roots: Record<string, string>) {
  const files = new Map<string, Entry>();
  let clock = 1000;
  const rootOf = (projectId: string, agentId: string | null) => roots[`${projectId}/${agentId ?? 'project'}`];
  const abs = (projectId: string, agentId: string | null, rel: string) => {
    if (rel.split('/').includes('..')) throw new Error(`hors du dossier : ${rel}`);
    return `${rootOf(projectId, agentId)}/${rel}`.replace(/\/$/, '');
  };
  const isDir = (p: string) => [...files.keys()].some((f) => f.startsWith(p + '/')) || Object.values(roots).includes(p);
  const stampOf = (e: Entry): FileStamp => ({ mtime: e.mtime, size: e.bytes.length });
  const core: CoreFiles = {
    fsTree: vi.fn(async (projectId: string, agentId: string | null) => {
      const root = rootOf(projectId, agentId);
      // The project's tree leaves the agents' worktrees out, as git lists it.
      const others = Object.values(roots).filter((r) => r !== root && r.startsWith(root + '/'));
      return {
        root: root.replace(/^\/([a-z]):/, (_, d: string) => `${d.toUpperCase()}:`).replace(/\//g, root.startsWith('/c:') ? '\\' : '/'),
        files: [...files.keys()]
          .filter((f) => f.startsWith(root + '/') && !others.some((o) => f.startsWith(o + '/')))
          .map((f) => f.slice(root.length + 1)),
        truncated: false,
        ignored: [],
      };
    }),
    fsStat: vi.fn(async (projectId: string, agentId: string | null, rel: string): Promise<FileStat> => {
      const p = abs(projectId, agentId, rel);
      const e = files.get(p);
      if (e) return { kind: 'file', ...stampOf(e), ctime: e.mtime, readonly: false };
      if (isDir(p)) return { kind: 'dir', size: 0, mtime: 1, ctime: 1, readonly: false };
      throw `NOT_FOUND:${rel}`;
    }),
    fsReadBytes: vi.fn(async (projectId: string, agentId: string | null, rel: string) => {
      const e = files.get(abs(projectId, agentId, rel));
      if (!e) throw `NOT_FOUND:${rel}`;
      return e.bytes.slice().buffer;
    }),
    fsWriteBytes: vi.fn(async (projectId: string, agentId: string | null, rel: string, bytes: Uint8Array, expect) => {
      const p = abs(projectId, agentId, rel);
      const e = files.get(p);
      if (expect === 'absent' && e) throw `${rel} existe déjà`;
      if (typeof expect === 'object') {
        if (!e) throw 'deleted';
        if (e.mtime !== expect.mtime || e.bytes.length !== expect.size) throw 'changed';
      }
      const next = { bytes: bytes.slice(), mtime: ++clock };
      files.set(p, next);
      return stampOf(next);
    }),
    fsMkdir: vi.fn(async () => {}),
    fsRename: vi.fn(async (projectId: string, agentId: string | null, from: string, to: string) => {
      const e = files.get(abs(projectId, agentId, from));
      if (!e) throw `NOT_FOUND:${from}`;
      files.delete(abs(projectId, agentId, from));
      files.set(abs(projectId, agentId, to), e);
    }),
    fsDelete: vi.fn(async (projectId: string, agentId: string | null, rel: string) => {
      files.delete(abs(projectId, agentId, rel));
    }),
  };
  return {
    core,
    /** A file written on disk by someone else (an agent). */
    put(path: string, text: string | Uint8Array) {
      files.set(path, { bytes: typeof text === 'string' ? new TextEncoder().encode(text) : text, mtime: ++clock });
    },
    remove(path: string) {
      files.delete(path);
    },
    text: (path: string) => new TextDecoder().decode(files.get(path)?.bytes),
    calls: () => Object.values(core).reduce((n, f) => n + (f as ReturnType<typeof vi.fn>).mock.calls.length, 0),
  };
}

const ROOTS = { 'p1/project': '/c:/code/demo', 'p1/a1': '/c:/code/demo/.claude/worktrees/a1', 'p2/project': '/c:/code/other' };

async function setUp() {
  const disk = fakeCore(ROOTS);
  disk.put('/c:/code/demo/src/app.ts', 'export const a = 1;\r\n');
  disk.put('/c:/code/demo/logo.png', new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0, 0, 0xff]));
  disk.put('/c:/code/demo/.claude/worktrees/a1/src/app.ts', 'export const a = 2;\n');
  disk.put('/c:/code/other/secret.txt', 'secret');
  const fs = new EscouadeFileSystem(disk.core, kit, true);
  const project = await fs.addSource({ projectId: 'p1', agentId: null });
  return { disk, fs, project };
}

const code = (p: Promise<unknown>) =>
  p.then(
    () => 'ok',
    (e: FsError) => e.code,
  );

describe('the file system provider of the VS Code view', () => {
  it('serves the folder the core gives for a source, as VS Code writes a file URI', async () => {
    const { fs, project } = await setUp();
    expect(project).toEqual(file('/c:/code/demo'));
    expect(await fs.readdir(file('/c:/code/demo'))).toEqual([
      ['src', FileType.Directory],
      ['logo.png', FileType.File],
    ]);
    // The project's tree leaves the worktree out: the agent's files are its source's.
    expect(await fs.readdir(file('/c:/code/demo/src'))).toEqual([['app.ts', FileType.File]]);
  });

  it('reads any file in bytes, a binary one as it is, and a text with its CRLF', async () => {
    const { fs, disk } = await setUp();
    expect([...(await fs.readFile(file('/c:/code/demo/logo.png')))]).toEqual([0x89, 0x50, 0x4e, 0x47, 0, 0, 0xff]);
    expect(new TextDecoder().decode(await fs.readFile(file('/c:/code/demo/src/app.ts')))).toBe('export const a = 1;\r\n');
    expect(disk.core.fsReadBytes).toHaveBeenLastCalledWith('p1', null, 'src/app.ts');
    // The case of the root does not matter on Windows and macOS (the disk ignores it below).
    expect(fs.sourceOf(file('/C:/Code/Demo/SRC/app.ts'))).toEqual({ source: { projectId: 'p1', agentId: null }, rel: 'SRC/app.ts' });
    expect(await code(fs.readFile(file('/c:/code/demo/missing.ts')))).toBe('EntryNotFound');
    const st = await fs.stat(file('/c:/code/demo/src/app.ts'));
    expect(st).toMatchObject({ type: FileType.File, size: 21 });
    expect(st.mtime).toBeGreaterThan(0);
  });

  it('never asks the core for a URI out of its sources, nor one that climbs out of them', async () => {
    const { fs, disk } = await setUp();
    const before = disk.calls();
    for (const uri of [
      file('/c:/code/other/secret.txt'),
      file('/c:/code/demo/../other/secret.txt'),
      file('/c:/code/demo/src/./../../other/secret.txt'),
      file('/c:/code/demo-other/a.ts'),
      file('/c:/code/demo/a.ts', 'wsl.localhost'),
      { scheme: 'vscode-userdata', authority: '', path: '/c:/code/demo/src/app.ts' },
    ]) {
      expect(await code(fs.readFile(uri))).toBe('EntryNotFound');
      expect(await code(fs.stat(uri))).toBe('EntryNotFound');
      expect(await code(fs.writeFile(uri, new Uint8Array([1]), { create: true, overwrite: true }))).toBe('EntryNotFound');
      expect(await code(fs.delete(uri))).toBe('EntryNotFound');
    }
    expect(disk.calls()).toBe(before);
    expect(disk.text('/c:/code/other/secret.txt')).toBe('secret');
  });

  it('serves an agent’s worktree as its own source, even inside the project’s folder', async () => {
    const { fs, disk } = await setUp();
    const wt = await fs.addSource({ projectId: 'p1', agentId: 'a1' });
    expect(wt).toEqual(file('/c:/code/demo/.claude/worktrees/a1'));
    const uri = file('/c:/code/demo/.claude/worktrees/a1/src/app.ts');
    expect(fs.sourceOf(uri)).toEqual({ source: { projectId: 'p1', agentId: 'a1' }, rel: 'src/app.ts' });
    expect(new TextDecoder().decode(await fs.readFile(uri))).toBe('export const a = 2;\n');
    expect(disk.core.fsReadBytes).toHaveBeenLastCalledWith('p1', 'a1', 'src/app.ts');
    await fs.stat(uri);
    await fs.writeFile(uri, new TextEncoder().encode('export const a = 3;\n'), { create: true, overwrite: true });
    expect(disk.core.fsWriteBytes).toHaveBeenLastCalledWith('p1', 'a1', 'src/app.ts', expect.anything(), expect.any(Object));
    expect(disk.text('/c:/code/demo/.claude/worktrees/a1/src/app.ts')).toBe('export const a = 3;\n');
    expect(disk.text('/c:/code/demo/src/app.ts')).toBe('export const a = 1;\r\n');
    expect(fs.uriOf({ projectId: 'p1', agentId: 'a1' }, 'src/app.ts')).toEqual(uri);
  });

  it('writes over the version VS Code checked, and refuses the write when the file changed in between', async () => {
    const { fs, disk } = await setUp();
    const uri = file('/c:/code/demo/src/app.ts');
    // VS Code stats the file just before writing it (its own check against the version it read).
    const seen = await fs.stat(uri);
    await fs.writeFile(uri, new TextEncoder().encode('mine\r\n'), { create: true, overwrite: true });
    expect(disk.core.fsWriteBytes).toHaveBeenLastCalledWith('p1', null, 'src/app.ts', expect.anything(), {
      mtime: seen.mtime,
      size: seen.size,
    });
    expect(disk.text('/c:/code/demo/src/app.ts')).toBe('mine\r\n');
    // An agent writes the file between VS Code's check and the write: refused, the agent's text kept, and never an
    // error that would let the overlay write it in memory instead.
    await fs.stat(uri);
    disk.put('/c:/code/demo/src/app.ts', 'the agent’s');
    const refused = await fs.writeFile(uri, new TextEncoder().encode('mine again'), { create: true, overwrite: true }).catch((e) => e);
    expect(refused).toBeInstanceOf(FsError);
    expect(refused).toMatchObject({ code: 'Unknown', message: 'changed' });
    expect(disk.text('/c:/code/demo/src/app.ts')).toBe('the agent’s');
    // VS Code's « Overwrite »: stat'd again, then written.
    await fs.stat(uri);
    await fs.writeFile(uri, new TextEncoder().encode('mine again'), { create: true, overwrite: true });
    expect(disk.text('/c:/code/demo/src/app.ts')).toBe('mine again');
  });

  it('creates a file as new, refuses one that exists without overwrite, and keeps the binary bytes it is given', async () => {
    const { fs, disk } = await setUp();
    const uri = file('/c:/code/demo/assets/blob.bin');
    await fs.writeFile(uri, new Uint8Array([0, 1, 2, 0xff]), { create: true, overwrite: false });
    expect(disk.core.fsWriteBytes).toHaveBeenLastCalledWith('p1', null, 'assets/blob.bin', expect.anything(), 'absent');
    expect([...(await fs.readFile(uri))]).toEqual([0, 1, 2, 0xff]);
    expect(await code(fs.writeFile(uri, new Uint8Array([9]), { create: true, overwrite: false }))).toBe('EntryExists');
    // Not to be created: refused, but never as « not found » (the overlay would write it in memory).
    expect(await code(fs.writeFile(file('/c:/code/demo/nope.ts'), new Uint8Array([1]), { create: false, overwrite: true }))).toBe(
      'Unknown',
    );
    // The tree is read again: the new folder and file are listed.
    expect(await fs.readdir(file('/c:/code/demo/assets'))).toEqual([['blob.bin', FileType.File]]);
    expect(await code(fs.writeFile(file('/c:/code/demo/src'), new Uint8Array([1]), { create: true, overwrite: true }))).toBe(
      'EntryIsADirectory',
    );
  });

  it('renames and deletes through the core, in one source only', async () => {
    const { fs, disk } = await setUp();
    await fs.addSource({ projectId: 'p1', agentId: 'a1' });
    await fs.rename(file('/c:/code/demo/src/app.ts'), file('/c:/code/demo/src/main.ts'));
    expect(disk.core.fsRename).toHaveBeenCalledWith('p1', null, 'src/app.ts', 'src/main.ts');
    expect(await code(fs.rename(file('/c:/code/demo/src/main.ts'), file('/c:/code/demo/.claude/worktrees/a1/main.ts')))).toBe('Unknown');
    await fs.delete(file('/c:/code/demo/src/main.ts'));
    expect(disk.core.fsDelete).toHaveBeenCalledWith('p1', null, 'src/main.ts');
    expect(await fs.readdir(file('/c:/code/demo'))).toEqual([['logo.png', FileType.File]]);
  });

  it('fires the changes the core’s watcher tells: a known file at once, then what the tree added or removed', async () => {
    const { fs, disk } = await setUp();
    await fs.addSource({ projectId: 'p1', agentId: 'a1' });
    const fired: { type: number; resource: UriLike }[][] = [];
    // Each change is told in both spellings of the drive (see below): the lower case one looked at here.
    const sub = fs.onDidChangeFile((c) => fired.push(c.filter((x) => x.resource.path.startsWith('/c:'))));
    await fs.readdir(file('/c:/code/demo'));
    // An agent changes a file, writes a new one, removes another; in the worktree and in the project.
    disk.put('/c:/code/demo/.claude/worktrees/a1/src/app.ts', 'export const a = 42;\n');
    disk.put('/c:/code/demo/src/new.ts', 'new');
    disk.remove('/c:/code/demo/logo.png');
    // The trees take their time to be read again (a busy worktree): the file known is told before.
    let go = () => {};
    const slow = { promise: new Promise<void>((r) => (go = r)), resolve: () => go() };
    const tree = disk.core.fsTree;
    disk.core.fsTree = vi.fn(async (projectId: string, agentId: string | null) => {
      await slow.promise;
      return tree(projectId, agentId);
    });
    const done = fs.diskChanged({
      projectId: 'p1',
      paths: [
        'C:\\code\\demo\\.claude\\worktrees\\a1\\src\\app.ts',
        'C:\\code\\demo\\src\\new.ts',
        'C:\\code\\demo\\logo.png',
        // Out of the sources served: nothing.
        'C:\\code\\elsewhere\\x.ts',
      ],
    });
    // At once: the files the trees know, in the source each is in.
    expect(fired[0]).toEqual([
      { type: FileChangeType.UPDATED, resource: file('/c:/code/demo/.claude/worktrees/a1/src/app.ts') },
      { type: FileChangeType.UPDATED, resource: file('/c:/code/demo/logo.png') },
    ]);
    slow.resolve();
    await done;
    expect(fired).toHaveLength(2);
    expect(fired[1]).toEqual(
      expect.arrayContaining([
        { type: FileChangeType.ADDED, resource: file('/c:/code/demo/src/new.ts') },
        { type: FileChangeType.DELETED, resource: file('/c:/code/demo/logo.png') },
      ]),
    );
    expect(fired[1]).toHaveLength(2);
    disk.core.fsTree = tree;
    expect(await fs.readdir(file('/c:/code/demo/src'))).toEqual([
      ['app.ts', FileType.File],
      ['new.ts', FileType.File],
    ]);
    // Another project's changes are not this source's.
    await fs.diskChanged({ projectId: 'p2', paths: ['C:\\code\\other\\secret.txt'] });
    expect(fired).toHaveLength(2);
    // Too many to tell: the files VS Code read are told updated.
    await fs.readFile(file('/c:/code/demo/src/app.ts'));
    await fs.diskChanged({ projectId: 'p1', paths: null });
    expect(fired).toHaveLength(3);
    expect(fired[2]).toEqual([{ type: FileChangeType.UPDATED, resource: file('/c:/code/demo/src/app.ts') }]);
    sub.dispose();
    await fs.diskChanged({ projectId: 'p1', paths: ['C:\\code\\demo\\src\\app.ts'] });
    expect(fired).toHaveLength(3);
  });

  it('tells a change of a path with a drive in both its spellings, VS Code keeping some of its own in capitals', async () => {
    const { fs, disk } = await setUp();
    const fired: { type: number; resource: UriLike }[][] = [];
    fs.onDidChangeFile((c) => fired.push([...c]));
    await fs.readFile(file('/c:/code/demo/src/app.ts'));
    disk.put('/c:/code/demo/src/app.ts', 'export const a = 7;\n');
    await fs.diskChanged({ projectId: 'p1', paths: ['C:\\code\\demo\\src\\app.ts'] });
    expect(fired[0]).toEqual([
      { type: FileChangeType.UPDATED, resource: file('/c:/code/demo/src/app.ts') },
      { type: FileChangeType.UPDATED, resource: file('/C:/code/demo/src/app.ts') },
    ]);
  });

  it('keeps the case of paths on Linux', async () => {
    const disk = fakeCore({ 'p1/project': '/home/me/demo' });
    disk.put('/home/me/demo/README.md', '# demo');
    const fs = new EscouadeFileSystem(disk.core, kit, false);
    expect(await fs.addSource({ projectId: 'p1', agentId: null })).toEqual(file('/home/me/demo'));
    expect(await code(fs.stat(file('/home/me/Demo/README.md')))).toBe('EntryNotFound');
    expect((await fs.stat(file('/home/me/demo/README.md'))).type).toBe(FileType.File);
    expect(fs.capabilities & 1024).toBe(1024);
  });
});
