import { EditorState } from '@codemirror/state';
import { describe, expect, it, vi } from 'vitest';
import type { SearchMatch, SearchQuery } from '../types';
import { definitionResolver, type DefinitionSource } from './definitions';
import type { NavTarget } from './goto';
import { DEFAULT_ALIASES } from './links';
import { loadLanguage } from './languages';

/** Where the click is in a text: `¦`, taken out of it. */
const CARET = '¦';

interface Clicked {
  /** The identifier the click is on. */
  word: string;
  targets: NavTarget[];
}

function source(over: Partial<DefinitionSource> = {}): DefinitionSource {
  return {
    read: async () => null,
    search: async () => ({ matches: [], truncated: false }),
    aliases: () => DEFAULT_ALIASES,
    ...over,
  };
}

/** The file `path` showing `text`, with its language, clicked where `¦` is; null where nothing is followed. */
async function click(path: string, text: string, src = source(), files: string[] = [path]): Promise<Clicked | null> {
  const pos = text.indexOf(CARET);
  const doc = text.replace(CARET, '');
  const lang = await loadLanguage(path);
  const state = EditorState.create({ doc, extensions: lang ? [lang] : [] });
  const spot = definitionResolver(src)({ state, pos, path, files });
  return spot && { word: doc.slice(spot.from, spot.to), targets: await spot.resolve() };
}

/** `text` with the caret put before the `n`-th (1-based) whole word `word` in it. */
function caret(text: string, word: string, n = 1): string {
  const m = [...text.matchAll(new RegExp(`\\b${word}\\b`, 'g'))][n - 1];
  if (!m) throw new Error(`no ${word} #${n}`);
  return text.slice(0, m.index) + CARET + text.slice(m.index);
}

describe('definitions in the file', () => {
  const TS = [
    'const top = 1;',
    'function run(n: number) {',
    '  const top = n;',
    '  return top + n;',
    '}',
    'class Box { size = 1; grow() { return this.size; } }',
    'type Size = number;',
    'const s: Size = run(top);',
    'new Box();',
    'later();',
    'function later() {}',
    'const helper = 1;',
    'function wrap() {',
    '  function helper() {}',
    '  return helper();',
    '}',
  ].join('\n');

  it.each([
    ['the nearest one before it in the scope it is used in', 'top', 3, { line: 3, col: 9 }],
    ['the one of the outer scope, an inner one not being seen there', 'top', 4, { line: 1, col: 7 }],
    ['a parameter', 'n', 3, { line: 2, col: 14 }],
    ['a function', 'run', 2, { line: 2, col: 10 }],
    ['a type', 'Size', 2, { line: 7, col: 6 }],
    ['a class', 'Box', 2, { line: 6, col: 7 }],
    ['a member of a class', 'size', 2, { line: 6, col: 13 }],
    ['the first one when none comes before it', 'later', 1, { line: 11, col: 10 }],
    ['a function declared in the block it is called from', 'helper', 3, { line: 14, col: 12 }],
  ])('finds %s in JS/TS', async (_, word, n, at) => {
    const c = await click('src/a.ts', caret(TS, word, n));
    expect(c?.word).toBe(word);
    expect(c?.targets).toEqual([{ path: 'src/a.ts', ...at }]);
  });

  it('finds a Rust function, a binding of `let` and a parameter', async () => {
    const RS = 'fn helper(x: i32) -> i32 { x }\nfn main() { let y = helper(1); println!("{}", y); }\n';
    expect((await click('src/main.rs', caret(RS, 'helper', 2)))?.targets).toMatchObject([{ line: 1, col: 4 }]);
    expect((await click('src/main.rs', caret(RS, 'y', 2)))?.targets).toMatchObject([{ line: 2, col: 17 }]);
    expect((await click('src/main.rs', caret(RS, 'x', 2)))?.targets).toMatchObject([{ line: 1, col: 11 }]);
  });

  it('finds a Python function, a class and a name given at the top of the file', async () => {
    const PY = 'LIMIT = 3\n\ndef load(path):\n    return path * LIMIT\n\nclass Store:\n    pass\n\nload(Store())\n';
    const at = async (word: string, n: number) => (await click('app/main.py', caret(PY, word, n)))?.targets;
    expect(await at('load', 2)).toMatchObject([{ path: 'app/main.py', line: 3, col: 5 }]);
    expect(await at('Store', 2)).toMatchObject([{ line: 6, col: 7 }]);
    expect(await at('LIMIT', 2)).toMatchObject([{ line: 1, col: 1 }]);
  });

  it('finds a Go function and a variable', async () => {
    const GO = 'package main\n\nfunc add(a int) int {\n\tb := a\n\treturn b\n}\n\nfunc main() { add(1) }\n';
    expect((await click('cmd/main.go', caret(GO, 'add', 2)))?.targets).toMatchObject([{ line: 3, col: 6 }]);
    expect((await click('cmd/main.go', caret(GO, 'b', 2)))?.targets).toMatchObject([{ line: 4, col: 2 }]);
  });
});

