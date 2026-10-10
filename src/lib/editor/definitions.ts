// Where an identifier is defined, without a language server (level B of the code navigation, tried after the links
// of `links.ts`): in its file, by the syntax tree; in the file an import of it names; else in the files of the same
// language where a search of the source finds what defines it.

import { ensureSyntaxTree, language, syntaxTree } from '@codemirror/language';
import { EditorState } from '@codemirror/state';
import { getStyleTags, tags } from '@lezer/highlight';
import { escapeRegExp } from '../format';
import type { SearchQuery, SearchResult } from '../types';
import { charColumn, nameAt, type NavContext, type NavResolver, type NavTarget } from './goto';
import { loadLanguage } from './languages';
import { dirOf, extOf, fileSet, importTarget, join, pythonModule, rustModuleHome, type Aliases } from './links';

// The tree types of @lezer/common, which only comes with CodeMirror.
type Tree = ReturnType<typeof syntaxTree>;
type SyntaxNode = ReturnType<Tree['resolveInner']>;

/** Where definitions are looked for: the files of the source shown. */
export interface DefinitionSource {
  /** The text of a file of the source (as the editor has it when it is open), null when it cannot be read. */
  read: (path: string) => Promise<string | null>;
  /** `code_search` in the source. */
  search: (q: SearchQuery) => Promise<SearchResult>;
  /** The import aliases of the source (its tsconfig.json). */
  aliases: () => Aliases;
}

/** Files whose words are prose, not names: nothing is looked for there (nor in a file without an extension). */
const PROSE = new Set(['md', 'markdown', 'mdx', 'txt', 'rst', 'adoc']);

