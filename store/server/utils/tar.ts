import { gunzipSync } from 'node:zlib';

// Reads the archive of a commit as GitHub makes it (`GET /repos/{o}/{r}/tarball/<sha>`: a gzip of a tar, a global
// pax header with the commit, then everything under one folder `owner-repo-sha/`), in memory, without writing a
// file and without running anything (docs/STORE.md, §3.3, §7). What it refuses is said, entry by entry: a link
// (symbolic or hard), a device or a pipe, a path that is absolute or climbs with `..`. Its sizes are bounded: the
// archive as kept (50 MB), what it unpacks to (a bomb stops at 200 MB) and its number of entries.

/** The archive as GitHub sends it and the store keeps it: 50 MB at most (docs/STORE.md, §3.3, check 8). */
export const ARCHIVE_MAX = 50 * 1024 * 1024;
/** What the archive may unpack to: a gzip bomb stops here, before it fills the worker's memory. */
export const UNPACKED_MAX = 200 * 1024 * 1024;
/** Entries an archive may hold. */
export const ENTRIES_MAX = 20_000;

export type ArchiveProblem =
  | { code: 'too-big'; size: number }
  | { code: 'bomb' }
  | { code: 'too-many' }
  | { code: 'corrupt' }
  | { code: 'symlink' | 'hardlink' | 'special' | 'absolute' | 'parent'; path: string };

export interface ArchiveRead {
  /** The regular files, by their path inside the plugin (GitHub's top folder taken off), with their bytes. */
  files: Map<string, Buffer>;
  /** The folders, by the same paths. */
  dirs: Set<string>;
  /** What the archive holds that the store refuses; empty when it is clean. */
  problems: ArchiveProblem[];
  /** The commit GitHub wrote in the global header (`comment`), if any. */
  comment: string | null;
}

const BLOCK = 512;

/** A field of a header: text up to its first NUL. */
const field = (b: Buffer, start: number, len: number) => {
  const raw = b.subarray(start, start + len);
  const nul = raw.indexOf(0);
  return raw.subarray(0, nul < 0 ? len : nul).toString('utf8');
};

/** A number of a header: octal text, or base-256 when its first bit is set (GNU, for sizes past 8 GB). */
function number(b: Buffer, start: number, len: number): number {
  if (b[start]! & 0x80) {
    let n = b[start]! & 0x7f;
    for (let i = start + 1; i < start + len; i++) n = n * 256 + b[i]!;
    return n;
  }
  const s = field(b, start, len).trim();
  return s ? parseInt(s, 8) : 0;
}

/** Whether a header's checksum holds: the bytes summed with the checksum's own field as spaces. */
function checksumHolds(b: Buffer): boolean {
  const want = number(b, 148, 8);
  let sum = 0;
  for (let i = 0; i < BLOCK; i++) sum += i >= 148 && i < 156 ? 32 : b[i]!;
  return sum === want;
}

/** The records of a pax header (`<len> key=value\n`). */
function pax(data: Buffer): Record<string, string> {
  const out: Record<string, string> = {};
  let at = 0;
  while (at < data.length) {
    const space = data.indexOf(32, at);
    if (space < 0) break;
    const len = parseInt(data.subarray(at, space).toString('ascii'), 10);
    if (!Number.isFinite(len) || len <= 0 || at + len > data.length) break;
    const record = data.subarray(space + 1, at + len - 1).toString('utf8');
    const eq = record.indexOf('=');
    if (eq > 0) out[record.slice(0, eq)] = record.slice(eq + 1);
    at += len;
  }
  return out;
}

/** A path as the archive names it, refused if it is absolute or climbs out; GitHub's top folder taken off. */
function inner(raw: string): { path: string } | { problem: 'absolute' | 'parent' } {
  const p = raw.replace(/\\/g, '/');
  if (p.startsWith('/') || /^[A-Za-z]:/.test(p)) return { problem: 'absolute' };
  const parts = p.split('/').filter((s) => s !== '' && s !== '.');
  if (parts.includes('..')) return { problem: 'parent' };
  return { path: parts.slice(1).join('/') };
}

/**
 * Reads `gz`, the archive of a commit: its files in memory, what it holds that the store refuses. A gzip that does not
 * read, or a tar whose header does not hold, is `corrupt`; past `UNPACKED_MAX`, a `bomb`.
 */
export function readArchive(gz: Buffer, limits = { archive: ARCHIVE_MAX, unpacked: UNPACKED_MAX, entries: ENTRIES_MAX }): ArchiveRead {
  const read: ArchiveRead = { files: new Map(), dirs: new Set(), problems: [], comment: null };
  if (gz.length > limits.archive) {
    read.problems.push({ code: 'too-big', size: gz.length });
    return read;
  }
  let tar: Buffer;
  try {
    tar = gunzipSync(gz, { maxOutputLength: limits.unpacked });
  } catch (e) {
    read.problems.push({ code: e instanceof RangeError || (e as { code?: string }).code === 'ERR_BUFFER_TOO_LARGE' ? 'bomb' : 'corrupt' });
    return read;
  }
  let at = 0;
  let entries = 0;
  let next: Record<string, string> = {};
  let longName: string | null = null;
  while (at + BLOCK <= tar.length) {
    const h = tar.subarray(at, at + BLOCK);
    if (h.every((x) => x === 0)) break;
    if (!checksumHolds(h)) {
      read.problems.push({ code: 'corrupt' });
      return read;
    }
    const type = String.fromCharCode(h[156] || 48);
    const size = number(h, 124, 12);
    const start = at + BLOCK;
    const end = start + size;
    if (end > tar.length) {
      read.problems.push({ code: 'corrupt' });
      return read;
    }
    const data = tar.subarray(start, end);
    at = start + Math.ceil(size / BLOCK) * BLOCK;

    // The headers that describe the next entry, or the whole archive.
    if (type === 'g') {
      read.comment = pax(data).comment ?? read.comment;
      continue;
    }
    if (type === 'x') {
      next = { ...next, ...pax(data) };
      continue;
    }
    if (type === 'L' || type === 'K') {
      // The long name of the next entry (`L`); the long target of a link (`K`) does not matter, a link is refused.
      if (type === 'L') longName = data.toString('utf8').replace(/\0+$/, '');
      continue;
    }

    if (++entries > limits.entries) {
      read.problems.push({ code: 'too-many' });
      return read;
    }
    const prefix = field(h, 345, 155);
    const name = next.path ?? longName ?? (prefix ? `${prefix}/${field(h, 0, 100)}` : field(h, 0, 100));
    next = {};
    longName = null;

    const where = inner(name);
    if ('problem' in where) {
      read.problems.push({ code: where.problem, path: name });
      continue;
    }
    const path = where.path;
    if (type === '2') read.problems.push({ code: 'symlink', path: path || name });
    else if (type === '1') read.problems.push({ code: 'hardlink', path: path || name });
    else if (type === '5') {
      if (path) read.dirs.add(path);
    } else if (type === '0' || type === '7') {
      if (path) read.files.set(path, data);
    } else read.problems.push({ code: 'special', path: path || name });
  }
  return read;
}
