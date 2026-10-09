import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { spotAt } from './goto';
import { DEFAULT_ALIASES, importTarget, linkResolvers, parseAliases, parseJsonc, type Aliases } from './links';

/**
 * The link under `|` in `text`, a file of the source at `path`: the text it underlines and where it leads, null when
 * nothing does.
 */
async function at(text: string, path: string, files: string[], aliases: Aliases = DEFAULT_ALIASES) {
  const pos = text.indexOf('|');
  const doc = text.slice(0, pos) + text.slice(pos + 1);
  const state = EditorState.create({ doc });
  const spot = spotAt(
    linkResolvers(() => aliases),
    { state, pos, path, files },
  );
  return spot ? { text: doc.slice(spot.from, spot.to), targets: await spot.resolve() } : null;
}

describe('imports of scripts', () => {
  const files = [
    'src/a/b.ts',
    'src/a/util.ts',
    'src/a/util.js',
    'src/a/types.d.ts',
    'src/a/Card.svelte',
    'src/a/data.json',
    'src/a/esm.ts',
    'src/lib/index.ts',
    'src/lib/x.ts',
    'src/widgets/index.svelte',
  ];

  it('follows a relative import, trying the extensions in order', async () => {
    expect(await at("import { a } from './ut|il';", 'src/a/b.ts', files)).toEqual({
      text: './util',
      targets: [{ path: 'src/a/util.ts' }],
    });
    expect(await at("import type { T } from './ty|pes';", 'src/a/b.ts', files)).toMatchObject({ targets: [{ path: 'src/a/types.d.ts' }] });
    expect(await at("import Card from './Ca|rd.svelte';", 'src/a/b.ts', files)).toMatchObject({ targets: [{ path: 'src/a/Card.svelte' }] });
    expect(await at("import data from './da|ta';", 'src/a/b.ts', files)).toMatchObject({ targets: [{ path: 'src/a/data.json' }] });
  });

  it('takes the index of a folder', async () => {
    expect(await at("import { x } from '../l|ib';", 'src/a/b.ts', files)).toMatchObject({ targets: [{ path: 'src/lib/index.ts' }] });
    expect(await at("import W from '../wid|gets';", 'src/a/b.ts', files)).toMatchObject({
      targets: [{ path: 'src/widgets/index.svelte' }],
    });
  });

  it('finds the TypeScript file an ES module import names by its compiled .js', async () => {
    expect(await at("import { e } from './es|m.js';", 'src/a/b.ts', files)).toMatchObject({ targets: [{ path: 'src/a/esm.ts' }] });
  });

  it('reads every form of import and export', async () => {
    const from = 'src/a/b.ts';
    for (const line of [
      "export * from './ut|il';",
      "export { a } from './ut|il';",
      "const m = await import('./ut|il');",
      "import './ut|il';",
      "const u = require('./ut|il');",
      'import { a } from "./ut|il";',
    ]) {
      expect(await at(line, from, files), line).toMatchObject({ text: './util', targets: [{ path: 'src/a/util.ts' }] });
    }
  });

  it('reads the imports of Svelte and Vue components too', async () => {
    const text = "<script>\n  import Card from './Ca|rd.svelte';\n</script>";
    expect(await at(text, 'src/a/Page.svelte', files)).toMatchObject({ targets: [{ path: 'src/a/Card.svelte' }] });
    expect(await at(text, 'src/a/Page.vue', files)).toMatchObject({ targets: [{ path: 'src/a/Card.svelte' }] });
  });

  it('leads to the path a missing file would have, for the editor to say it is not there', async () => {
    expect(await at("import { n } from './no|pe';", 'src/a/b.ts', files)).toEqual({ text: './nope', targets: [{ path: 'src/a/nope' }] });
  });

  it('leaves packages alone', async () => {
    expect(await at("import { useState } from 're|act';", 'src/a/b.ts', files)).toBeNull();
    expect(await at("import x from '@scope/|x';", 'src/a/b.ts', files)).toBeNull();
  });

  it('leaves a path climbing above the root alone', async () => {
    expect(await at("import x from '../../|x';", 'src/a.ts', ['x.ts', 'src/a.ts'])).toBeNull();
    expect(importTarget('../../x', 'src/a.ts', ['x.ts'], DEFAULT_ALIASES)).toBeNull();
  });

  it('follows the aliases of tsconfig.json, comments and trailing commas included', async () => {
    const aliases = parseAliases(`{
      // The root's aliases
      "compilerOptions": {
        /* where they start */ "baseUrl": ".",
        "paths": {
          "@/*": ["src/*"],
          "~utils": ["src/a/util.ts",],
          "~gone": ["src/gone.ts"],
        },
      },
    }`);
    expect(await at("import { x } from '@/lib/|x';", 'src/a/b.ts', files, aliases)).toEqual({
      text: '@/lib/x',
      targets: [{ path: 'src/lib/x.ts' }],
    });
    expect(await at("import { u } from '~ut|ils';", 'src/a/b.ts', files, aliases)).toMatchObject({ targets: [{ path: 'src/a/util.ts' }] });
    // An alias of one name leads to its file even when it is missing, for the editor to say so.
    expect(await at("import { g } from '~go|ne';", 'src/a/b.ts', files, aliases)).toMatchObject({ targets: [{ path: 'src/gone.ts' }] });
  });

  it('leaves to the packages what a wildcard alias does not find, as TypeScript does', async () => {
    const aliases = parseAliases('{ "compilerOptions": { "paths": { "*": ["types/*"], "@/*": ["src/*"] } } }');
    const typed = [...files, 'types/foo.d.ts'];
    expect(await at("import React from 're|act';", 'src/a/b.ts', typed, aliases)).toBeNull();
    expect(await at("import { m } from '@/mis|sing';", 'src/a/b.ts', typed, aliases)).toBeNull();
    expect(await at("import { f } from 'fo|o';", 'src/a/b.ts', typed, aliases)).toMatchObject({ targets: [{ path: 'types/foo.d.ts' }] });
  });

  it('resolves from baseUrl what the tree has, and leaves the rest to the packages', async () => {
    const aliases = parseAliases('{ "compilerOptions": { "baseUrl": "src" } }');
    expect(await at("import { x } from 'lib/|x';", 'src/a/b.ts', files, aliases)).toMatchObject({ targets: [{ path: 'src/lib/x.ts' }] });
    expect(await at("import React from 're|act';", 'src/a/b.ts', files, aliases)).toBeNull();
  });

  it("knows SvelteKit's $lib", async () => {
    expect(await at("import { x } from '$lib/|x';", 'src/routes/+page.svelte', files)).toMatchObject({
      targets: [{ path: 'src/lib/x.ts' }],
    });
    expect(await at("import { x } from '$l|ib';", 'src/routes/+page.ts', files)).toMatchObject({ targets: [{ path: 'src/lib/index.ts' }] });
  });

  it('keeps the default aliases when the tsconfig cannot be read', () => {
    expect(parseAliases('{ not json')).toEqual(DEFAULT_ALIASES);
  });
});

