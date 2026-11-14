// ESC-444 (study): the files of the VS Code view, by the core. A file system provider of VS Code (`file:` URIs of the
// projects' folders, put in front of the in-memory one where VS Code keeps its settings) over the editor's commands
// (`fs_tree`, `fs_read`, `fs_write`, `fs_create`, `fs_mkdir`, `fs_rename`, `fs_delete`): the page never reads the disk.
// A file is read as the editor reads it (its text with LF, its line endings and its BOM kept aside) and written back the
// same way, refused when it changed on disk since (`expectedHash`).

import { Emitter } from '@codingame/monaco-vscode-api/vscode/vs/base/common/event';
import { Disposable } from '@codingame/monaco-vscode-api/vscode/vs/base/common/lifecycle';
import {
  FileSystemProviderCapabilities,
  FileSystemProviderError,
  FileSystemProviderErrorCode,
  FileType,
  type IFileChange,
  type IFileDeleteOptions,
  type IFileOverwriteOptions,
  type IFileSystemProviderWithFileReadWriteCapability,
  type IFileWriteOptions,
  type IStat,
} from '@codingame/monaco-vscode-files-service-override';
import { api } from '../ipc';
import { dirsOf, relativeTo, typeOf, type Dirs } from './tree';

type Uri = { path: string; scheme: string };

interface Root {
  projectId: string;
  /** The root's URI path (`/c:/code/demo`). */
  path: string;
  dirs: Dirs | null;
}

const fold = /Win|Mac/i.test(navigator.userAgent);
const notFound = () => FileSystemProviderError.create('not found', FileSystemProviderErrorCode.FileNotFound);

export class EscouadeFileSystem extends Disposable implements IFileSystemProviderWithFileReadWriteCapability {
  readonly capabilities = FileSystemProviderCapabilities.FileReadWrite | (fold ? 0 : FileSystemProviderCapabilities.PathCaseSensitive);
  private readonly changes = new Emitter<readonly IFileChange[]>();
  readonly onDidChangeFile = this.changes.event;
  readonly onDidChangeCapabilities = new Emitter<void>().event;
  private readonly roots: Root[] = [];
  /** How each file read is to be written back. */
  private readonly meta = new Map<string, { hash: string; eol: 'lf' | 'crlf'; bom: boolean }>();
  /** What the calls to the core took (ms), for the study's measures. */
  readonly timings: { op: string; ms: number }[] = [];

  /** Serves the project's folder at `rootPath` (its URI path). */
  addRoot(projectId: string, rootPath: string) {
    if (!this.roots.some((r) => r.projectId === projectId)) this.roots.push({ projectId, path: rootPath, dirs: null });
  }

  private find(uri: Uri): { root: Root; rel: string } | null {
    if (uri.scheme !== 'file') return null;
    for (const root of this.roots) {
      const rel = relativeTo(root.path, uri.path, fold);
      if (rel !== null) return { root, rel };
    }
    return null;
  }

  private async timed<T>(op: string, f: () => Promise<T>): Promise<T> {
    const t0 = performance.now();
    try {
      return await f();
    } finally {
      this.timings.push({ op, ms: performance.now() - t0 });
    }
  }

  private async dirs(root: Root): Promise<Dirs> {
    if (!root.dirs) {
      const tree = await this.timed('tree', () => api.fsTree(root.projectId, null));
      root.dirs = dirsOf(tree.files);
    }
    return root.dirs;
  }

  private changed(root: Root, uris: { resource: Uri; type: number }[]) {
    root.dirs = null;
    this.changes.fire(uris as unknown as IFileChange[]);
  }

  watch() {
    return Disposable.None;
  }

  async stat(uri: Uri): Promise<IStat> {
    const at = this.find(uri);
    if (!at) throw notFound();
    const type = typeOf(await this.dirs(at.root), at.rel);
    if (type === null) throw notFound();
    return { type: type === 1 ? FileType.File : FileType.Directory, ctime: 0, mtime: 0, size: 0 };
  }

  async readdir(uri: Uri): Promise<[string, FileType][]> {
    const at = this.find(uri);
    if (!at) throw notFound();
    const entries = (await this.dirs(at.root)).get(at.rel);
    if (!entries) throw notFound();
    return [...entries].map(([name, t]) => [name, t === 1 ? FileType.File : FileType.Directory]);
  }

  async readFile(uri: Uri): Promise<Uint8Array> {
    const at = this.find(uri);
    if (!at) throw notFound();
    const file = await this.timed('read', () => api.fsRead(at.root.projectId, null, at.rel));
    if (file.kind !== 'text' || file.text === null)
      throw FileSystemProviderError.create(
        file.kind === 'tooLarge' ? 'too large' : 'binary',
        file.kind === 'tooLarge' ? FileSystemProviderErrorCode.FileTooLarge : FileSystemProviderErrorCode.Unavailable,
      );
    this.meta.set(uri.path, { hash: file.hash, eol: file.eol, bom: file.bom });
    return new TextEncoder().encode(file.text);
  }

  async writeFile(uri: Uri, content: Uint8Array, opts: IFileWriteOptions): Promise<void> {
    const at = this.find(uri);
    if (!at) throw notFound();
    const exists = typeOf(await this.dirs(at.root), at.rel) !== null;
    if (!exists && !opts.create) throw notFound();
    if (exists && !opts.overwrite) throw FileSystemProviderError.create('exists', FileSystemProviderErrorCode.FileExists);
    if (!exists) await api.fsCreate(at.root.projectId, null, at.rel);
    const meta = this.meta.get(uri.path);
    const hash = await this.timed('write', () =>
      api.fsWrite({
        projectId: at.root.projectId,
        agentId: null,
        path: at.rel,
        text: new TextDecoder().decode(content).replace(/\r\n/g, '\n'),
        eol: meta?.eol ?? 'lf',
        bom: meta?.bom ?? false,
        expectedHash: exists ? (meta?.hash ?? null) : null,
      }),
    );
    this.meta.set(uri.path, { hash, eol: meta?.eol ?? 'lf', bom: meta?.bom ?? false });
    if (!exists) this.changed(at.root, [{ resource: uri, type: 1 }]);
  }

  async mkdir(uri: Uri): Promise<void> {
    const at = this.find(uri);
    if (!at) throw notFound();
    await api.fsMkdir(at.root.projectId, null, at.rel);
    this.changed(at.root, [{ resource: uri, type: 1 }]);
  }

  async delete(uri: Uri, _opts: IFileDeleteOptions): Promise<void> {
    const at = this.find(uri);
    if (!at) throw notFound();
    await api.fsDelete(at.root.projectId, null, at.rel);
    this.changed(at.root, [{ resource: uri, type: 2 }]);
  }

  async rename(from: Uri, to: Uri, _opts: IFileOverwriteOptions): Promise<void> {
    const a = this.find(from);
    const b = this.find(to);
    if (!a || !b || a.root !== b.root) throw notFound();
    await api.fsRename(a.root.projectId, null, a.rel, b.rel);
    this.changed(a.root, [
      { resource: from, type: 2 },
      { resource: to, type: 1 },
    ]);
  }
}
