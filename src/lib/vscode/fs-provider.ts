// The files of the VS Code view, by the core (ESC-447): a file system provider of VS Code for the `file:` URIs of the
// sources the view shows (a project's checkout, an agent's worktree), put in front of the in-memory one where VS Code
// keeps its workspace file. It asks the core only, through the editor's commands (`CoreFiles`): the page never reads
// the disk, and never out of a source's folder (a URI out of every root is not this provider's; the core refuses a
// path out of its root anyway).
//
// - Files are bytes both ways (`fs_read_bytes`, `fs_write_bytes`): text or not, VS Code keeps their encoding, BOM and
//   line endings itself, and shows a binary file as it does.
// - `stat` gives the file's real time and size (`fs_stat`): VS Code compares them with those of the version it read
//   to tell a file changed on disk since, and then shows its conflict (« The content of the file is newer »: Compare,
//   Overwrite). A write still names the stamp VS Code saw just before (`fs_write_bytes` refuses with `changed` when
//   the file changed in between).
// - The changes on disk the core's watcher tells (`diskChanged`) are fired as VS Code's file changes: a file an agent
//   wrote is read again when it is not modified in the view, the tree follows what was added or removed.
//
// Pure of monaco: what it needs of VS Code (its errors, its URIs) is given (`VsKit`), so that the tests run it alone.

import type { DiskChange } from '../disk-changes';
import type { FileStamp, FileStat, FileTree } from '../types';
import { dirsOf, entriesOf, FILE, fileUriOf, relativeTo, typeOf, type Dirs, type FileUri } from './tree';

/** What the provider asks the core: the editor's commands, nothing else (`api` in the app, a fake in the tests). */
export interface CoreFiles {
  fsTree(projectId: string, agentId: string | null): Promise<FileTree>;
  fsStat(projectId: string, agentId: string | null, path: string): Promise<FileStat>;
  fsReadBytes(projectId: string, agentId: string | null, path: string): Promise<ArrayBuffer>;
  fsWriteBytes(
    projectId: string,
    agentId: string | null,
    path: string,
    bytes: Uint8Array,
    expect: 'absent' | 'any' | FileStamp,
  ): Promise<FileStamp>;
  fsMkdir(projectId: string, agentId: string | null, path: string): Promise<void>;
  fsRename(projectId: string, agentId: string | null, from: string, to: string): Promise<void>;
  fsDelete(projectId: string, agentId: string | null, path: string): Promise<void>;
}

/** The codes of VS Code's `FileSystemProviderErrorCode` this provider uses. */
export type ErrorCode = 'EntryNotFound' | 'EntryExists' | 'EntryIsADirectory' | 'EntryTooLarge' | 'Unknown';

/** The part of a VS Code URI the provider reads. */
export interface UriLike {
  readonly scheme: string;
  readonly authority: string;
  readonly path: string;
}

/** What the provider needs of VS Code, given by the workbench: its errors (the overlay tells them by their class), its URIs. */
export interface VsKit<U extends UriLike> {
  error(message: string, code: ErrorCode): Error;
  uri(u: FileUri): U;
}

// VS Code's values (vs/platform/files/common/files).
export const FileType = { File: 1, Directory: 2 } as const;
export const FileChangeType = { UPDATED: 0, ADDED: 1, DELETED: 2 } as const;
const FILE_READ_WRITE = 2;
const PATH_CASE_SENSITIVE = 1024;
const PERMISSION_READONLY = 1;

export interface Stat {
  type: number;
  ctime: number;
  mtime: number;
  size: number;
  permissions?: number;
}

export interface FileChange<U> {
  type: number;
  resource: U;
}

interface Disposable {
  dispose(): void;
}

/** A source shown: the project's checkout (`agentId` null) or an agent's worktree. */
export interface Source {
  projectId: string;
  agentId: string | null;
}

interface Root extends Source {
  /** The URI of its folder. */
  uri: FileUri;
  /** Its folders as the tree lists them, read when first asked and again after a change on disk. */
  dirs: Promise<Dirs> | null;
  /** The last listing read, while `dirs` is read again. */
  listed: Dirs | null;
}

/** An error of the core's, as it comes (a string, mostly). */
const text = (e: unknown) => (e instanceof Error ? e.message : String(e));
const isNotFound = (e: unknown) => text(e).startsWith('NOT_FOUND');

