// Where the links of a file lead, worked out from the source's tree alone (level A of the code navigation, without a
// language server): the imports of scripts and style sheets, Rust's `mod x;`, Python's imports, the relative links of
// Markdown and HTML and, in any file, a path the tree has (with `:line:col` after it).

import type { EditorState } from '@codemirror/state';
import type { NavResolver, NavSpot, NavTarget } from './goto';

/** The import aliases of a tsconfig.json or jsconfig.json, their targets from the source's root. */
export interface Aliases {
  /** `compilerOptions.paths`: a pattern (with at most one `*`) and where it leads, in order. */
  paths: { pattern: string; targets: string[] }[];
  /** `compilerOptions.baseUrl` from the source's root ('' for the root itself), null without one. */
  baseUrl: string | null;
}

/** SvelteKit's `$lib`, declared in a tsconfig it generates outside of the tree. */
const SVELTEKIT = [
  { pattern: '$lib', targets: ['src/lib'] },
  { pattern: '$lib/*', targets: ['src/lib/*'] },
];

export const DEFAULT_ALIASES: Aliases = { paths: SVELTEKIT, baseUrl: null };

/** JSON as tsconfig files write it, with comments and trailing commas. Throws on what is not JSON still. */
export function parseJsonc(text: string): unknown {
  let out = '';
  // A comma is held back until what follows shows it is not a trailing one.
  let comma = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '/' && text[i + 1] === '/') {
      const end = text.indexOf('\n', i);
      i = end < 0 ? text.length : end - 1;
    } else if (c === '/' && text[i + 1] === '*') {
      const end = text.indexOf('*/', i + 2);
      i = end < 0 ? text.length : end + 1;
    } else if (c === ' ' || c === '\t' || c === '\n' || c === '\r') {
      out += c;
    } else {
      if (comma && c !== '}' && c !== ']') out += ',';
      comma = c === ',';
      if (c === '"') {
        let j = i + 1;
        while (j < text.length && text[j] !== '"') j += text[j] === '\\' ? 2 : 1;
        out += text.slice(i, j + 1);
        i = j;
      } else if (!comma) {
        out += c;
      }
    }
  }
  return JSON.parse(out);
}

/** The aliases of a tsconfig.json or jsconfig.json at the source's root; the default ones when it cannot be read. */
export function parseAliases(text: string): Aliases {
  let config: unknown;
  try {
    config = parseJsonc(text);
  } catch {
    return DEFAULT_ALIASES;
  }
  const o = (config as { compilerOptions?: { baseUrl?: unknown; paths?: unknown } } | null)?.compilerOptions;
  const baseUrl = typeof o?.baseUrl === 'string' ? normalize(o.baseUrl) : null;
  const paths: Aliases['paths'] = [];
  if (o?.paths && typeof o.paths === 'object') {
    for (const [pattern, to] of Object.entries(o.paths)) {
      if (!Array.isArray(to)) continue;
      // Without baseUrl, TypeScript reads them from the tsconfig's folder: the root.
      const targets = to.filter((t) => typeof t === 'string').map((t: string) => normalize(join(baseUrl ?? '', t)));
      paths.push({ pattern, targets: targets.filter((t) => t !== null) });
    }
  }
  return { paths: [...paths, ...SVELTEKIT.filter((d) => !paths.some((p) => p.pattern === d.pattern))], baseUrl };
}

/** `path` with its `.` and `..` worked out, from the source's root; null when it climbs above the root. */
export function normalize(path: string): string | null {
  const out: string[] = [];
  for (const s of path.split('/')) {
    if (s === '' || s === '.') continue;
    if (s !== '..') out.push(s);
    else if (!out.length) return null;
    else out.pop();
  }
  return out.join('/');
}

const dirOf = (path: string) => path.slice(0, Math.max(0, path.lastIndexOf('/')));
const join = (dir: string, rel: string) => (dir ? `${dir}/${rel}` : rel);
const extOf = (path: string) => /\.([^./]+)$/.exec(path)?.[1].toLowerCase() ?? '';

const sets = new WeakMap<readonly string[], ReadonlySet<string>>();

/** The tree's files as a set, made once per tree: a big one is looked up at each move of the mouse. */
function fileSet(files: readonly string[]): ReadonlySet<string> {
  let s = sets.get(files);
  if (!s) sets.set(files, (s = new Set(files)));
  return s;
}

