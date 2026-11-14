// ESC-444 (study): the folders of a source as the VS Code view lists them, from the flat list of files `fs_tree` gives.
// Pure, for the tests: the file system provider (`fs-provider.ts`) keeps one per project.

export const FILE = 1;
export const DIR = 2;

/** Each folder (`''` for the root, `a/b` below) with its entries, a file or a folder. */
export type Dirs = Map<string, Map<string, typeof FILE | typeof DIR>>;

export function dirsOf(files: string[]): Dirs {
  const dirs: Dirs = new Map([['', new Map()]]);
  for (const file of files) {
    const parts = file.split('/').filter(Boolean);
    let dir = '';
    parts.forEach((name, i) => {
      const last = i === parts.length - 1;
      dirs.get(dir)!.set(name, last ? FILE : DIR);
      if (!last) {
        dir = dir ? `${dir}/${name}` : name;
        if (!dirs.has(dir)) dirs.set(dir, new Map());
      }
    });
  }
  return dirs;
}

/**
 * The path of `uriPath` (a URI's path: `/c:/code/demo/src/a.ts`, `/home/me/demo/src/a.ts`) from the root `rootPath`
 * (the same form), or null when it is not below it. `fold`: compared without the case (Windows, macOS).
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
  return uriPath.replace(/\/+$/, '').slice(root.length + 1);
}

/** The type of the entry `rel` in `dirs`, or null when there is none. */
export function typeOf(dirs: Dirs, rel: string): typeof FILE | typeof DIR | null {
  if (rel === '' || dirs.has(rel)) return DIR;
  const cut = rel.lastIndexOf('/');
  const parent = cut < 0 ? '' : rel.slice(0, cut);
  return dirs.get(parent)?.get(rel.slice(cut + 1)) ?? null;
}