export class EscouadeFileSystem<U extends UriLike> {
  readonly capabilities: number;
  private readonly listeners = new Set<(changes: readonly FileChange<U>[]) => void>();
  private readonly roots: Root[] = [];
  /** The stamp of each file VS Code stat'd last, by `key`: what a write expects to find. */
  private readonly stamps = new Map<string, FileStamp>();
  /** The files VS Code read, by `key`: those a change told without paths may concern. */
  private readonly read = new Map<string, { root: Root; rel: string }>();

  constructor(
    private readonly core: CoreFiles,
    private readonly kit: VsKit<U>,
    /** Paths compared without their case (Windows, macOS). */
    private readonly fold: boolean,
  ) {
    this.capabilities = FILE_READ_WRITE | (fold ? 0 : PATH_CASE_SENSITIVE);
  }

  /** VS Code's `Event`: a listener, called with each batch of changes, until disposed. */
  readonly onDidChangeFile = (listener: (changes: readonly FileChange<U>[]) => void, thisArgs?: unknown, disposables?: Disposable[]) => {
    const f = thisArgs ? listener.bind(thisArgs) : listener;
    this.listeners.add(f);
    const d = { dispose: () => this.listeners.delete(f) };
    disposables?.push(d);
    return d;
  };
  readonly onDidChangeCapabilities = () => ({ dispose() {} });

  private fire(changes: FileChange<U>[]) {
    if (changes.length) for (const f of this.listeners) f(changes);
  }

  /** Serves the source's folder; its URI. */
  async addSource(src: Source): Promise<U> {
    let root = this.roots.find((r) => r.projectId === src.projectId && r.agentId === src.agentId);
    if (!root) {
      const tree = await this.core.fsTree(src.projectId, src.agentId);
      const uri = fileUriOf(tree.root);
      if (!uri) throw new Error(`not an absolute path: ${tree.root}`);
      // Asked twice at once: the first one stays.
      root = this.roots.find((r) => r.projectId === src.projectId && r.agentId === src.agentId);
      if (!root) {
        const listed = dirsOf(tree.files);
        root = { ...src, uri, dirs: Promise.resolve(listed), listed };
        this.roots.push(root);
      }
    }
    return this.kit.uri(root.uri);
  }

  /** The URI of the file `rel` of a source served, or null. */
  uriOf(src: Source, rel: string): U | null {
    const root = this.roots.find((r) => r.projectId === src.projectId && r.agentId === src.agentId);
    return root ? this.kit.uri({ authority: root.uri.authority, path: join(root.uri.path, rel) }) : null;
  }

  /** The source a URI is in, and its path there: the deepest root holding it (a worktree in the project's folder). */
  sourceOf(uri: UriLike): { source: Source; rel: string } | null {
    const at = this.find(uri);
    return at ? { source: { projectId: at.root.projectId, agentId: at.root.agentId }, rel: at.rel } : null;
  }

  private find(uri: UriLike, among: readonly Root[] = this.roots): { root: Root; rel: string } | null {
    if (uri.scheme !== 'file') return null;
    return this.findPath({ authority: uri.authority, path: uri.path }, among);
  }

  private findPath(uri: FileUri, among: readonly Root[]): { root: Root; rel: string } | null {
    let best: { root: Root; rel: string } | null = null;
    for (const root of among) {
      if (root.uri.authority.toLowerCase() !== uri.authority.toLowerCase()) continue;
      const rel = relativeTo(root.uri.path, uri.path, this.fold);
      if (rel !== null && (!best || root.uri.path.length > best.root.uri.path.length)) best = { root, rel };
    }
    return best;
  }

  private key(root: Root, rel: string) {
    const k = `${root.projectId}\n${root.agentId ?? ''}\n${rel}`;
    return this.fold ? k.toLowerCase() : k;
  }

  private dirs(root: Root): Promise<Dirs> {
    root.dirs ??= this.core.fsTree(root.projectId, root.agentId).then(
      (t) => (root.listed = dirsOf(t.files)),
      (e) => {
        root.dirs = null;
        throw e;
      },
    );
    return root.dirs;
  }

  private notFound(): Error {
    return this.kit.error('not found', 'EntryNotFound');
  }

  /** An error of a read: a missing file is VS Code's « not found », the next provider asked. */
  private readError(e: unknown): Error {
    if (isNotFound(e)) return this.notFound();
    const msg = text(e);
    if (msg.startsWith('TOO_LARGE')) return this.kit.error(msg, 'EntryTooLarge');
    return this.kit.error(msg, 'Unknown');
  }

