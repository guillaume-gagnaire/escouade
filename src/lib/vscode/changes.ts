// The changes from the base in the VS Code view (ESC-476, A7): the base version of each file of a source served, as the
// core reads it (`fs_base`: HEAD for the project's checkout, where its branch left its base for an agent's worktree, the
// old editor's base), under a scheme of its own (`escouade-base:`) that nothing writes to; and the files changed, as the
// core lists them (`git_files`, the tab « Fichiers »'s), as the letters and colours of the explorer.
//
// - The base is the original resource of VS Code's quick diff (`originalOf`): its marks between the numbers and the text,
//   the peek of a change, and its diff editor (`vscode.diff` of the base and the file).
// - It is told changed (`onDidChange`) when the core's watcher or its git loop tells something of the project: VS Code
//   then reads the base again and draws its marks anew, as it does when the file changes.
// - The page never runs git: both lists come from the core's commands the tab « Fichiers » already uses.
//
// Pure of monaco: what it needs of VS Code (its URIs) is given (`ChangesKit`), so that the tests run it alone.

import type { FileBase, FileChange } from '../types';
import type { Source, UriLike } from './fs-provider';

/** The scheme of the base versions: read-only, served by `BaseChanges.text`. */
export const BASE_SCHEME = 'escouade-base';
/** The query of a base URI that stands for no file at all: the right side of the comparison of a deleted file. */
const GONE = 'gone';

/** What it asks the core: the commands of the tab « Fichiers » and of the old editor, nothing else. */
export interface CoreChanges {
  fsBase(projectId: string, agentId: string | null, path: string): Promise<FileBase | null>;
  gitFiles(projectId: string, agentId: string | null): Promise<FileChange[]>;
}

/** The URIs it reads and makes. */
export interface ChangesUri extends UriLike {
  readonly query: string;
  /** Its text, as VS Code writes it: what tells two URIs apart. */
  toString(): string;
}

/** What it needs of the workbench: VS Code's URIs, and where a file of a source is (the file system provider's). */
export interface ChangesKit<U extends ChangesUri> {
  uri(u: { scheme: string; authority: string; path: string; query?: string }): U;
  sourceOf(uri: UriLike): { source: Source; rel: string } | null;
  uriOf(src: Source, rel: string): U | null;
}

export type Letter = 'A' | 'M' | 'D';

/** A file changed from the base, as the explorer and the list of the changes show it. */
export interface Changed {
  rel: string;
  letter: Letter;
}

/** What the explorer shows of a file changed: its letter, the theme colour of its kind, its tooltip, its folders marked. */
export interface Decoration {
  badge: Letter | typeof FOLDER_DOT;
  color: string;
  tooltip: string;
  propagate: false;
}

/** The mark of a folder holding files changed. */
export const FOLDER_DOT = '•';

/** The colours of the letters: git's keys of VS Code, laid from Escouade's tokens (`theme.ts`). */
export const LETTER_COLORS: Record<Letter, string> = {
  A: 'gitDecoration.addedResourceForeground',
  M: 'gitDecoration.modifiedResourceForeground',
  D: 'gitDecoration.deletedResourceForeground',
};

const keyOf = (s: Source) => `${s.projectId}\n${s.agentId ?? ''}`;

/** The files of the source among those `git_files` lists for it: the checkout's own, or the agent's worktree's. */
export function filesOf(list: readonly FileChange[], src: Source): Changed[] {
  return list
    .filter((f) => (src.agentId === null ? !f.inWorktree : f.inWorktree && f.agentId === src.agentId))
    .map((f) => ({ rel: f.path.replace(/\\/g, '/'), letter: f.status }));
}

interface Listed {
  source: Source;
  /** By the path, its case folded where the system ignores it. */
  files: Map<string, Changed>;
  /** The reference the base is named by (« main », « HEAD »), once a base was read; null: not a repository. */
  reference?: string | null;
}

