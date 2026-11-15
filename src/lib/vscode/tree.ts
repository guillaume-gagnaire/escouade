// The paths of the VS Code view (ESC-447): the folders of a source as the view lists them, from the flat list of files
// `fs_tree` gives, and a URI's path read against a root. Pure, for the tests: the file system provider (`fs-provider.ts`)
// keeps one listing per source.

export const FILE = 1;
export const DIR = 2;
export type Kind = typeof FILE | typeof DIR;

/** Each folder (`''` for the root, `a/b` below) with its entries, a file or a folder. */
export type Dirs = Map<string, Map<string, Kind>>;

export function dirsOf(files: readonly string[]): Dirs {
  const dirs: Dirs = new Map([['', new Map()]]);
  for (const file of files) {
    const parts = file.split('/').filter(Boolean);
    let dir = '';
    parts.forEach((name, i) => {
      const last = i === parts.length - 1;
      // A folder named like a file seen before keeps being a folder: the tree lists files only.
      if (!last || !dirs.get(dir)!.has(name)) dirs.get(dir)!.set(name, last ? FILE : DIR);
      if (!last) {
        dir = dir ? `${dir}/${name}` : name;
        if (!dirs.has(dir)) dirs.set(dir, new Map());
      }
    });
  }
  return dirs;
}

/** Every entry of `dirs` (files and folders, not the root) by its path from the root. */
export function entriesOf(dirs: Dirs): Map<string, Kind> {
  const out = new Map<string, Kind>();
  for (const [dir, entries] of dirs) for (const [name, kind] of entries) out.set(dir ? `${dir}/${name}` : name, kind);
  return out;
}

/**
 * The path of `uriPath` (a URI's path: `/c:/code/demo/src/a.ts`, `/home/me/demo/src/a.ts`) from the root `rootPath`
 * (the same form), or null when it is not below it, or names `.` or `..` on the way (never out of the root, whatever
 * the core would make of it). `fold`: compared without the case (Windows, macOS).
 */
export function relativeTo(rootPath: string, uriPath: string, fold: boolean): string | null {
  const norm = (p: string) => {
    const s = p.replace(/\/+$/, '');
    return fold ? s.toLowerCase() : s;
  };
  const root = norm(rootPath);
  const path = norm(uriPath);
  if (path === root) return '';
  if (!path.startsWith(root + '/')) return null;
  const rel = uriPath.replace(/\/+$/, '').slice(root.length + 1);
  if (rel.split('/').some((part) => part === '' || part === '.' || part === '..')) return null;
  return rel;
}

/** The type of the entry `rel` in `dirs`, or null when there is none. */
export function typeOf(dirs: Dirs, rel: string): Kind | null {
  if (rel === '' || dirs.has(rel)) return DIR;
  const cut = rel.lastIndexOf('/');
  const parent = cut < 0 ? '' : rel.slice(0, cut);
  return dirs.get(parent)?.get(rel.slice(cut + 1)) ?? null;
}

/** A `file:` URI, as VS Code makes one of a path of the system: its host (for a share), its path. */
export interface FileUri {
  authority: string;
  path: string;
}

/**
 * An absolute path of the system (`C:\code\demo\a.ts`, `/home/me/a.ts`, `\\wsl.localhost\Ubuntu\home\me`) as a `file:`
 * URI, as VS Code's `Uri.file` writes it: forward slashes, the drive letter in lower case (`/c:/code/demo/a.ts`), the
 * host of a share as its authority. Null for a path that is not absolute.
 */
export function fileUriOf(abs: string): FileUri | null {
  const p = abs.replace(/\\/g, '/');
  const share = /^\/\/([^/]+)(\/.*)?$/.exec(p);
  if (share) return { authority: share[1], path: share[2] || '/' };
  const drive = /^([A-Za-z]):(\/|$)/.exec(p);
  if (drive) return { authority: '', path: `/${drive[1].toLowerCase()}:${p.slice(2) || '/'}` };
  return p.startsWith('/') ? { authority: '', path: p } : null;
}