describe('parseJsonc', () => {
  it('keeps what looks like a comment inside a string', () => {
    expect(parseJsonc('{ "a": "http://x/*y*/", /* c */ "b": [1, 2,], }')).toEqual({ a: 'http://x/*y*/', b: [1, 2] });
  });
});

describe('links of style sheets', () => {
  const files = ['src/styles/base.css', 'src/styles/_vars.scss', 'src/img/logo.png', 'src/app.css'];

  it('follows @import and url() from the sheet', async () => {
    expect(await at("@import './ba|se.css';", 'src/styles/main.css', files)).toMatchObject({
      text: './base.css',
      targets: [{ path: 'src/styles/base.css' }],
    });
    expect(await at('@import url("ba|se.css");', 'src/styles/main.css', files)).toMatchObject({
      targets: [{ path: 'src/styles/base.css' }],
    });
    expect(await at('.logo { background: url(../img/lo|go.png); }', 'src/styles/main.css', files)).toMatchObject({
      text: '../img/logo.png',
      targets: [{ path: 'src/img/logo.png' }],
    });
  });

  it('finds the partial a SCSS import names', async () => {
    expect(await at("@use 'va|rs';", 'src/styles/main.scss', files)).toMatchObject({ targets: [{ path: 'src/styles/_vars.scss' }] });
    expect(await at("@import 'va|rs';", 'src/styles/main.scss', files)).toMatchObject({ targets: [{ path: 'src/styles/_vars.scss' }] });
  });

  it('leaves addresses with a scheme alone', async () => {
    expect(await at("@import url('https://fonts.example/c|ss');", 'src/styles/main.css', files)).toBeNull();
    expect(await at('a { background: url(data:image/png;ba|se64,AAAA); }', 'src/styles/main.css', files)).toBeNull();
  });
});