/** The extensions whose files define names for each other (a script imports a component, C has headers); else each its own. */
const FAMILIES: Record<string, string> = {
  ...Object.fromEntries(['js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'mts', 'cts', 'svelte', 'vue'].map((e) => [e, 'js'])),
  ...Object.fromEntries(['c', 'h', 'cc', 'cpp', 'cxx', 'hpp', 'hh'].map((e) => [e, 'c'])),
  rs: 'rust',
  py: 'python',
  pyi: 'python',
  go: 'go',
};
const familyOf = (path: string) => FAMILIES[extOf(path)] ?? extOf(path);

/** What defines NAME in the files of each family (`\b` and `\s` as JavaScript reads them), and in the others. */
const DEFINES: Record<string, string> = {
  js: String.raw`\b(function\*?|class|interface|type|enum|const|let|var)\s+NAME\b`,
  rust: String.raw`\b(fn|struct|enum|trait|type|mod|const|static|macro_rules!)\s+NAME\b`,
  python: String.raw`\b(def|class)\s+NAME\b`,
  go: String.raw`\bfunc\s+(\([^)]*\)\s*)?NAME\b|\btype\s+NAME\b`,
};
const DEFINES_ELSE = String.raw`\bNAME\b\s*[:=(]`;

/** Lines asked of the search: what git finds is then checked with the family's pattern. */
const SEARCH_MAX = 1000;
/** How long a whole file may take to parse, in milliseconds (a part of a huge one is then all there is). */
const PARSE_MS = 500;

/**
 * `template` with `name` in it, as git is asked for it. Wider than the template, whose JavaScript form checks each line
 * found: git's `\b` only knows ASCII letters (`café` would never end at one). So the `\b` ending the name at the end of
 * an alternative becomes a character that is no letter, digit or `_` (or the line's end), and the other ones go.
 */
function gitPattern(template: string, name: string): string {
  return template
    .replace(/NAME\\b(?=\||$)/g, () => 'NAME(\\W|$)')
    .split('\\b')
    .join('')
    .split('NAME')
    .join(escapeRegExp(name));
}

/** A character of a name, in most languages: none may come right before or after the one looked for. */
const NAME_CHAR = String.raw`[\p{L}\p{N}_$]`;

/** `template` with `name` in it, as JavaScript reads it, its boundaries around the name knowing every letter (`café`). */
function exactPattern(template: string, name: string): RegExp {
  const bounded = template.split('\\bNAME').join(`(?<!${NAME_CHAR})NAME`).split('NAME\\b').join(`NAME(?!${NAME_CHAR})`);
  return new RegExp(bounded.split('NAME').join(escapeRegExp(name)), 'u');
}

/** What is not a name: comments, strings, numbers and other literals, keywords. */
const INERT = [tags.comment, tags.literal, tags.keyword];

/** Whether `node` is in a comment, a string, a keyword or a literal, by the highlighting of the nearest one that has some. */
function inert(node: SyntaxNode | null): boolean {
  for (let n = node; n; n = n.parent) {
    const style = getStyleTags(n);
    if (style) return style.tags.some((t) => INERT.some((i) => t.set.includes(i)));
  }
  return false;
}

/** The identifier at `pos`, as the language cuts words, unless it is in a comment, a string or a keyword. */
function identifierAt(state: EditorState, pos: number): { from: number; to: number; name: string } | null {
  const id = nameAt(state, pos);
  return id && !inert(syntaxTree(state).resolveInner(id.from, 1)) ? id : null;
}

interface Span {
  from: number;
  to: number;
}

/** What an import brings: what it is called in the module the import names (null: the module itself). */
type Imported =
  | { kind: 'js'; spec: string; name: string | null }
  /** Rust's `use crate::a::B`: its path, whose last part is the name. */
  | { kind: 'rust'; path: string[] }
  | { kind: 'python'; module: string; name: string };

/** A definition of the name looked for: where it is, the stretch it is seen in, and what brings it when an import does. */
interface Def extends Span {
  scope: Span;
  imported?: Imported;
}

/** How a language's tree defines names. */
interface Grammar {
  /** The nodes that may name a definition. */
  names: ReadonlySet<string>;
  /** The definition `n` (whose text is the name looked for) makes, if it makes one; `all` is the whole file. */
  def(n: SyntaxNode, state: EditorState, all: Span): Def | null;
}

const NONE: ReadonlySet<string> = new Set();

/**
 * The stretch the name `n` defines is seen in: the nearest of the `scopes` around it, the whole file without one. A
 * function's own name (a child of one of the `named`) is seen where the function is, not only in it.
 */
function scopeOf(n: SyntaxNode, scopes: ReadonlySet<string>, named: ReadonlySet<string>, all: Span): Span {
  let p = n.parent;
  if (p && named.has(p.name)) p = p.parent;
  for (; p; p = p.parent) if (scopes.has(p.name)) return { from: p.from, to: p.to };
  return all;
}

function ancestor(n: SyntaxNode, name: string): SyntaxNode | null {
  for (let p = n.parent; p; p = p.parent) if (p.name === name) return p;
  return null;
}

const slice = (state: EditorState, n: Span) => state.sliceDoc(n.from, n.to);
const unquote = (s: string) => s.replace(/^(['"`])(.*)\1$/s, '$2');

const JS_SCOPES = new Set([
  'Script',
  'Block',
  'ClassBody',
  'ArrowFunction',
  'FunctionDeclaration',
  'FunctionExpression',
  'MethodDeclaration',
  'ForStatement',
  'CatchClause',
]);
const JS_NAMED = new Set(['FunctionDeclaration', 'MethodDeclaration']);

/** JS and TS: variables, functions, classes, parameters, types, a class's members; the names an import brings. */
const js: Grammar = {
  names: new Set(['VariableDefinition', 'TypeDefinition', 'PropertyDefinition']),
  def(n, state, all) {
    const p = n.parent;
    if (!p) return null;
    const at = { from: n.from, to: n.to };
    if (n.name === 'PropertyDefinition') {
      // A class's only: an object's keys and a type's fields are not looked for.
      const member = p.name === 'PropertyDeclaration' || p.name === 'MethodDeclaration';
      return member ? { ...at, scope: scopeOf(n, JS_SCOPES, JS_NAMED, all) } : null;
    }
    const imp = ancestor(n, 'ImportDeclaration');
    if (!imp) return { ...at, scope: scopeOf(n, JS_SCOPES, JS_NAMED, all) };
    const spec = imp.getChild('String');
    if (!spec) return null;
    // `{ a as b }`, `* as b`: b is what the module calls a, or the module itself.
    const as = n.prevSibling?.name === 'as' ? n.prevSibling.prevSibling : null;
    const name = !as ? slice(state, n) : as.name === 'Star' ? null : unquote(slice(state, as));
    return { ...at, scope: all, imported: { kind: 'js', spec: unquote(slice(state, spec)), name } };
  },
};

const RS_SCOPES = new Set([
  'SourceFile',
  'Block',
  'DeclarationList',
  'FunctionItem',
  'ClosureExpression',
  'MatchArm',
  'IfExpression',
  'WhileExpression',
  'ForExpression',
]);
const RS_NAMED = new Set(['FunctionItem']);
/** Rust's items, and the node naming each: the first such child (`type Y = S;` holds S too). */
const RS_ITEMS: Record<string, string> = {
  FunctionItem: 'BoundIdentifier',
  ConstItem: 'BoundIdentifier',
  StaticItem: 'BoundIdentifier',
  ModItem: 'BoundIdentifier',
  DeclarativeMacroItem: 'BoundIdentifier',
  StructItem: 'TypeIdentifier',
  EnumItem: 'TypeIdentifier',
  UnionItem: 'TypeIdentifier',
  TypeItem: 'TypeIdentifier',
  TraitItem: 'TypeIdentifier',
  AssociatedType: 'TypeIdentifier',
  MacroDefinition: 'Identifier',
};

/** The path of the name `n` of a `use`, from the crate's root or the module: `use crate::a::{B as C}` gives B's. */
function usePath(n: SyntaxNode, state: EditorState): string[] {
  const p = n.parent!;
  let own =
    p.name === 'UseAsClause' && p.firstChild
      ? slice(state, p.firstChild)
      : p.name === 'ScopedIdentifier'
        ? state.sliceDoc(p.from, n.to)
        : slice(state, n);
  for (let a: SyntaxNode | null = p; a && a.name !== 'UseDeclaration'; a = a.parent) {
    const list = a.name === 'ScopedUseList' ? a.getChild('UseList') : null;
    if (list) own = state.sliceDoc(a.from, list.from) + own;
  }
  return own.replace(/\s+/g, '').split('::').filter(Boolean);
}

/** Rust: functions, structs, enums, traits, types, consts, statics, modules, macros, bindings of patterns (`let`, parameters); the names `use` brings. */
const rust: Grammar = {
  names: new Set(['BoundIdentifier', 'TypeIdentifier', 'Identifier']),
  def(n, state, all) {
    const p = n.parent;
    if (!p) return null;
    const at = { from: n.from, to: n.to, scope: scopeOf(n, RS_SCOPES, RS_NAMED, all) };
    const naming = RS_ITEMS[p.name];
    if (naming) return naming === n.name && p.getChild(naming)?.from === n.from ? at : null;
    // Elsewhere, only a pattern's names are defined there: `let`, parameters, `match` arms.
    if (n.name !== 'BoundIdentifier') return null;
    return ancestor(n, 'UseDeclaration') ? { ...at, imported: { kind: 'rust', path: usePath(n, state) } } : at;
  },
};

const PY_SCOPES = new Set(['Script', 'Body']);

/** Python: functions, classes, the names given at the top of the file; the names `from … import` brings. */
const python: Grammar = {
  names: new Set(['VariableName']),
  def(n, state, all) {
    const p = n.parent;
    if (!p) return null;
    const at = { from: n.from, to: n.to };
    if (p.name === 'FunctionDefinition' || p.name === 'ClassDefinition') {
      return p.getChild('VariableName')?.from === n.from ? { ...at, scope: scopeOf(n, PY_SCOPES, NONE, all) } : null;
    }
    if (p.name === 'AssignStatement') {
      if (p.parent?.name !== 'Script') return null;
      // The names before the last `=` (or before the type of `x: int`), not the value.
      const ops = p.getChildren('AssignOp');
      const end = ops.length ? ops[ops.length - 1].from : (p.getChild('TypeDef')?.from ?? -1);
      return n.to <= end ? { ...at, scope: all } : null;
    }
    const from = p.name === 'ImportStatement' ? p.getChild('from') : null;
    const kw = from && p.getChild('import');
    if (!from || !kw || n.from < kw.to) return null;
    const as = n.prevSibling?.name === 'as' ? n.prevSibling.prevSibling : null;
    const module = state.sliceDoc(from.to, kw.from).replace(/\s+/g, '');
    return { ...at, scope: scopeOf(n, PY_SCOPES, NONE, all), imported: { kind: 'python', module, name: slice(state, as ?? n) } };
  },
};

const GO_SCOPES = new Set(['SourceFile', 'Block', 'FunctionDecl', 'MethodDecl', 'FunctionLiteral']);
const GO_NAMED = new Set(['FunctionDecl']);

/** Go: what its tree calls `DefName` (functions, types, variables, constants, parameters), but a package's name. */
const go: Grammar = {
  names: new Set(['DefName']),
  def(n, _, all) {
    const p = n.parent?.name;
    if (p === 'PackageClause' || p === 'ImportSpec') return null;
    return { from: n.from, to: n.to, scope: scopeOf(n, GO_SCOPES, GO_NAMED, all) };
  },
};

const GRAMMARS: Record<string, Grammar> = { js, rust, python, go };

/** The definitions of `name` in `tree`, in the order of the text. */
function definitionsIn(tree: Tree, state: EditorState, g: Grammar, name: string): Def[] {
  const all = { from: 0, to: state.doc.length };
  const out: Def[] = [];
  tree.iterate({
    enter(ref) {
      if (!g.names.has(ref.name) || ref.to - ref.from !== name.length || state.sliceDoc(ref.from, ref.to) !== name) return;
      const d = g.def(ref.node, state, all);
      if (d) out.push(d);
    },
  });
  return out;
}

/** The one `pos` refers to: the nearest before it in a scope it is in, else the first in such a scope, else the first. */
function nearest(defs: Def[], pos: number): Def | null {
  const seen = defs.filter((d) => d.scope.from <= pos && pos <= d.scope.to);
  return seen.filter((d) => d.from <= pos).at(-1) ?? seen[0] ?? defs[0] ?? null;
}

/** `text` with the language of `path`, and its whole tree. */
async function parse(text: string, path: string): Promise<{ state: EditorState; tree: Tree }> {
  const lang = await loadLanguage(path);
  const state = EditorState.create({ doc: text, extensions: lang ?? [] });
  return { state, tree: ensureSyntaxTree(state, state.doc.length, PARSE_MS) ?? syntaxTree(state) };
}

/** The whole tree of the editor's `state`; parsed apart while the editor's language is still loading. */
async function treeOf(state: EditorState, path: string): Promise<Tree> {
  if (!state.facet(language)) return (await parse(state.doc.toString(), path)).tree;
  return ensureSyntaxTree(state, state.doc.length, PARSE_MS) ?? syntaxTree(state);
}

function place(path: string, state: EditorState, pos: number): NavTarget {
  const line = state.doc.lineAt(pos);
  return { path, line: line.number, col: charColumn(line.text, pos - line.from) };
}

/** A Rust module: its file, and the folder of the modules it declares. */
interface RustModule {
  file: string;
  dir: string;
}

const rustModuleOf = (file: string): RustModule => ({ file, dir: rustModuleHome(file) });

/** The root of the crate of `file`: the nearest lib.rs or main.rs above it. */
function crateRoot(file: string, files: ReadonlySet<string>): RustModule | null {
  for (let dir = dirOf(file); ; dir = dirOf(dir)) {
    const root = ['lib.rs', 'main.rs'].map((n) => join(dir, n)).find((f) => files.has(f));
    if (root) return { file: root, dir };
    if (!dir) return null;
  }
}

function rustChild(m: RustModule, name: string, files: ReadonlySet<string>): RustModule | null {
  const file = [join(m.dir, `${name}.rs`), join(m.dir, `${name}/mod.rs`)].find((f) => files.has(f));
  return file ? { file, dir: join(m.dir, name) } : null;
}

/** The module that declares `m`: the one whose folder holds it (a crate's root has none). */
function rustParent(m: RustModule, files: ReadonlySet<string>): RustModule | null {
  if (/(^|\/)(lib|main)\.rs$/.test(m.file)) return null;
  const dir = dirOf(m.dir);
  const file = [join(dir, 'mod.rs'), join(dir, 'lib.rs'), join(dir, 'main.rs'), ...(dir ? [`${dir}.rs`] : [])].find((f) => files.has(f));
  return file ? { file, dir } : null;
}

/** The file a `use` path of the file `from` takes its last part from (null for a module itself), or null for another crate's. */
function rustUse(path: string[], from: string, files: ReadonlySet<string>): { file: string; name: string | null } | null {
  const [head, ...rest] = path;
  // From the crate's root, else from the module: `self::`, `super::`, and a module it declares (Rust 2018's `a::B`).
  let m: RustModule | null = head === 'crate' ? crateRoot(from, files) : rustModuleOf(from);
  if (head !== 'crate' && head !== 'self') rest.unshift(head);
  for (; m && rest[0] === 'super'; rest.shift()) m = rustParent(m, files);
  const name = rest.pop();
  for (const part of rest) m = m && rustChild(m, part, files);
  if (!m || !name) return null;
  const sub = rustChild(m, name, files);
  return sub ? { file: sub.file, name: null } : { file: m.file, name };
}

/** Where the name an import brings is defined in the file it comes from; null when that file is not in the tree. */
async function importedFrom(i: Imported, src: DefinitionSource, ctx: NavContext): Promise<NavTarget | null> {
  const set = fileSet(ctx.files);
  const found =
    i.kind === 'js'
      ? { file: importTarget(i.spec, ctx.path, ctx.files, src.aliases()), name: i.name }
      : i.kind === 'python'
        ? { file: pythonModule(i.module, ctx.path, ctx.files), name: i.name }
        : rustUse(i.path, ctx.path, set);
  if (!found?.file || !set.has(found.file)) return null;
  const file = found.file;
  const grammar = GRAMMARS[familyOf(file)];
  const text = file === ctx.path ? ctx.state.doc.toString() : await src.read(file).catch(() => null);
  // What cannot be read or looked into opens at its top.
  if (!found.name || !grammar || text === null) return { path: file };
  const { state, tree } = await parse(text, file);
  const defs = definitionsIn(tree, state, grammar, found.name);
  // What the module defines at its top, rather than a local of the same name.
  const def = defs.find((d) => d.scope.from === 0 && d.scope.to === state.doc.length) ?? defs[0];
  return def ? place(file, state, def.from) : { path: file };
}

/** What defines `name` in the source's files of the family of the file clicked, its line left out: that file first, then its folder. */
async function searched(src: DefinitionSource, ctx: NavContext, name: string): Promise<NavTarget[]> {
  const family = familyOf(ctx.path);
  const template = DEFINES[family] ?? DEFINES_ELSE;
  const exact = exactPattern(template, name);
  const line = ctx.state.doc.lineAt(ctx.pos).number;
  const { matches } = await src.search({
    pattern: gitPattern(template, name),
    regex: true,
    caseSensitive: true,
    wholeWord: false,
    maxResults: SEARCH_MAX,
  });
  const dir = dirOf(ctx.path);
  const found: { t: NavTarget; rank: number }[] = [];
  for (const m of matches) {
    if (familyOf(m.path) !== family || (m.path === ctx.path && m.line === line)) continue;
    const x = exact.exec(m.text);
    if (!x) continue;
    // The name ends what defines it, or begins it for the other languages' `name =`; the text may start further in
    // the line.
    const col = m.offset + charColumn(m.text, x.index + x[0].lastIndexOf(name));
    found.push({ t: { path: m.path, line: m.line, col, text: m.text }, rank: m.path === ctx.path ? 0 : dirOf(m.path) === dir ? 1 : 2 });
  }
  return found.sort((a, b) => a.rank - b.rank).map((f) => f.t);
}

/** Where `name`, at `ctx.pos`, is defined: in the file, else where its import leads, else where a search finds it. */
async function definitions(src: DefinitionSource, ctx: NavContext, name: string): Promise<NavTarget[]> {
  const grammar = GRAMMARS[familyOf(ctx.path)];
  const def = grammar ? nearest(definitionsIn(await treeOf(ctx.state, ctx.path), ctx.state, grammar, name), ctx.pos) : null;
  if (!def) return searched(src, ctx, name);
  // An import of a package, or of a file the tree lacks: the import itself.
  const there = def.imported ? await importedFrom(def.imported, src, ctx) : null;
  return [there ?? place(ctx.path, ctx.state, def.from)];
}

/** The resolver of identifiers: where the one under the mouse (or the cursor) is defined. */
export function definitionResolver(src: DefinitionSource): NavResolver {
  return (ctx) => {
    const ext = extOf(ctx.path);
    if (!ext || PROSE.has(ext)) return null;
    const id = identifierAt(ctx.state, ctx.pos);
    return id && { from: id.from, to: id.to, resolve: () => definitions(src, ctx, id.name) };
  };
}