describe('definitions an import brings', () => {
  const APP =
    "import { u, v as w } from './util';\nimport D from './util';\nimport { useState } from 'react';\nconsole.log(u, w, D, useState);\n";
  const UTIL = 'export const v = 0;\nexport const u = 1;\n';
  const files = ['src/app.ts', 'src/util.ts'];
  const read = vi.fn(async (p: string) => (p === 'src/util.ts' ? UTIL : null));
  const at = async (word: string) => (await click('src/app.ts', caret(APP, word, 2), source({ read }), files))?.targets;

  it('follows a named import to the file it names, then to its definition there', async () => {
    expect(await at('u')).toEqual([{ path: 'src/util.ts', line: 2, col: 14 }]);
    expect(read).toHaveBeenCalledWith('src/util.ts');
    // Renamed: under the name the module gives it.
    expect(await at('w')).toMatchObject([{ path: 'src/util.ts', line: 1, col: 14 }]);
  });

  it('opens the file at its top when nothing there has the name, and stays on an import of a package', async () => {
    expect(await at('D')).toEqual([{ path: 'src/util.ts' }]);
    expect(await at('useState')).toMatchObject([{ path: 'src/app.ts', line: 3, col: 10 }]);
  });

  it('follows `use crate::a::B` in Rust to the module that defines it', async () => {
    const LIB =
      'mod a;\nuse crate::a::B;\nuse crate::a::{C as Sea};\nuse a::E;\nuse std::fmt::Debug;\npub fn f(b: B, c: Sea, e: E, d: &dyn Debug) {}\n';
    const A = 'use std::fmt;\n\npub struct B;\npub enum C {}\npub type E = B;\n';
    const src = source({ read: async (p) => (p === 'src/a.rs' ? A : null) });
    const rs = ['src/lib.rs', 'src/a.rs'];
    expect((await click('src/lib.rs', caret(LIB, 'B', 2), src, rs))?.targets).toEqual([{ path: 'src/a.rs', line: 3, col: 12 }]);
    expect((await click('src/lib.rs', caret(LIB, 'Sea', 2), src, rs))?.targets).toMatchObject([{ path: 'src/a.rs', line: 4, col: 10 }]);
    // Without `crate::`, from the module (Rust 2018); another crate's stays on its `use`.
    expect((await click('src/lib.rs', caret(LIB, 'E', 2), src, rs))?.targets).toMatchObject([{ path: 'src/a.rs', line: 5, col: 10 }]);
    expect((await click('src/lib.rs', caret(LIB, 'Debug', 2), src, rs))?.targets).toEqual([{ path: 'src/lib.rs', line: 5, col: 15 }]);
  });

  it('follows `use super::x` and `use self::m::x` in Rust to the modules around the file', async () => {
    const B = 'use super::helper;\nuse self::deep::Leaf;\nfn f() { helper(); Leaf; }\n';
    const texts: Record<string, string> = { 'src/a.rs': 'mod b;\n\npub fn helper() {}\n', 'src/a/b/deep.rs': 'pub struct Leaf;\n' };
    const src = source({ read: async (p) => texts[p] ?? null });
    const rs = ['src/lib.rs', 'src/a.rs', 'src/a/b.rs', 'src/a/b/deep.rs'];
    expect((await click('src/a/b.rs', caret(B, 'helper', 2), src, rs))?.targets).toEqual([{ path: 'src/a.rs', line: 3, col: 8 }]);
    expect((await click('src/a/b.rs', caret(B, 'Leaf', 2), src, rs))?.targets).toEqual([{ path: 'src/a/b/deep.rs', line: 1, col: 12 }]);
  });

  it('follows `from .models import User` in Python', async () => {
    const PY = 'from .models import User as U\n\nU()\n';
    const src = source({ read: async (p) => (p === 'pkg/models.py' ? 'class User:\n    pass\n' : null) });
    const c = await click('pkg/app.py', caret(PY, 'U', 2), src, ['pkg/app.py', 'pkg/models.py']);
    expect(c?.targets).toMatchObject([{ path: 'pkg/models.py', line: 1, col: 7 }]);
  });
});