/** What an import without its extension may name, in the order bundlers try them. */
const EXTS = ['.ts', '.tsx', '.d.ts', '.js', '.jsx', '.mjs', '.cjs', '.svelte', '.vue', '.json'];
/** The TypeScript sources an ES module import names by the file they compile to. */
const COMPILED: Record<string, string[]> = { '.js': ['.ts', '.tsx'], '.jsx': ['.tsx'], '.mjs': ['.mts'], '.cjs': ['.cts'] };

/** The file the module `base` is: itself, with an extension, its TypeScript source, or the index of its folder. */
function moduleFile(base: string, files: ReadonlySet<string>): string | null {
  const candidates = [base, ...EXTS.map((e) => base + e)];
  const m = /\.[cm]?jsx?$/.exec(base);
  if (m) candidates.push(...(COMPILED[m[0]] ?? []).map((e) => base.slice(0, m.index) + e));
  candidates.push(...EXTS.map((e) => join(base, `index${e}`)));
  return candidates.find((f) => files.has(f)) ?? null;
}

/**
 * Where the alias matching `spec` leads (an exact one first, then the longest prefix, as TypeScript does); undefined
 * without one. An exact alias whose targets the tree lacks gives its first one, for the editor to say it is missing; a
 * wildcard one finding nothing gives undefined: TypeScript then looks for a package, and `"*": ["types/*"]` must not
 * make `react` a missing file.
 */
function aliasTarget(spec: string, aliases: Aliases, files: ReadonlySet<string>): string | null | undefined {
  let best: { star: string; targets: string[]; rank: number } | null = null;
  for (const p of aliases.paths) {
    const i = p.pattern.indexOf('*');
    let star = '';
    let rank = Infinity;
    if (i < 0) {
      if (p.pattern !== spec) continue;
    } else {
      const pre = p.pattern.slice(0, i);
      const post = p.pattern.slice(i + 1);
      if (spec.length < pre.length + post.length || !spec.startsWith(pre) || !spec.endsWith(post)) continue;
      star = spec.slice(pre.length, spec.length - post.length);
      rank = pre.length;
    }
    if (!best || rank > best.rank) best = { star, targets: p.targets, rank };
  }
  if (!best) return undefined;
  const star = best.star;
  // A function, not a string: `$&` in the import must stay as written.
  const bases = best.targets.map((t) => normalize(t.replace('*', () => star))).filter((t) => t !== null);
  for (const b of bases) {
    const f = moduleFile(b, files);
    if (f) return f;
  }
  return best.rank === Infinity ? (bases[0] ?? null) : undefined;
}

/**
 * The file the import `spec` of the file `from` names, from the source's root. One the tree lacks gives the path it
 * would have, for the editor to say so; null for a package, or for a path above the root.
 */
export function importTarget(spec: string, from: string, files: readonly string[], aliases: Aliases): string | null {
  const set = fileSet(files);
  if (/^\.\.?(\/|$)/.test(spec)) {
    const base = normalize(join(dirOf(from), spec));
    return base ? (moduleFile(base, set) ?? base) : null;
  }
  if (spec.startsWith('/')) return null;
  const aliased = aliasTarget(spec, aliases, set);
  if (aliased !== undefined) return aliased;
  // From baseUrl, only what the tree has: the rest are packages.
  const base = aliases.baseUrl === null ? null : normalize(join(aliases.baseUrl, spec));
  return base ? moduleFile(base, set) : null;
}

/** Read each side of the position at most: a minified file's single line is not scanned whole at each mouse move. */
const REACH = 1000;

interface Hit {
  from: number;
  to: number;
  text: string;
  m: RegExpExecArray;
}

/** The match of `re` (flags `dg`) on the line of `pos` whose group `g` covers `pos`, in positions of the document. */
function hitAt(state: EditorState, pos: number, re: RegExp, g: number): Hit | null {
  const line = state.doc.lineAt(pos);
  const start = Math.max(line.from, pos - REACH);
  const text = state.sliceDoc(start, Math.min(line.to, pos + REACH));
  for (const m of text.matchAll(re)) {
    const span = m.indices?.[g];
    if (!span) continue;
    const from = start + span[0];
    const to = start + span[1];
    if (from <= pos && pos <= to) return { from, to, text: m[g], m };
  }
  return null;
}

const spot = (h: Hit, target: NavTarget): NavSpot => ({ from: h.from, to: h.to, resolve: async () => [target] });