  /**
   * An error of a write in a source: never « not found » (nor « unavailable »), which would let the overlay write it
   * in the memory behind, as if it were saved.
   */
  private writeError(e: unknown): Error {
    return this.kit.error(text(e), 'Unknown');
  }

  watch(): Disposable {
    // The core's watcher tells the changes of the whole project (`diskChanged`).
    return { dispose() {} };
  }

  async stat(uri: U): Promise<Stat> {
    const at = this.find(uri);
    if (!at) throw this.notFound();
    let st: FileStat;
    try {
      st = await this.core.fsStat(at.root.projectId, at.root.agentId, at.rel);
    } catch (e) {
      throw this.readError(e);
    }
    if (st.kind === 'file') this.stamps.set(this.key(at.root, at.rel), { mtime: st.mtime, size: st.size });
    return {
      type: st.kind === 'dir' ? FileType.Directory : FileType.File,
      ctime: st.ctime,
      mtime: st.mtime,
      size: st.size,
      permissions: st.readonly ? PERMISSION_READONLY : undefined,
    };
  }

  async readdir(uri: U): Promise<[string, number][]> {
    const at = this.find(uri);
    if (!at) throw this.notFound();
    let dirs: Dirs;
    try {
      dirs = await this.dirs(at.root);
    } catch (e) {
      throw this.readError(e);
    }
    const entries = dirs.get(at.rel);
    if (entries) return [...entries].map(([name, k]) => [name, k === FILE ? FileType.File : FileType.Directory]);
    // A folder the tree does not list (ignored, or empty): there, but nothing shown in it.
    if ((await this.stat(uri)).type === FileType.Directory) return [];
    throw this.kit.error('not a folder', 'Unknown');
  }

  async readFile(uri: U): Promise<Uint8Array> {
    const at = this.find(uri);
    if (!at) throw this.notFound();
    let bytes: ArrayBuffer;
    try {
      bytes = await this.core.fsReadBytes(at.root.projectId, at.root.agentId, at.rel);
    } catch (e) {
      throw this.readError(e);
    }
    this.read.set(this.key(at.root, at.rel), at);
    return new Uint8Array(bytes);
  }

  async writeFile(uri: U, content: Uint8Array, opts: { create: boolean; overwrite: boolean }): Promise<void> {
    const at = this.find(uri);
    // Out of every source: not this provider's (the workspace file, in memory).
    if (!at) throw this.notFound();
    if (at.rel === '') throw this.kit.error('is a folder', 'EntryIsADirectory');
    const { root, rel } = at;
    let now: FileStat | null = null;
    try {
      now = await this.core.fsStat(root.projectId, root.agentId, rel);
    } catch (e) {
      if (!isNotFound(e)) throw this.writeError(e);
    }
    if (now?.kind === 'dir') throw this.kit.error('is a folder', 'EntryIsADirectory');
    if (!now && !opts.create) throw this.writeError(`${rel}: not found`);
    if (now && !opts.overwrite) throw this.kit.error('exists', 'EntryExists');
    const k = this.key(root, rel);
    // The stamp VS Code saw when it checked, just before, that this is still the file it read.
    const expect = now ? (this.stamps.get(k) ?? { mtime: now.mtime, size: now.size }) : 'absent';
    try {
      this.stamps.set(k, await this.core.fsWriteBytes(root.projectId, root.agentId, rel, content, expect));
    } catch (e) {
      this.stamps.delete(k);
      throw this.writeError(e);
    }
    if (!now) root.dirs = null;
  }

  async mkdir(uri: U): Promise<void> {
    const at = this.find(uri);
    if (!at) throw this.notFound();
    try {
      await this.core.fsMkdir(at.root.projectId, at.root.agentId, at.rel);
    } catch (e) {
      throw this.writeError(e);
    }
    at.root.dirs = null;
  }

  async delete(uri: U): Promise<void> {
    const at = this.find(uri);
    if (!at) throw this.notFound();
    try {
      // To the system's trash, always.
      await this.core.fsDelete(at.root.projectId, at.root.agentId, at.rel);
    } catch (e) {
      throw this.writeError(e);
    }
    this.forget(at.root, at.rel);
    at.root.dirs = null;
  }

  async rename(from: U, to: U): Promise<void> {
    const a = this.find(from);
    const b = this.find(to);
    if (!a) throw this.notFound();
    if (!b || a.root !== b.root) throw this.writeError('not in the same source');
    try {
      await this.core.fsRename(a.root.projectId, a.root.agentId, a.rel, b.rel);
    } catch (e) {
      throw this.writeError(e);
    }
    this.forget(a.root, a.rel);
    a.root.dirs = null;
  }

