// The tab « Fichiers » of a pull request's view (ESC-467, PR-3 of the PR in Escouade, direction A of design/pr-2-3): what
// it says of the files git gives (`PrContentsFile`, the whole pull request's or one commit's). Their tree, its filter,
// those folded and why, the « Vu » kept by pull request and by commit, the language of a file, where a line opens in the
// editor. The component draws; these say.

import type { PrContentsFile, PrLine, PrViewed } from './types';

/** What the tab shows: the whole pull request, or one of its commits by its hash. */
export type FilesScope = 'pr' | { sha: string };

/** Past these lines of diff, a file is folded until « Afficher »: a lockfile or a generated one nobody reads line by line. */
export const LARGE_LINES = 400;

/** How many lines a part of a file's diff draws at once, as it comes near the screen. */
export const CHUNK_LINES = 100;

/** The height of a line of diff, in pixels: what a part not drawn yet keeps of the page. */
export const LINE_PX = 20;

/** The most pull requests whose « Vu » is kept: the one marked longest ago goes first. */
export const VIEWED_KEPT = 100;

// ---------- « Vu » ----------

/** Where a pull request's « Vu » is kept in `ui.prViewed`. */
export const viewedKey = (projectId: string, hostRepo: string | undefined, number: number) =>
  `${projectId}#${(hostRepo ?? '').toLowerCase()}#${number}`;

/**
 * A file's mark: what its diff says, folded into a short hash (FNV-1a). A commit that changes the file changes its diff,
 * hence its mark, and the file marked « Vu » before is no longer: as GitHub does.
 */
export function fileMark(f: PrContentsFile): string {
  let h = 0x811c9dc5;
  const add = (s: string) => {
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 0x01000193);
    }
    h ^= 10;
    h = Math.imul(h, 0x01000193);
  };
  add(f.status);
  add(f.previousPath ?? '');
  add(f.binary ? 'binary' : '');
  add(`${f.additions}/${f.deletions}`);
  for (const l of f.lines) if (l.kind !== '@') add(l.kind + l.text);
  return (h >>> 0).toString(36);
}

/** Whether `f` is marked « Vu » in `scope`: in the whole pull request, its mark must be the one it had then. */
export function isViewed(v: PrViewed | undefined, scope: FilesScope, f: PrContentsFile): boolean {
  if (!v) return false;
  if (scope === 'pr') return v.pr[f.path] === fileMark(f);
  return v.commits[scope.sha]?.includes(f.path) ?? false;
}

/** `v` with `f` marked « Vu » (`on`) or not, in `scope`. */
export function setViewed(v: PrViewed | undefined, scope: FilesScope, f: PrContentsFile, on: boolean, now: number): PrViewed {
  const next: PrViewed = { at: now, pr: { ...(v?.pr ?? {}) }, commits: { ...(v?.commits ?? {}) } };
  if (scope === 'pr') {
    if (on) next.pr[f.path] = fileMark(f);
    else delete next.pr[f.path];
  } else {
    const paths = (next.commits[scope.sha] ?? []).filter((p) => p !== f.path);
    if (on) paths.push(f.path);
    if (paths.length) next.commits[scope.sha] = paths;
    else delete next.commits[scope.sha];
  }
  return next;
}

/** How many of `files` are « Vu » in `scope`. */
export const viewedCount = (v: PrViewed | undefined, scope: FilesScope, files: PrContentsFile[]) =>
  files.reduce((n, f) => n + (isViewed(v, scope, f) ? 1 : 0), 0);

/** The « Vu » kept: the `VIEWED_KEPT` marked last; what is not one is dropped (a state written by hand, an older one). */
export function keptViewed(all: unknown): Record<string, PrViewed> {
  if (!all || typeof all !== 'object') return {};
  const ok = Object.entries(all as Record<string, unknown>).filter(
    (e): e is [string, PrViewed] =>
      !!e[1] &&
      typeof e[1] === 'object' &&
      typeof (e[1] as PrViewed).at === 'number' &&
      typeof (e[1] as PrViewed).pr === 'object' &&
      typeof (e[1] as PrViewed).commits === 'object',
  );
  ok.sort((a, b) => b[1].at - a[1].at);
  return Object.fromEntries(ok.slice(0, VIEWED_KEPT));
}

// ---------- folded ----------

/** Why a file is folded until « Afficher »: binary (nothing to show), generated, or large; null when it is not. */
export function foldReason(f: PrContentsFile): 'binary' | 'generated' | 'large' | null {
  if (f.binary) return 'binary';
  if (f.generated) return 'generated';
  if (f.lines.length > LARGE_LINES) return 'large';
  return null;
}

// ---------- the tree ----------

/** A row of the tree: a folder (its chain of single folders in one, `src/auth`) or a file, at its depth. */
export type TreeRow =
  | { kind: 'dir'; path: string; name: string; depth: number }
  | { kind: 'file'; path: string; name: string; depth: number; file: PrContentsFile };

interface Dir {
  dirs: Map<string, Dir>;
  files: PrContentsFile[];
}

/** Whether `path` answers `filter`: each of its words, without case, somewhere in the path. */
export function matches(path: string, filter: string): boolean {
  const words = filter.toLowerCase().split(/\s+/).filter(Boolean);
  const p = path.toLowerCase();
  return words.every((w) => p.includes(w));
}

/**
 * The tree of `files` that answer `filter`, flattened in its order: at each level the folders, then the files, by name.
 * A folder holding a single folder and no file is said with it (`src/auth`). Those under a folder in `closed` are left
 * out, the folder stays.
 */