const SCRIPTS = new Set(['js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs', 'mts', 'cts', 'svelte', 'vue']);
const IMPORTS = [
  // import … from '…', export … from '…'
  /\bfrom\s*(['"])([^'"\n]*)\1/dg,
  // import '…', import('…')
  /\bimport\s*\(?\s*(['"])([^'"\n]*)\1/dg,
  /\brequire\s*\(\s*(['"])([^'"\n]*)\1/dg,
];

function importResolver(aliases: () => Aliases): NavResolver {
  return ({ state, pos, path, files }) => {
    if (!SCRIPTS.has(extOf(path))) return null;
    for (const re of IMPORTS) {
      const h = hitAt(state, pos, re, 2);
      if (!h) continue;
      const t = importTarget(h.text, path, files, aliases());
      return t === null ? null : spot(h, { path: t });
    }
    return null;
  };
}

/** A scheme (`https:`, `mailto:`, `data:`, `sass:`…) or `//`: an address, not a file. */
const ADDRESS = /^([a-z][a-z\d+.-]*:|\/\/)/i;

/**
 * The file a relative link of the file `from` names (its `?query` and `#anchor` dropped), from the source's root: the
 * first of the `variants` of its path the tree has, else the path itself, for the editor to say it is missing. A link
 * from the root (`/x`) only leads to a file the tree has: a site may serve it from elsewhere. Null for an address, an
 * anchor or a template.
 */
function linkTarget(url: string, from: string, files: ReadonlySet<string>, variants = (base: string) => [base]): string | null {
  let path = url.replace(/[?#].*$/, '');
  try {
    path = decodeURI(path);
  } catch {
    // Kept as written.
  }
  if (!path || ADDRESS.test(url) || /^[~$@{<]/.test(path)) return null;
  const rooted = path.startsWith('/');
  const base = normalize(rooted ? path : join(dirOf(from), path));
  if (!base) return null;
  return variants(base).find((f) => files.has(f)) ?? (rooted ? null : base);
}

/** Sass's partials: `@use 'vars'` is `_vars.scss`, its extension left out. */
function sassVariants(base: string): string[] {
  const dir = dirOf(base);
  const name = base.slice(dir ? dir.length + 1 : 0);
  const out = [base];
  for (const e of ['.scss', '.sass', '.css']) out.push(join(dir, `_${name}${e}`), base + e);
  return [...out, join(base, '_index.scss'), join(base, 'index.scss')];
}

const STYLES = new Set(['css', 'scss', 'sass', 'less', 'html', 'htm', 'svelte', 'vue']);
const STYLE_IMPORT = /@(?:import|use|forward)\s+(?:url\(\s*)?(['"]?)([^'"\s;)]+)\1/dg;
const STYLE_URL = /\burl\(\s*(['"]?)([^'"\s)]+)\1\s*\)/dg;

const styleResolver: NavResolver = ({ state, pos, path, files }) => {
  if (!STYLES.has(extOf(path))) return null;
  for (const re of [STYLE_IMPORT, STYLE_URL]) {
    const h = hitAt(state, pos, re, 2);
    if (!h) continue;
    const t = linkTarget(h.text, path, fileSet(files), re === STYLE_IMPORT ? sassVariants : undefined);
    return t === null ? null : spot(h, { path: t });
  }
  return null;
};

const MOD = /\bmod\s+([A-Za-z_]\w*)\s*;/dg;

const rustResolver: NavResolver = ({ state, pos, path, files }) => {
  if (extOf(path) !== 'rs') return null;
  const h = hitAt(state, pos, MOD, 1);
  if (!h) return null;
  const dir = dirOf(path);
  const name = path.slice(dir ? dir.length + 1 : 0);
  // A crate's root and a mod.rs hold their modules beside them; another file, in the folder of its own name.
  const home = /^(mod|lib|main)\.rs$/.test(name) ? dir : join(dir, name.slice(0, -'.rs'.length));
  const candidates = [join(home, `${h.text}.rs`), join(home, `${h.text}/mod.rs`)];
  const set = fileSet(files);
  return spot(h, { path: candidates.find((f) => set.has(f)) ?? candidates[0] });
};

const PY_FROM = /\bfrom\s+(\.+[\w.]*|[A-Za-z_][\w.]*)\s+import\b/dg;
/** The modules of `import a.b, c as d`. */
const PY_IMPORT = /(?:\bimport\s+|,\s*)([A-Za-z_][\w.]*)/dg;

/**
 * The file of the Python module `mod` imported by the file `from`: relative to its package with dots, else from the
 * source's root. A relative one the tree lacks gives the path it would have; an absolute one is a package's.
 */
function pyTarget(mod: string, from: string, files: ReadonlySet<string>): string | null {
  const dots = /^\.*/.exec(mod)![0].length;
  const name = mod.slice(dots).split('.').filter(Boolean).join('/');
  let base = '';
  if (dots) {
    const parts = dirOf(from).split('/').filter(Boolean);
    // `.` is the file's package, each further dot the package above.
    if (dots - 1 > parts.length) return null;
    base = parts.slice(0, parts.length - (dots - 1)).join('/');
  }
  const candidates = name ? [`${join(base, name)}.py`, join(base, `${name}/__init__.py`)] : [join(base, '__init__.py')];
  return candidates.find((f) => files.has(f)) ?? (dots ? candidates[0] : null);
}

const pythonResolver: NavResolver = ({ state, pos, path, files }) => {
  const ext = extOf(path);
  if (ext !== 'py' && ext !== 'pyi') return null;
  const line = state.doc.lineAt(pos);
  const imports = /^\s*import\s/.test(state.sliceDoc(line.from, Math.min(line.to, line.from + 100)));
  const h = hitAt(state, pos, PY_FROM, 1) ?? (imports ? hitAt(state, pos, PY_IMPORT, 1) : null);
  if (!h) return null;
  const t = pyTarget(h.text, path, fileSet(files));
  return t === null ? null : spot(h, { path: t });
};

const MARKDOWN = new Set(['md', 'markdown', 'mdx']);
const MARKUP = new Set(['html', 'htm', 'svelte', 'vue', ...MARKDOWN]);
const MD_LINKS = [
  // [text](url "title"), ![alt](url)
  /\]\(\s*<?([^)\s>]+)>?(?:\s+(?:"[^"\n]*"|'[^'\n]*'))?\s*\)/dg,
  // [ref]: url
  /^\s*\[[^\]\n]+\]:\s*<?([^\s>]+)>?/dg,
];
const ATTRS = /\b(?:src|href)\s*=\s*(["'])([^"'\n]*)\1/dg;

const markupResolver: NavResolver = ({ state, pos, path, files }) => {
  const ext = extOf(path);
  if (!MARKUP.has(ext)) return null;
  const forms: [RegExp, number][] = [...(MARKDOWN.has(ext) ? MD_LINKS.map((re): [RegExp, number] => [re, 1]) : []), [ATTRS, 2]];
  for (const [re, g] of forms) {
    const h = hitAt(state, pos, re, g);
    if (!h) continue;
    const t = linkTarget(h.text, path, fileSet(files));
    return t === null ? null : spot(h, { path: t });
  }
  return null;
};

/** A path as texts write it, with an optional `:line` or `:line:col` after it. */
const PATH = /([\p{L}\p{N}_.\-/\\@+~$%]+)(?::(\d+)(?::(\d+))?)?/dgu;

/** The tree's file a path written in a text names: from the root, else from the file's folder (only that with `./`). */
function treePath(name: string, from: string, files: ReadonlySet<string>): string | null {
  const local = normalize(join(dirOf(from), name));
  const inTree = (p: string | null) => (p && files.has(p) ? p : null);
  if (/^\.\.?\//.test(name)) return inTree(local);
  return inTree(normalize(name)) ?? inTree(local);
}

const pathResolver: NavResolver = ({ state, pos, path, files }) => {
  const h = hitAt(state, pos, PATH, 0);
  if (!h) return null;
  const set = fileSet(files);
  const [, written, line, col] = h.m;
  const name = written.replace(/\\/g, '/');
  const file = treePath(name, path, set);
  if (file) {
    const target: NavTarget = { path: file };
    if (line) target.line = Number(line);
    if (col) target.col = Number(col);
    return spot(h, target);
  }
  // A path that ends a sentence: "see src/a.ts."
  const bare = name.replace(/\.+$/, '');
  const shorter = bare !== name ? treePath(bare, path, set) : null;
  const to = h.from + bare.length;
  return shorter && pos <= to ? spot({ ...h, to }, { path: shorter }) : null;
};

/** The resolvers of links, in the order they are tried; `aliases` are read at each use (they load with the tree). */
export function linkResolvers(aliases: () => Aliases): NavResolver[] {
  return [importResolver(aliases), styleResolver, rustResolver, pythonResolver, markupResolver, pathResolver];
}
