// Naming a file or a folder from the editor's tree, to create it or to rename one: where it goes and what keeps it
// from going there.

import { IS_MAC } from '../platform';

export type EntryKind = 'file' | 'dir';

/** What the source holds, for a name to be checked against. */
export interface Names {
  files: readonly string[];
  /** Folders without any file that the tree shows nonetheless (made since it was read). */
  dirs?: readonly string[];
  /** Files of `files` git ignores (the tree shows them greyed): one in the way is told as such. */
  ignored?: readonly string[];
  /** Windows' rules on names: everywhere but on macOS by default. */
  windows?: boolean;
}

const WHAT: Record<EntryKind, string> = { file: 'fichier', dir: 'dossier' };

/** The file `name` (folders to create included, `\` read as `/`) of the folder `dir` ('' for the root). */
export function newFilePath(dir: string, name: string): string {
  const rel = name.replaceAll('\\', '/');
  return dir ? `${dir}/${rel}` : rel;
}

/** A name Windows keeps as typed and gives a file: no ending dot or space, no character it refuses, no device. */
function windowsName(part: string): boolean {
  if (/[. ]$/.test(part) || /[<>:"|?*\u0000-\u001f]/.test(part)) return false;
  return !/^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i.test(part.split('.')[0].trimEnd());
}

/**
 * Why the file (or folder, `kind`) `name` cannot be created in `dir`, or null when it can (or nothing is typed yet). On
 * a disk that ignores case, as on Windows and macOS, `APP.TS` is the `app.ts` already there.
 */
export function newFileError(name: string, dir: string, names: Names, kind: EntryKind = 'file'): string | null {
  return nameError(name, dir, kind, names, null);
}

/**
 * Why the file or folder `from` cannot be renamed `name` (from its folder: `a/b.ts` moves it below), or null when it
 * can, when nothing is typed or when it is its own name. Only its case changed, it can: the one there is itself.
 */
export function renameError(name: string, from: string, kind: EntryKind, names: Names): string | null {
  return nameError(name, from.slice(0, Math.max(0, from.lastIndexOf('/'))), kind, names, from);
}

function nameError(name: string, dir: string, kind: EntryKind, names: Names, from: string | null): string | null {
  if (!name.trim()) return null;
  const rel = name.replaceAll('\\', '/');
  if (rel.startsWith('/')) return 'Un nom ne peut pas commencer par une barre oblique.';
  if (rel.endsWith('/')) return `Le nom doit finir par celui d’un ${WHAT[kind]}.`;
  const windows = names.windows ?? !IS_MAC;
  const parts = rel.split('/');
  if (parts.some((p) => p === '' || p === '.' || p === '..' || (windows && !windowsName(p)))) {
    return `« ${name} » n’est pas un nom de ${WHAT[kind]} valide.`;
  }
  const base = dir ? `${dir}/` : '';
  const path = (base + rel).toLowerCase();
  // Renamed, it and what it holds move out of the way.
  const own = from?.toLowerCase();
  const away = (p: string) => own !== undefined && (p === own || p.startsWith(own + '/'));
  if (from !== null) {
    if (base + rel === from) return null;
    if (path.startsWith(own + '/')) {
      return kind === 'dir'
        ? 'Un dossier ne peut pas aller dans lui-même.'
        : `« ${from.slice(from.lastIndexOf('/') + 1)} » est un fichier.`;
    }
    if (path === own) return null;
  }
  const files = names.files.map((f) => f.toLowerCase()).filter((f) => !away(f));
  const dirs = (names.dirs ?? []).map((d) => d.toLowerCase()).filter((d) => !away(d));
  const known = new Set(files);
  for (let i = 1; i < parts.length; i++) {
    if (known.has((base + parts.slice(0, i).join('/')).toLowerCase())) return `« ${parts[i - 1]} » est un fichier.`;
  }
  const below = (p: string) => p === path || p.startsWith(path + '/');
  if (known.has(path) || files.some(below) || dirs.some(below)) {
    const hidden = (names.ignored ?? []).some((f) => f.toLowerCase() === path);
    return `« ${name} » existe déjà à cet endroit${hidden ? ' (ignoré par git)' : ''}.`;
  }
  return null;
}
