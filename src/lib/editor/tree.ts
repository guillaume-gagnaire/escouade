// The editor's file tree: a flat list of paths shown as folders and files, the way VS Code's explorer does.

import { IS_MAC } from '../platform';

export type FileStatus = 'M' | 'A' | 'D';

export interface TreeRow {
  /** `new`: the field naming a file to create in the folder `path` ('' for the root). */
  kind: 'dir' | 'file' | 'new';
  path: string;
  /** For a folder holding a single folder and no file, the chain shown on one row (`src/lib/editor`). */
  name: string;
  depth: number;
  /** A folder shown open. */
  open: boolean;
  /** A file changed in git. */
  status: FileStatus | null;
  /** A folder holding changed files: the strongest change among them. */
  inside: FileStatus | null;
  /** A file git ignores that the tree shows nonetheless: one the project copies into its worktrees. */
  ignored: boolean;
}

interface Node {
  dirs: Map<string, Node>;
  files: string[];
}

const nameOf = (p: string) => p.slice(p.lastIndexOf('/') + 1);
const collator = new Intl.Collator(undefined, { sensitivity: 'base' });
const byName = (a: string, b: string) => collator.compare(a, b) || (a < b ? -1 : a > b ? 1 : 0);
const STRENGTH: FileStatus[] = ['M', 'A', 'D'];

/**
 * The rows shown, `adding` the folder getting a new file ('' for the root), its field first among what it holds.
 * `ignored` lists the files git ignores, which the tree draws apart; `dirs` folders to show even without any file (git
 * lists none of those).
 */
export function treeRows(
  files: string[],
  expanded: Record<string, boolean>,
  status: Record<string, FileStatus>,
  adding: string | null = null,
  ignored: readonly string[] = [],
  dirs: readonly string[] = [],
): TreeRow[] {
  const ignoredSet = new Set(ignored);
  const root: Node = { dirs: new Map(), files: [] };
  /** The node of the folder `parts`, made with those above it when missing. */
  const folder = (parts: string[]) => {
    let n = root;
    for (const d of parts) {
      let c = n.dirs.get(d);
      if (!c) n.dirs.set(d, (c = { dirs: new Map(), files: [] }));
      n = c;
    }
    return n;
  };
  for (const f of files) folder(f.split('/').slice(0, -1)).files.push(f);
  for (const d of dirs) folder(d.split('/'));
  // The strongest change under each folder, in one pass over the changes.
  const held = new Map<string, FileStatus>();
  for (const [p, s] of Object.entries(status)) {
    for (const d of ancestors(p)) {
      const was = held.get(d);
      if (!was || STRENGTH.indexOf(s) < STRENGTH.indexOf(was)) held.set(d, s);
    }
  }
  const inside = (dir: string) => held.get(dir) ?? null;
  const isOpen = (path: string) => Object.hasOwn(expanded, path) && !!expanded[path];
  const field = (path: string, depth: number): TreeRow => ({
    kind: 'new',
    path,
    name: '',
    depth,
    open: false,
    status: null,
    inside: null,
    ignored: false,
  });
  const rows: TreeRow[] = [];
  const walk = (n: Node, depth: number, prefix: string) => {
    for (const first of [...n.dirs.keys()].sort(byName)) {
      let node = n.dirs.get(first)!;
      let path = prefix + first;
      let name = first;
      // A folder that only holds a folder shares its row, unless it gets the new file.
      while (!node.files.length && node.dirs.size === 1 && path !== adding) {
        const [[child, next]] = node.dirs;
        path += '/' + child;
        name += '/' + child;
        node = next;
      }
      const open = isOpen(path);
      rows.push({ kind: 'dir', path, name, depth, open, status: null, inside: inside(path), ignored: false });
      if (!open) continue;
      if (path === adding) rows.push(field(path, depth + 1));
      walk(node, depth + 1, path + '/');
    }
    for (const path of [...n.files].sort((a, b) => byName(nameOf(a), nameOf(b)))) {
      rows.push({
        kind: 'file',
        path,
        name: nameOf(path),
        depth,
        open: false,
        status: Object.hasOwn(status, path) ? status[path] : null,
        inside: null,
        ignored: ignoredSet.has(path),
      });
    }
  };
  if (adding === '') rows.push(field('', 0));
  walk(root, 0, '');
  return rows;
}

/** The folders above `path`, outermost first. */
export function ancestors(path: string): string[] {
  const parts = path.split('/').slice(0, -1);
  return parts.map((_, i) => parts.slice(0, i + 1).join('/'));
}

type Keys = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** The key deleting the row it is on: Delete alone, and on macOS Cmd+Backspace too, as in the Finder (a MacBook has no Delete key). */
export function isDeleteKey(e: Keys, mac = IS_MAC): boolean {
  const none = !e.ctrlKey && !e.altKey && !e.shiftKey;
  return none && ((e.key === 'Delete' && !e.metaKey) || (mac && e.key === 'Backspace' && e.metaKey));
}

/** Where `path` is once `from` (a file, or a folder holding it) is renamed `to`; null when it is neither it nor in it. */
export function movedPath(path: string, from: string, to: string): string | null {
  if (path === from) return to;
  return path.startsWith(from + '/') ? to + path.slice(from.length) : null;
}
