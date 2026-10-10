import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { ALLOWED } from './hardcoded-allow';
import { notAllowed, scan, type Finding } from './hardcoded';

/** What a finding says, as a failure shows it. */
const said = (f: Finding) => `${f.file}:${f.line} ${f.kind} « ${f.text} »`;
const texts = (file: string, source: string) => scan(file, source).map((f) => f.text);

describe('the detector of texts written in the code', () => {
  it('finds a text in the markup, and leaves what holds no letter', () => {
    const found = scan('src/A.svelte', '<div>\n  <p>Bonjour</p>\n  <span> · — × ⌕ 42 </span>\n</div>');
    expect(found).toEqual([{ file: 'src/A.svelte', line: 2, kind: 'text', text: 'Bonjour' }]);
  });

  it('finds a text in the attributes that are read, and an accented one in any other', () => {
    const source = '<button title="Fermer" aria-label="Fermer la vue" class="btn ghost" data-k="x">×</button>\n<Row desc="Réglé" />';
    expect(scan('src/A.svelte', source).map((f) => [f.kind, f.text, f.line])).toEqual([
      ['attribute', 'Fermer', 1],
      ['attribute', 'Fermer la vue', 1],
      ['attribute', 'Réglé', 2],
    ]);
    expect(texts('src/A.svelte', '<input placeholder="Rechercher…" /><img alt="Logo" src="x.png" />')).toEqual(['Rechercher…', 'Logo']);
  });

  it('finds a text an expression of the markup shows, not the key given to t', () => {
    const source = [
      '<p>{dirty ? "Non enregistré" : "Enregistré"}</p>',
      '<p>{name || "Sans titre"}</p>',
      '<button title={`Rechercher (${keyLabel("Ctrl+F")})`}>⌕</button>',
      '<p>{t("common.cancel")} {count} {items.join(", ")} {`${a}/${b}`}</p>',
      '<button onclick={() => (mode = "split")}>{t("common.close")}</button>',
    ].join('\n');
    expect(scan('src/A.svelte', source).map((f) => [f.line, f.text])).toEqual([
      [1, 'Non enregistré'],
      [1, 'Enregistré'],
      [2, 'Sans titre'],
      [3, 'Rechercher (${…})'],
    ]);
  });

  it('finds an accented string in a script, not in its comments, regexes or keys', () => {
    const source = [
      '<script lang="ts">',
      '  // Réglages',
      "  const label: string = 'Réglages';",
      "  const id = 'settings';",
      '  const accents = /[éè]/;',
      '  toast(`${n} élément copié`);',
      '</script>',
      '',
      "{#if ok}{@const x = 'Copié'}{x}{/if}",
    ].join('\n');
    expect(scan('src/A.svelte', source).map((f) => [f.kind, f.line, f.text])).toEqual([
      ['string', 3, 'Réglages'],
      ['string', 6, '${…} élément copié'],
      ['string', 9, 'Copié'],
    ]);
  });

  it('reads a TypeScript module the same way', () => {
    const source = "import type { X } from './types';\n\nexport const COLUMNS: X[] = [{ id: 'todo', label: 'À faire' }];\n// « À faire »\n";
    expect(scan('src/lib/board.ts', source)).toEqual([{ file: 'src/lib/board.ts', line: 3, kind: 'string', text: 'À faire' }]);
    // A module may hold « </script> » in a string: it is still read.
    expect(texts('src/lib/x.ts', "export const s = '</script>';\nexport const t = 'Fermé';")).toEqual(['Fermé']);
  });

  it('lets through the exceptions, for their file or for all', () => {
    const found = [
      ...scan('src/A.svelte', '<p>Escouade</p><p>Claude Code</p><p>Bonjour</p>'),
      ...scan('src/B.svelte', '<p>Claude Code</p>'),
    ];
    const allowed = [
      { file: '*', text: 'Escouade', why: 'nom propre' },
      { file: 'src/A.svelte', text: 'Claude Code', why: 'nom propre' },
    ];
    expect(notAllowed(found, allowed).map((f) => [f.file, f.text])).toEqual([
      ['src/A.svelte', 'Bonjour'],
      ['src/B.svelte', 'Claude Code'],
    ]);
  });
});

// --- the real sources ------------------------------------------------------------------------------------------

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const pendingDir = join(root, 'src/lib/i18n/pending');

/** The files of `src/` the detector reads: not the tests, nor their helpers, nor the catalogs. */
function sources(dir = join(root, 'src'), out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const rel = relative(root, path).replace(/\\/g, '/');
    if (rel === 'src/test' || rel === 'src/lib/i18n') continue;
    if (statSync(path).isDirectory()) sources(path, out);
    else if (/\.(svelte|ts)$/.test(name) && !/\.test\.(ts|svelte)$/.test(name) && !name.endsWith('.d.ts')) out.push(rel);
  }
  return out.sort();
}

/** The files each extraction task has still to do, from `pending/<task>.txt` (one path per line, `#` for comments). */
function pending(): Map<string, string[]> {
  const lists = new Map<string, string[]>();
  if (!existsSync(pendingDir)) return lists;
  for (const name of readdirSync(pendingDir).filter((n) => n.endsWith('.txt'))) {
    const lines = readFileSync(join(pendingDir, name), 'utf8').split(/\r?\n/);
    lists.set(
      name,
      lines.map((l) => l.trim()).filter((l) => l && !l.startsWith('#')),
    );
  }
  return lists;
}

describe('the sources of the app', () => {
  const files = sources();
  const lists = pending();
  const listed = [...lists.values()].flat();
  const findings = files.flatMap((f) => scan(f, readFileSync(join(root, f), 'utf8')));

  it('list each file still to extract once, and only files that exist', () => {
    expect(listed.filter((f, i) => listed.indexOf(f) !== i)).toEqual([]);
    expect(listed.filter((f) => !files.includes(f))).toEqual([]);
  });

  it('have no text written in the code, outside the files still to extract', () => {
    const left = notAllowed(findings, ALLOWED).filter((f) => !listed.includes(f.file));
    expect(left.map(said)).toEqual([]);
  });

  it('keep only exceptions that still let a text through, each with its reason', () => {
    for (const a of ALLOWED) {
      expect({ ...a, why: a.why.trim() !== '' }).toEqual({ ...a, why: true });
      expect({ ...a, used: notAllowed(findings, [a]).length < findings.length }).toEqual({ ...a, used: true });
    }
  });
});