describe('Rust modules', () => {
  const files = ['src/lib.rs', 'src/a.rs', 'src/a/x.rs', 'src/x.rs', 'src/net/mod.rs', 'src/net/http/mod.rs'];

  it('finds a module declared in lib.rs, main.rs or mod.rs next to it', async () => {
    expect(await at('mod |x;', 'src/lib.rs', files)).toEqual({ text: 'x', targets: [{ path: 'src/x.rs' }] });
    expect(await at('pub mod n|et;', 'src/lib.rs', files)).toMatchObject({ targets: [{ path: 'src/net/mod.rs' }] });
    expect(await at('pub(crate) mod ht|tp;', 'src/net/mod.rs', files)).toMatchObject({ targets: [{ path: 'src/net/http/mod.rs' }] });
  });

  it("finds a module declared in another file in that file's folder", async () => {
    expect(await at('mod |x;', 'src/a.rs', files)).toMatchObject({ targets: [{ path: 'src/a/x.rs' }] });
  });

  it('leads to where a missing module would be', async () => {
    expect(await at('#[cfg(test)]\nmod te|sts;', 'src/a.rs', files)).toMatchObject({ targets: [{ path: 'src/a/tests.rs' }] });
  });

  it('leaves a module written in place alone', async () => {
    expect(await at('mod te|sts {', 'src/lib.rs', files)).toBeNull();
  });
});

describe('Python imports', () => {
  const files = [
    'pkg/__init__.py',
    'pkg/x.py',
    'pkg/sub/m.py',
    'pkg/sub/__init__.py',
    'pkg/x/y.py',
    'pkg/models/__init__.py',
    'app/main.py',
  ];

  it('follows relative imports from the package of the file', async () => {
    expect(await at('from .|x import y', 'pkg/m.py', files)).toEqual({ text: '.x', targets: [{ path: 'pkg/x.py' }] });
    expect(await at('from ..x.|y import z', 'pkg/sub/m.py', files)).toMatchObject({ text: '..x.y', targets: [{ path: 'pkg/x/y.py' }] });
    expect(await at('from ..mod|els import M', 'pkg/sub/m.py', files)).toMatchObject({ targets: [{ path: 'pkg/models/__init__.py' }] });
  });

  it("follows the source's own modules from its root, and leaves the others alone", async () => {
    expect(await at('import pkg.s|ub.m', 'app/main.py', files)).toMatchObject({ text: 'pkg.sub.m', targets: [{ path: 'pkg/sub/m.py' }] });
    expect(await at('import os, pkg.|x as px', 'app/main.py', files)).toMatchObject({ text: 'pkg.x', targets: [{ path: 'pkg/x.py' }] });
    expect(await at('from pk|g.x import y', 'app/main.py', files)).toMatchObject({ targets: [{ path: 'pkg/x.py' }] });
    expect(await at('import o|s', 'app/main.py', files)).toBeNull();
  });

  it('leaves a relative import climbing above the root alone', async () => {
    expect(await at('from ...|x import y', 'pkg/m.py', files)).toBeNull();
  });
});