  private forget(root: Root, rel: string) {
    const k = this.key(root, rel);
    for (const m of [this.stamps, this.read]) for (const key of [...m.keys()]) if (key === k || key.startsWith(k + '/')) m.delete(key);
  }

  /**
   * The change of `rel` as VS Code's events say it. Where the case is ignored, a file updated with a drive is told in
   * both of its spellings (`/c:/…`, `/C:/…`): VS Code finds the models to read again by their path, case included, and
   * some of its own (made by `URI.file` from a path of the system) keep the drive in capitals. A file added or removed
   * is told once, as the explorer knows it (the other spelling would be a stranger to its tree).
   */
  private changesOf(root: Root, rel: string, type: number): FileChange<U>[] {
    const path = join(root.uri.path, rel);
    const paths = [path];
    const drive = /^\/([a-zA-Z]):/.exec(path);
    if (this.fold && drive && type === FileChangeType.UPDATED) {
      const flip = drive[1] === drive[1].toLowerCase() ? drive[1].toUpperCase() : drive[1].toLowerCase();
      paths.push(`/${flip}${path.slice(2)}`);
    }
    return paths.map((p) => ({ type, resource: this.kit.uri({ authority: root.uri.authority, path: p }) }));
  }

  /**
   * What the core's watcher told of a project, fired as VS Code's file changes, so that a file open and not modified
   * is read again at once: first the files known (in the tree, or read by VS Code) told updated, without waiting; then,
   * once the source's tree is read again (it can take seconds in a busy worktree), what it added or removed. Nothing for
   * a path the source does not serve. Without paths (too many), the files VS Code read are the ones told updated.
   */
  async diskChanged(c: DiskChange): Promise<void> {
    const roots = this.roots.filter((r) => r.projectId === c.projectId);
    if (!roots.length) return;
    const touched = new Map<Root, Set<string> | null>();
    if (c.paths === null) for (const r of roots) touched.set(r, null);
    else
      for (const p of c.paths) {
        const u = fileUriOf(p);
        const at = u && this.findPath(u, roots);
        if (!at || at.rel === '') continue;
        const set = touched.get(at.root) ?? new Set<string>();
        set.add(at.rel);
        touched.set(at.root, set);
      }
    const now: FileChange<U>[] = [];
    const sent = new Map<Root, Set<string>>();
    const work: { root: Root; rels: Set<string>; before: Dirs | null }[] = [];
    for (const [root, given] of touched) {
      const read = new Set([...this.read.values()].filter((r) => r.root === root).map((r) => r.rel));
      const rels = given ?? read;
      const before = root.listed;
      const known = new Set([...rels].filter((rel) => read.has(rel) || (before && typeOf(before, rel) === FILE)));
      for (const rel of known) now.push(...this.changesOf(root, rel, FileChangeType.UPDATED));
      sent.set(root, known);
      root.dirs = null;
      work.push({ root, rels, before });
    }
    this.fire(now);
    const later: FileChange<U>[] = [];
    for (const { root, rels, before } of work) {
      const after = before ? await this.dirs(root).catch(() => null) : null;
      const kinds = new Map<string, number>();
      if (before && after) {
        const was = entriesOf(before);
        const is = entriesOf(after);
        for (const p of is.keys()) if (!was.has(p)) kinds.set(p, FileChangeType.ADDED);
        for (const p of was.keys()) if (!is.has(p)) kinds.set(p, FileChangeType.DELETED);
      }
      // A file the tree now lists, not told yet. A path the source does not serve (an agent's worktree in the project's
      // folder, a temporary file, git's own) is told nothing: VS Code reads it fresh if it ever opens it.
      for (const rel of rels)
        if (!kinds.has(rel) && !sent.get(root)!.has(rel) && after && typeOf(after, rel) === FILE) kinds.set(rel, FileChangeType.UPDATED);
      for (const [rel, type] of kinds) {
        // A file removed is stat'd anew if it comes back.
        if (type === FileChangeType.DELETED) this.forget(root, rel);
        later.push(...this.changesOf(root, rel, type));
      }
    }
    this.fire(later);
  }
}

function join(root: string, rel: string) {
  if (!rel) return root;
  return root.endsWith('/') ? root + rel : `${root}/${rel}`;
}