export class BaseChanges<U extends ChangesUri> {
  private readonly sources = new Map<string, Listed>();
  private readonly baseListeners = new Set<(uri: U) => void>();
  private readonly listListeners = new Set<(uris: U[]) => void>();
  /** The base URIs VS Code read, to tell them changed. */
  private readonly opened = new Map<string, U>();
  private readonly reads = new Map<string, Promise<void>>();
  /** The base URIs of the files new since their base, by `toString()`. */
  private readonly fresh = new Set<string>();

  constructor(
    private readonly core: CoreChanges,
    private readonly kit: ChangesKit<U>,
    /** Paths compared without their case (Windows, macOS). */
    private readonly fold: boolean,
  ) {}

  private fileKey(rel: string) {
    return this.fold ? rel.toLowerCase() : rel;
  }

  /** The base version of a file of a source served, under `escouade-base:`; null for any other URI. */
  originalOf(file: UriLike): U | null {
    if (file.scheme !== 'file' || !this.kit.sourceOf(file)) return null;
    return this.kit.uri({ scheme: BASE_SCHEME, authority: file.authority, path: file.path });
  }

  /** The file a base URI is the base of. */
  fileOf(base: ChangesUri): U | null {
    if (base.scheme !== BASE_SCHEME || base.query === GONE) return null;
    return this.kit.uri({ scheme: 'file', authority: base.authority, path: base.path });
  }

  /** A URI of nothing: what a file deleted is compared with. */
  goneOf(file: UriLike): U {
    return this.kit.uri({ scheme: BASE_SCHEME, authority: file.authority, path: file.path, query: GONE });
  }

  /**
   * The text of a base URI, read by the core: '' for a file new since the base (all its lines added) and for a URI of
   * nothing; null out of a repository, or for a URI of no source (no marks).
   */
  async text(base: U): Promise<string | null> {
    if (base.scheme !== BASE_SCHEME) return null;
    if (base.query === GONE) return '';
    const file = this.fileOf(base);
    const at = file && this.kit.sourceOf(file);
    if (!at) return null;
    this.opened.set(base.toString(), base);
    const got = await this.core.fsBase(at.source.projectId, at.source.agentId, at.rel).catch(() => null);
    this.listed(at.source).reference = got ? got.reference : null;
    if (!got) return null;
    if (got.text === null) this.fresh.add(base.toString());
    else this.fresh.delete(base.toString());
    return got.text ?? '';
  }

  /** Whether the file of the base URI was not in its base, when it was read last. */
  isNew(base: UriLike): boolean {
    return this.fresh.has(base.toString());
  }

  /** The reference the base of the source is named by (« main », « HEAD »), once one of its bases was read. */
  reference(src: Source): string | null {
    return this.sources.get(keyOf(src))?.reference ?? null;
  }

  /** Called with each base URI told changed: VS Code reads it again (the content provider's `onDidChange`). */
  onDidChange(f: (uri: U) => void): { dispose(): void } {
    this.baseListeners.add(f);
    return { dispose: () => this.baseListeners.delete(f) };
  }

  /** Called with the files whose letter changed (the file decoration provider's `onDidChangeFileDecorations`). */
  onDidChangeFiles(f: (uris: U[]) => void): { dispose(): void } {
    this.listListeners.add(f);
    return { dispose: () => this.listListeners.delete(f) };
  }

  private listed(src: Source): Listed {
    const k = keyOf(src);
    let l = this.sources.get(k);
    if (!l) this.sources.set(k, (l = { source: src, files: new Map() }));
    return l;
  }

  /** The source is followed: its changed files read now. */
  async addSource(src: Source): Promise<void> {
    this.listed(src);
    await this.readFiles(src);
  }

  /** Its files changed read again; at most one read at a time per source, one more after it when asked meanwhile. */
  private readFiles(src: Source): Promise<void> {
    const k = keyOf(src);
    const running = this.reads.get(k);
    if (running) {
      const next = running.then(() => this.readOnce(src));
      this.reads.set(k, next);
      return next;
    }
    const p = this.readOnce(src).finally(() => {
      if (this.reads.get(k) === p) this.reads.delete(k);
    });
    this.reads.set(k, p);
    return p;
  }