describe('definitions searched for in the source', () => {
  const match = (path: string, line: number, text: string): SearchMatch => ({ path, line, col: 1, text });

  it('searches the files of the same language, the clicked line left out, the same file first, then its folder', async () => {
    const search = vi.fn(async (_: SearchQuery) => ({
      matches: [
        match('lib/far.c', 3, 'int render(void) {'),
        match('src/ui/app.c', 2, '  render();'),
        match('src/ui/app.py', 1, 'render = 1'),
        match('src/ui/sub/deep.c', 7, 'static int render (int x)'),
        match('src/ui/x.c', 4, 'prerender(1);'),
        match('src/ui/app.h', 5, 'int render(void);'),
        match('src/ui/app.c', 9, 'int render(void) { return 0; }'),
      ],
      truncated: false,
    }));
    const c = await click('src/ui/app.c', 'int main(void) {\n  ¦render();\n}\n', source({ search }));
    expect(search).toHaveBeenCalledExactlyOnceWith({
      pattern: 'render[[:space:]]*[:=(]',
      regex: true,
      caseSensitive: true,
      wholeWord: false,
      maxResults: 1000,
    });
    expect(c?.targets).toEqual([
      { path: 'src/ui/app.c', line: 9, col: 5, text: 'int render(void) { return 0; }' },
      { path: 'src/ui/app.h', line: 5, col: 5, text: 'int render(void);' },
      { path: 'lib/far.c', line: 3, col: 5, text: 'int render(void) {' },
      { path: 'src/ui/sub/deep.c', line: 7, col: 12, text: 'static int render (int x)' },
    ]);
  });

  it.each([
    ['src/a.ts', 'run();', String.raw`(function\*?|class|interface|type|enum|const|let|var)[[:space:]]+run([^[:alnum:]_]|$)`],
    [
      'src/a.rs',
      'fn f() { run(); }',
      String.raw`(fn|struct|enum|trait|type|mod|const|static|macro_rules!)[[:space:]]+run([^[:alnum:]_]|$)`,
    ],
    ['a.py', 'run()', String.raw`(def|class)[[:space:]]+run([^[:alnum:]_]|$)`],
    [
      'a.go',
      'package a\nfunc f() { run() }',
      String.raw`func[[:space:]]+(\([^)]*\)[[:space:]]*)?run([^[:alnum:]_]|$)|type[[:space:]]+run([^[:alnum:]_]|$)`,
    ],
  ])('asks git for what defines a name in %s, without the `\\b` and `\\s` git lacks on macOS', async (path, text, pattern) => {
    const search = vi.fn(async (_: SearchQuery) => ({
      matches: [match('lib/x' + path.slice(path.lastIndexOf('.')), 4, 'pub(crate) fn run_all() {}; fn run() {}')],
      truncated: false,
    }));
    const c = await click(path, caret(text, 'run'), source({ search }));
    expect(search.mock.calls[0][0].pattern).toBe(pattern);
    // What git finds more than the language's pattern is left out.
    expect(c?.targets.length).toBe(path.endsWith('.rs') ? 1 : 0);
  });

  it('counts Svelte and Vue files with the scripts', async () => {
    const search = vi.fn(async () => ({ matches: [match('src/B.svelte', 2, '  export let run = 1;')], truncated: false }));
    const c = await click('src/a.ts', '¦run();', source({ search }));
    expect(c?.targets).toEqual([{ path: 'src/B.svelte', line: 2, col: 14, text: '  export let run = 1;' }]);
  });
});

describe('what is followed', () => {
  it('is an identifier of code: not a word of a comment, a string, a keyword or a number, nor of prose', async () => {
    const TS = '// render here\nconst s = "render";\nconst n = 42;\nrender(s, n);\n';
    expect(await click('src/a.ts', caret(TS, 'render'))).toBeNull();
    expect(await click('src/a.ts', caret(TS, 'render', 2))).toBeNull();
    expect(await click('src/a.ts', caret(TS, 'const'))).toBeNull();
    expect(await click('src/a.ts', caret(TS, '42'))).toBeNull();
    expect((await click('src/a.ts', caret(TS, 'render', 3)))?.word).toBe('render');
    expect(await click('README.md', 'Call ¦render to draw.')).toBeNull();
    const RS = '// helper\nfn f() { let s = "helper"; helper(); }\n';
    expect(await click('src/a.rs', caret(RS, 'helper'))).toBeNull();
    expect(await click('src/a.rs', caret(RS, 'helper', 2))).toBeNull();
    expect((await click('src/a.rs', caret(RS, 'helper', 3)))?.word).toBe('helper');
  });

  it('is what the template of a string shows', async () => {
    const c = await click('src/a.ts', 'const top = 1;\nconst s = `${¦top}`;\n');
    expect(c?.targets).toMatchObject([{ line: 1, col: 7 }]);
  });
});
