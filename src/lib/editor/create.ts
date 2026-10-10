// Naming a file to create from the editor's tree: where it goes and what keeps it from being created.

import { IS_MAC } from '../platform';

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
 * Why the file `name` cannot be created in `dir` among the source's `files`, or null when it can (or nothing is typed
 * yet). On a disk that ignores case, as on Windows and macOS, `APP.TS` is the `app.ts` already there. A file of
 * `ignored` (git ignores it, the tree shows it greyed) is told as such.
 */
export function newFileError(
  name: string,
  dir: string,
  files: string[],
  windows = !IS_MAC,
  ignored: readonly string[] = [],
): string | null {
  if (!name.trim()) return null;
  const rel = name.replaceAll('\\', '/');
  if (rel.startsWith('/')) return 'Un nom ne peut pas commencer par une barre oblique.';
  if (rel.endsWith('/')) return 'Le nom doit finir par celui d’un fichier.';
  const parts = rel.split('/');
  if (parts.some((p) => p === '' || p === '.' || p === '..' || (windows && !windowsName(p)))) {
    return `« ${name} » n’est pas un nom de fichier valide.`;
  }
  const known = new Set(files.map((f) => f.toLowerCase()));
  const base = dir ? `${dir}/` : '';
  for (let i = 1; i < parts.length; i++) {
    if (known.has((base + parts.slice(0, i).join('/')).toLowerCase())) return `« ${parts[i - 1]} » est un fichier.`;
  }
  const path = (base + rel).toLowerCase();
  if (known.has(path) || files.some((f) => f.toLowerCase().startsWith(path + '/'))) {
    const hidden = ignored.some((f) => f.toLowerCase() === path);
    return `« ${name} » existe déjà à cet endroit${hidden ? ' (ignoré par git)' : ''}.`;
  }
  return null;
}