  private async readOnce(src: Source): Promise<void> {
    const list = await this.core.gitFiles(src.projectId, src.agentId).catch(() => null);
    const l = this.sources.get(keyOf(src));
    if (!l || !list) return;
    const now = new Map(filesOf(list, src).map((f) => [this.fileKey(f.rel), f]));
    const touched: U[] = [];
    for (const [k, f] of now) if (l.files.get(k)?.letter !== f.letter) touched.push(...this.uris(src, f.rel));
    for (const [k, f] of l.files) if (!now.has(k)) touched.push(...this.uris(src, f.rel));
    l.files = now;
    if (touched.length) for (const f of this.listListeners) f(touched);
  }

  /** The file and its folders up to the source's root (excluded): their marks change with it. */
  private uris(src: Source, rel: string): U[] {
    const parts = rel.split('/');
    const out: U[] = [];
    for (let i = parts.length; i > 0; i--) {
      const u = this.kit.uriOf(src, parts.slice(0, i).join('/'));
      if (u) out.push(u);
    }
    return out;
  }

  /** The source no longer shown (its project closed). */
  removeProject(projectId: string) {
    for (const [k, l] of this.sources) if (l.source.projectId === projectId) this.sources.delete(k);
    for (const [k, u] of this.opened) if (this.kit.sourceOf(this.fileOf(u) ?? u)?.source.projectId === projectId) this.opened.delete(k);
  }

  /**
   * Something changed in the project (a file on disk, its git state: a commit, a checkout, the base moved): its sources'
   * files changed read again, and the bases VS Code read of it told changed, so that it draws its marks anew.
   */
  async projectChanged(projectId: string): Promise<void> {
    for (const u of this.opened.values()) {
      const file = this.fileOf(u);
      if (file && this.kit.sourceOf(file)?.source.projectId === projectId) for (const f of this.baseListeners) f(u);
    }
    await Promise.all([...this.sources.values()].filter((l) => l.source.projectId === projectId).map((l) => this.readFiles(l.source)));
  }

  /**
   * The letter of a file; for a folder holding files changed (the root of the source apart), its dot, in the colour of
   * a change whatever the files under it, as the editor of the 2.0 design draws it. Not left to VS Code's bubbling,
   * which takes the colour of whichever file it meets first.
   */
  decoration(uri: UriLike, words: (letter: Letter | 'folder', reference: string | null) => string): Decoration | undefined {
    if (uri.scheme !== 'file') return undefined;
    const at = this.kit.sourceOf(uri);
    if (!at) return undefined;
    const files = this.sources.get(keyOf(at.source))?.files;
    if (!files) return undefined;
    const reference = this.reference(at.source);
    const key = this.fileKey(at.rel);
    const f = files.get(key);
    if (f) return { badge: f.letter, color: LETTER_COLORS[f.letter], tooltip: words(f.letter, reference), propagate: false };
    if (at.rel === '' || ![...files.keys()].some((k) => k.startsWith(key + '/'))) return undefined;
    return { badge: FOLDER_DOT, color: LETTER_COLORS.M, tooltip: words('folder', reference), propagate: false };
  }

  /** The files of the source changed from its base, by their path. */
  changed(src: Source): Changed[] {
    const files = [...(this.sources.get(keyOf(src))?.files.values() ?? [])];
    return files.sort((a, b) => a.rel.localeCompare(b.rel));
  }

  /** The sources followed. */
  followed(): Source[] {
    return [...this.sources.values()].map((l) => l.source);
  }

  /** A comparison of the file with its base: its base and itself, or nothing and its base for a file deleted. */
  comparison(file: UriLike, deleted: boolean): { left: U; right: U } | null {
    const base = this.originalOf(file);
    if (!base) return null;
    const right = deleted ? this.goneOf(file) : this.kit.uri({ scheme: 'file', authority: file.authority, path: file.path });
    return { left: base, right };
  }
}

/** The lines added or changed, from VS Code's diff information of an editor (its changes' modified ranges). */
export function changedLines(changes: readonly { modified: { startLineNumber: number; endLineNumberExclusive: number } }[]) {
  return changes.reduce((n, c) => n + Math.max(0, c.modified.endLineNumberExclusive - c.modified.startLineNumber), 0);
}