describe('links of Markdown and HTML', () => {
  const files = ['docs/guide.md', 'docs/img/a.png', 'README.md', 'index.html', 'src/main.ts', 'src/style.css'];

  it('follows the relative links of a Markdown file', async () => {
    expect(await at('See [the guide](./gu|ide.md).', 'docs/intro.md', files)).toEqual({
      text: './guide.md',
      targets: [{ path: 'docs/guide.md' }],
    });
    expect(await at('![a](img/|a.png "A")', 'docs/intro.md', files)).toMatchObject({ targets: [{ path: 'docs/img/a.png' }] });
    expect(await at('[back](../READ|ME.md#install)', 'docs/intro.md', files)).toMatchObject({ targets: [{ path: 'README.md' }] });
    expect(await at('[guide]: ./gu|ide.md', 'docs/intro.md', files)).toMatchObject({ targets: [{ path: 'docs/guide.md' }] });
  });

  it('follows src and href of HTML, from the root when they start with /', async () => {
    expect(await at('<script type="module" src="/src/ma|in.ts"></script>', 'index.html', files)).toMatchObject({
      text: '/src/main.ts',
      targets: [{ path: 'src/main.ts' }],
    });
    expect(await at('<link rel="stylesheet" href="src/sty|le.css">', 'index.html', files)).toMatchObject({
      targets: [{ path: 'src/style.css' }],
    });
  });

  it('leaves addresses with a scheme, anchors and missing pages from the root alone', async () => {
    expect(await at('[site](https://exam|ple.com)', 'docs/intro.md', files)).toBeNull();
    expect(await at('<a href="mailto:me@exa|mple.com">', 'index.html', files)).toBeNull();
    expect(await at('<a href="#ins|tall">', 'index.html', files)).toBeNull();
    expect(await at('<a href="/ab|out">', 'index.html', files)).toBeNull();
  });
});

describe('paths anywhere', () => {
  const files = ['src/lib/x.ts', 'src/a.ts', 'notes/todo.txt', 'README.md'];

  it('follows a path the tree has, in any file', async () => {
    expect(await at('The bug is in src/l|ib/x.ts, see there.', 'notes/todo.txt', files)).toEqual({
      text: 'src/lib/x.ts',
      targets: [{ path: 'src/lib/x.ts' }],
    });
    expect(await at('// moved from src/|a.ts', 'src/lib/x.ts', files)).toMatchObject({ targets: [{ path: 'src/a.ts' }] });
    expect(await at('See READ|ME.md.', 'notes/todo.txt', files)).toMatchObject({ text: 'README.md', targets: [{ path: 'README.md' }] });
  });

  it('takes the line and column written after the path', async () => {
    expect(await at('error at src/|a.ts:42:7', 'notes/todo.txt', files)).toEqual({
      text: 'src/a.ts:42:7',
      targets: [{ path: 'src/a.ts', line: 42, col: 7 }],
    });
    expect(await at('src/a.ts:4|2', 'notes/todo.txt', files)).toEqual({ text: 'src/a.ts:42', targets: [{ path: 'src/a.ts', line: 42 }] });
  });

  it("reads a path from the file's folder and with Windows separators", async () => {
    expect(await at('see ./to|do.txt', 'notes/plan.md', files)).toMatchObject({ targets: [{ path: 'notes/todo.txt' }] });
    expect(await at('src\\lib\\|x.ts', 'notes/todo.txt', files)).toMatchObject({ targets: [{ path: 'src/lib/x.ts' }] });
  });

  it('leaves a path the tree does not have alone', async () => {
    expect(await at('src/lib/y|.ts', 'notes/todo.txt', files)).toBeNull();
    expect(await at('../../|x.ts', 'notes/todo.txt', files)).toBeNull();
    expect(await at('just w|ords', 'notes/todo.txt', files)).toBeNull();
  });
});
