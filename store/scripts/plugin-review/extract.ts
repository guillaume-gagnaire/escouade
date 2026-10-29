import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { readArchive, type ArchiveProblem } from '../../server/utils/tar';

// Unpacks a version's archive for the review (docs/STORE.md §4.2, step 2), into a folder that must not exist yet:
// the regular files alone, read in memory by the store's own reader (server/utils/tar.ts), which refuses links,
// special files and paths that are absolute or climb with `..`. Each path is checked again against the folder before
// it is written; nothing is made executable; nothing runs.

export interface Extracted {
  files: Map<string, Buffer>;
  /** What the archive held that was not written, entry by entry. */
  problems: ArchiveProblem[];
  /** The commit GitHub wrote in the archive's header. */
  comment: string | null;
}

/** What a refused entry is, in the report's words. */
export function problemText(p: ArchiveProblem): string {
  switch (p.code) {
    case 'too-big':
      return `archive trop grosse (${p.size} octets)`;
    case 'bomb':
      return 'archive qui se décompresse au-delà de 200 Mo';
    case 'too-many':
      return 'plus de 20 000 entrées';
    case 'corrupt':
      return 'archive illisible';
    case 'symlink':
      return `lien symbolique non suivi : ${p.path}`;
    case 'hardlink':
      return `lien physique non suivi : ${p.path}`;
    case 'special':
      return `fichier spécial ignoré : ${p.path}`;
    case 'absolute':
      return `chemin absolu refusé : ${p.path}`;
    case 'parent':
      return `chemin qui remonte refusé : ${p.path}`;
  }
}

/** Whether `path` (the archive's) stays inside `root` once joined: the second guard after the reader's. */
export function inside(root: string, path: string): boolean {
  if (!path || path.includes('\0') || /^[A-Za-z]:/.test(path) || path.startsWith('/') || path.startsWith('\\')) return false;
  const full = resolve(root, path);
  const rel = relative(resolve(root), full);
  return !!rel && !rel.startsWith('..') && !rel.includes(`..${sep}`) && resolve(root, rel) === full;
}

/** Reads `gz` and writes its regular files under `dest`, which is created (and must not exist). */
export function extract(gz: Buffer, dest: string): Extracted {
  const read = readArchive(gz);
  mkdirSync(dest, { recursive: false });
  const problems = [...read.problems];
  const files = new Map<string, Buffer>();
  for (const [path, bytes] of read.files) {
    if (!inside(dest, path)) {
      problems.push({ code: 'parent', path });
      continue;
    }
    const full = join(dest, path);
    try {
      mkdirSync(dirname(full), { recursive: true });
      // `wx`: never over what is already there (a file named like a folder of another entry).
      writeFileSync(full, bytes, { flag: 'wx', mode: 0o644 });
      files.set(path, bytes);
    } catch {
      problems.push({ code: 'special', path });
    }
  }
  return { files, problems, comment: read.comment };
}
