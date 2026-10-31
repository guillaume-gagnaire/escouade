import { gzipSync } from 'node:zlib';
import { TooBig, type Source, type TreeEntry } from './source';
import { ARCHIVE_MAX, readArchive, UNPACKED_MAX, type ArchiveProblem, type ArchiveRead } from './tar';

// A plugin in a folder of its repository (ESC-313, docs/STORE.md §3.6): the folders of an archive that hold a
// manifest, a folder taken out of the archive (its files, its own problems, never through a link), and the archive the
// store keeps of it: a tar.gz of that folder alone, made the same way every time, so that a folder at a commit always
// gives the same bytes and the same fingerprint. The root's plugin keeps GitHub's archive as it came.

export const MANIFEST = 'escouade-plugin.json';
/** The folders with a manifest the page lists, at most. */
export const FOLDERS_MAX = 50;

/** The folders of the archive that hold a manifest, the root (`''`) first, then by name; `FOLDERS_MAX` at most. */
export function manifestFolders(read: ArchiveRead): string[] {
  const found: string[] = [];
  for (const p of read.files.keys()) {
    if (p === MANIFEST) found.push('');
    else if (p.endsWith(`/${MANIFEST}`)) found.push(p.slice(0, -MANIFEST.length - 1));
  }
  return found.sort((a, b) => (a === '' ? -1 : b === '' ? 1 : a.localeCompare(b))).slice(0, FOLDERS_MAX);
}

/** The links of the archive (symbolic or hard), by their path. */
const links = (read: ArchiveRead) =>
  read.problems.flatMap((p) => ((p.code === 'symlink' || p.code === 'hardlink') && 'path' in p ? [p.path] : []));

/** The link the folder goes through (the folder itself or one of its parents), if any: the store never follows one. */
export function linkOn(read: ArchiveRead, path: string): string | null {
  if (!path) return null;
  const parts = path.split('/');
  const all = new Set(links(read));
  for (let i = 1; i <= parts.length; i++) {
    const at = parts.slice(0, i).join('/');
    if (all.has(at)) return at;
  }
  return null;
}

export type FolderProblem = 'link' | 'no-manifest';

/** Whether `path` (normalized) may be submitted from this archive: not through a link, and with a manifest. */
export function folderProblem(read: ArchiveRead, path: string): FolderProblem | null {
  if (linkOn(read, path)) return 'link';
  return read.files.has(path ? `${path}/${MANIFEST}` : MANIFEST) ? null : 'no-manifest';
}

const under = (p: string, path: string) => p.startsWith(`${path}/`);

/**
 * The folder `path` of the archive, as if it were the archive: its files and folders by their path inside it, the
 * problems of its own entries (a link, a device), and those of the whole archive (too big, a bomb, corrupt). The root
 * gives the archive itself.
 */
export function scope(read: ArchiveRead, path: string): ArchiveRead {
  if (!path) return read;
  const cut = (p: string) => p.slice(path.length + 1);
  const files = new Map<string, Buffer>();
  for (const [p, b] of read.files) if (under(p, path)) files.set(cut(p), b);
  const dirs = new Set<string>();
  for (const d of read.dirs) if (under(d, path)) dirs.add(cut(d));
  const problems: ArchiveProblem[] = [];
  for (const p of read.problems) {
    if (!('path' in p)) problems.push(p);
    // A path refused because it is absolute or climbs: it cannot be placed, it counts for every folder.
    else if (p.code === 'absolute' || p.code === 'parent') problems.push(p);
    else if (under(p.path, path)) problems.push({ ...p, path: cut(p.path) });
  }
  return { files, dirs, problems, comment: read.comment };
}

// ---- The folder's archive ----

const BLOCK = 512;

function octal(n: number, len: number): string {
  return n.toString(8).padStart(len - 1, '0') + '\0';
}

function header(name: string, size: number, type: '0' | '5' | 'g' | 'x'): Buffer {
  const h = Buffer.alloc(BLOCK);
  h.write(name, 0, 100, 'utf8');
  h.write(octal(type === '5' ? 0o755 : 0o644, 8), 100, 'ascii');
  h.write(octal(0, 8), 108, 'ascii');
  h.write(octal(0, 8), 116, 'ascii');
  h.write(octal(size, 12), 124, 'ascii');
  h.write(octal(0, 12), 136, 'ascii');
  h.write('        ', 148, 'ascii');
  h.write(type, 156, 'ascii');
  h.write('ustar\0', 257, 'ascii');
  h.write('00', 263, 'ascii');
  let sum = 0;
  for (const b of h) sum += b;
  h.write(octal(sum, 7) + ' ', 148, 'ascii');
  return h;
}