export function fileTree(files: PrContentsFile[], filter = '', closed: Record<string, boolean> = {}): TreeRow[] {
  const root: Dir = { dirs: new Map(), files: [] };
  for (const f of files) {
    if (filter && !matches(f.path, filter)) continue;
    const parts = f.path.split('/');
    let d = root;
    for (const part of parts.slice(0, -1)) {
      let next = d.dirs.get(part);
      if (!next) d.dirs.set(part, (next = { dirs: new Map(), files: [] }));
      d = next;
    }
    d.files.push(f);
  }
  const rows: TreeRow[] = [];
  const walk = (d: Dir, prefix: string, depth: number) => {
    for (const [name0, sub0] of [...d.dirs].sort(([a], [b]) => a.localeCompare(b))) {
      let [name, sub] = [name0, sub0];
      while (sub.files.length === 0 && sub.dirs.size === 1) {
        const [n, s] = [...sub.dirs][0];
        name = `${name}/${n}`;
        sub = s;
      }
      const path = prefix + name;
      rows.push({ kind: 'dir', path, name, depth });
      if (!closed[path]) walk(sub, `${path}/`, depth + 1);
    }
    for (const f of [...d.files].sort((a, b) => a.path.localeCompare(b.path))) {
      rows.push({ kind: 'file', path: f.path, name: f.path.slice(prefix.length), depth, file: f });
    }
  };
  walk(root, '', 0);
  return rows;
}

/** The files in the tree's order (every folder open): the order of the diffs, so that the tree follows the scrolling. */
export const treeOrder = (files: PrContentsFile[], filter = '') =>
  fileTree(files, filter).flatMap((r) => (r.kind === 'file' ? [r.file] : []));

/** A status in one letter, as the tree shows it. */
export const statusLetter = (s: PrContentsFile['status']) => ({ added: 'A', modified: 'M', removed: 'D', renamed: 'R', copied: 'C' })[s];

// ---------- the lines ----------

/** The parts of a file's lines, `CHUNK_LINES` each: each is drawn as it comes near the screen. */
export function chunks(lines: PrLine[], size = CHUNK_LINES): { from: number; lines: PrLine[] }[] {
  const out: { from: number; lines: PrLine[] }[] = [];
  for (let i = 0; i < lines.length; i += size) out.push({ from: i, lines: lines.slice(i, i + size) });
  return out;
}

/**
 * The line of the file that line `i` of its diff opens at in the editor: its number in the tip; a removed line, the
 * one that now stands where it was (the next one kept or added in its hunk, else the one before). Null for a hunk's
 * head, or a file removed.
 */
export function editorLine(f: PrContentsFile, i: number): number | null {
  if (f.status === 'removed') return null;
  const l = f.lines[i];
  if (!l || l.kind === '@') return null;
  if (l.new) return l.new;
  for (let k = i + 1; k < f.lines.length && f.lines[k].kind !== '@'; k++) if (f.lines[k].new) return f.lines[k].new!;
  for (let k = i - 1; k >= 0 && f.lines[k].kind !== '@'; k--) if (f.lines[k].new) return f.lines[k].new! + 1;
  return 1;
}

/** The first line a file's diff changes, where « Ouvrir dans l'éditeur » of the file goes. */
export function firstLine(f: PrContentsFile): number | null {
  if (f.status === 'removed') return null;
  const i = f.lines.findIndex((l) => l.kind === '+' || l.kind === '-');
  return i < 0 ? null : editorLine(f, i);
}

/** The two sides of a file's lines to color, each as one text (the grammar follows a side): the tip's (kept and added
 * lines) and the base's (removed ones, with those kept around them); where each line of the diff is in its side. */
export function sides(lines: PrLine[]): { tip: string[]; base: string[]; at: { side: 'tip' | 'base'; i: number }[] } {
  const tip: string[] = [];
  const base: string[] = [];
  const at: { side: 'tip' | 'base'; i: number }[] = [];
  for (const l of lines) {
    if (l.kind === '-') {
      at.push({ side: 'base', i: base.length });
      base.push(l.text);
    } else if (l.kind === '@') {
      at.push({ side: 'tip', i: -1 });
    } else {
      if (l.kind === ' ') base.push(l.text);
      at.push({ side: 'tip', i: tip.length });
      tip.push(l.text);
    }
  }
  return { tip, base, at };
}

const EXT: Record<string, string> = {
  ts: 'ts',
  mts: 'ts',
  cts: 'ts',
  tsx: 'tsx',
  js: 'js',
  mjs: 'js',
  cjs: 'js',
  jsx: 'jsx',
  json: 'json',
  jsonc: 'jsonc',
  rs: 'rust',
  py: 'python',
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  ps1: 'powershell',
  css: 'css',
  scss: 'scss',
  html: 'html',
  htm: 'html',
  xml: 'xml',
  svg: 'xml',
  svelte: 'svelte',
  vue: 'vue',
  md: 'markdown',
  markdown: 'markdown',
  yml: 'yaml',
  yaml: 'yaml',
  toml: 'toml',
  sql: 'sql',
  go: 'go',
  java: 'java',
  kt: 'kotlin',
  kts: 'kotlin',
  c: 'c',
  h: 'c',
  cc: 'cpp',
  cpp: 'cpp',
  hpp: 'cpp',
  cs: 'csharp',
  php: 'php',
  rb: 'ruby',
  swift: 'swift',
  ini: 'ini',
  graphql: 'graphql',
  gql: 'graphql',
  prisma: 'prisma',
};

/** The language a file is colored in, by its name; null for one the app does not color. */
export function langOf(path: string): string | null {
  const name = path.split('/').pop() ?? path;
  if (/^dockerfile$/i.test(name)) return 'dockerfile';
  const dot = name.lastIndexOf('.');
  if (dot <= 0 && !name.startsWith('.')) return null;
  return EXT[name.slice(dot + 1).toLowerCase()] ?? null;
}