/** A pax record, `<len> key=value\n`, its length counting itself. */
function record(key: string, value: string): string {
  const body = ` ${key}=${value}\n`;
  let len = Buffer.byteLength(body) + 1;
  while (String(len).length + Buffer.byteLength(body) !== len) len = String(len).length + Buffer.byteLength(body);
  return `${len}${body}`;
}

const pad = (b: Buffer) => Buffer.concat([b, Buffer.alloc((BLOCK - (b.length % BLOCK)) % BLOCK)]);

function entry(name: string, type: '0' | '5', data: Buffer): Buffer[] {
  const out: Buffer[] = [];
  // A name past the 100 bytes of the header goes in a pax header before it.
  if (Buffer.byteLength(name) > 99) {
    const pax = Buffer.from(record('path', name));
    out.push(header('././@PaxHeader', pax.length, 'x'), pad(pax));
  }
  out.push(header(Buffer.byteLength(name) > 99 ? name.slice(0, 99) : name, data.length, type));
  if (data.length) out.push(pad(data));
  return out;
}

/**
 * The archive of a folder, as the store keeps it: a gzip of a tar whose global header names the commit, then its
 * folders and regular files under one top folder (`top/`), sorted, without dates nor owners: the same folder at the same
 * commit gives the same bytes. Read as the app reads GitHub's archives (the top folder taken off).
 */
export function packFolder(folder: ArchiveRead, top: string, commit: string): Buffer {
  const parts: Buffer[] = [];
  const g = Buffer.from(record('comment', commit));
  parts.push(header('pax_global_header', g.length, 'g'), pad(g));
  parts.push(...entry(`${top}/`, '5', Buffer.alloc(0)));
  for (const d of [...folder.dirs].sort()) parts.push(...entry(`${top}/${d}/`, '5', Buffer.alloc(0)));
  for (const p of [...folder.files.keys()].sort()) parts.push(...entry(`${top}/${p}`, '0', folder.files.get(p)!));
  parts.push(Buffer.alloc(BLOCK * 2));
  const gz = gzipSync(Buffer.concat(parts), { level: 9 });
  // The system byte of the gzip header says where it was made: the same everywhere, so are the bytes.
  gz[9] = 0xff;
  return gz;
}

// ---- The folders the submission's page offers ----

export interface FolderList {
  /** The folders with a manifest, the root first; `FOLDERS_MAX` at most. */
  folders: string[];
  /** Whether there were more. */
  more: boolean;
  /** Every folder with a manifest, and the links, to judge a folder typed. */
  all: Set<string>;
  links: string[];
}

/** What the page needs of an archive: its folders with a manifest and its links. */
export function folderList(read: ArchiveRead): FolderList {
  const every = new Set<string>();
  for (const p of read.files.keys()) {
    if (p === MANIFEST) every.add('');
    else if (p.endsWith(`/${MANIFEST}`)) every.add(p.slice(0, -MANIFEST.length - 1));
  }
  return { folders: manifestFolders(read), more: every.size > FOLDERS_MAX, all: every, links: links(read) };
}

const lists = new Map<string, FolderList>();

/** The folder list of `key` (`repo@commit`), read once by `load`; the last 20 are kept. */
export async function cachedFolders(key: string, load: () => Promise<FolderList>): Promise<FolderList> {
  const kept = lists.get(key);
  if (kept) return kept;
  const list = await load();
  lists.set(key, list);
  if (lists.size > 20) lists.delete(lists.keys().next().value!);
  return list;
}

/** Whether a folder typed may be submitted, by the list: not through a link, with a manifest. */
export function judgeFolder(list: FolderList, path: string): FolderProblem | null {
  const parts = path.split('/');
  for (let i = 1; path && i <= parts.length; i++) if (list.links.includes(parts.slice(0, i).join('/'))) return 'link';
  return list.all.has(path) ? null : 'no-manifest';
}

// ---- A folder read by the commit's tree ----

/** The files a plugin's folder may hold: past them, the store does not read it file by file. */
export const FOLDER_FILES_MAX = 2_000;
/** The files read from GitHub at once. */
const AT_ONCE = 8;

const README_OR_LICENSE = /^(README(\.(fr|en))?\.md|(LICEN[CS]E|COPYING)(\.[A-Za-z0-9]+)?)$/i;

const empty = (): ArchiveRead => ({ files: new Map(), dirs: new Set(), problems: [], comment: null });

/** The tree's entries as an archive read without bytes: its folders, its links, every file present. */
export function treeRead(entries: TreeEntry[]): ArchiveRead {
  const read = empty();
  for (const e of entries) {
    if (e.type === 'tree') read.dirs.add(e.path);
    else if (e.type === 'commit') read.problems.push({ code: 'special', path: e.path });
    else if (e.mode === '120000') read.problems.push({ code: 'symlink', path: e.path });
    else read.files.set(e.path, Buffer.alloc(0));
  }
  return read;
}

/**
 * The plugin's folder `path` at `commit`, read without the repository's archive (ESC-313): the commit's tree once,
 * then the folder's files alone, by their blobs, and the root's README and LICENSE (they stand in for the folder's).
 * A big repository (the app's, with its twenty plugins) costs one tree and the folder's files, never its 50 MB. Bounded
 * as the archive is: `FOLDER_FILES_MAX` files, `UNPACKED_MAX` bytes. A tree GitHub cut short (past 100,000 entries)
 * falls back to the archive, bounded at 50 MB. `root` holds the root's files read.
 */
export async function readFolder(
  source: Source,
  repo: string,
  commit: string,
  path: string,
): Promise<{ read: ArchiveRead; root: ArchiveRead; pathLink: string | null }> {
  const tree = await source.tree(repo, commit);
  if (tree.truncated) {
    let whole: ArchiveRead;
    try {
      whole = readArchive(await source.tarball(repo, commit, ARCHIVE_MAX));
    } catch (e) {
      if (!(e instanceof TooBig)) throw e;
      const read = { ...empty(), problems: [{ code: 'too-big', size: e.size }] as ArchiveProblem[] };
      return { read, root: empty(), pathLink: null };
    }
    const pathLink = linkOn(whole, path);
    return { read: pathLink ? { ...empty(), comment: whole.comment } : scope(whole, path), root: whole, pathLink };
  }
  const shape = treeRead(tree.entries);
  const pathLink = linkOn(shape, path);
  const read: ArchiveRead = { ...empty(), comment: commit };
  const root: ArchiveRead = { ...empty(), comment: commit };
  const wanted: { entry: TreeEntry; into: ArchiveRead; as: string }[] = [];
  for (const e of tree.entries) {
    if (e.type === 'blob' && e.mode !== '120000' && !e.path.includes('/') && README_OR_LICENSE.test(e.path))
      wanted.push({ entry: e, into: root, as: e.path });
    if (pathLink || !under(e.path, path)) continue;
    const as = e.path.slice(path.length + 1);
    if (e.type === 'tree') read.dirs.add(as);
    else if (e.type === 'commit') read.problems.push({ code: 'special', path: as });
    else if (e.mode === '120000') read.problems.push({ code: 'symlink', path: as });
    else wanted.push({ entry: e, into: read, as });
  }
  const inFolder = wanted.filter((w) => w.into === read);
  const total = inFolder.reduce((n, w) => n + (w.entry.size ?? 0), 0);
  if (inFolder.length > FOLDER_FILES_MAX) return { read: { ...empty(), problems: [{ code: 'too-many' }] }, root, pathLink };
  if (total > UNPACKED_MAX) return { read: { ...empty(), problems: [{ code: 'bomb' }] }, root, pathLink };
  for (let i = 0; i < wanted.length; i += AT_ONCE)
    await Promise.all(
      wanted.slice(i, i + AT_ONCE).map(async (w) => {
        w.into.files.set(w.as, await source.blob(repo, w.entry.sha, Math.min(UNPACKED_MAX, (w.entry.size ?? UNPACKED_MAX) + 1)));
      }),
    );
  return { read, root, pathLink };
}
