import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { ALLOWED } from './hardcoded-allow';
import { notAllowed, scan, type Finding } from './hardcoded';

/** What a finding says, as a failure shows it. */
const said = (f: Finding) => `${f.file}:${f.line} ${f.kind} « ${f.text} »`;
const texts = (file: string, source: string) => scan(file, source).map((f) => f.text);

/**
 * What is left, under `dir`, of the lists of files each extraction task had still to do (`pending/<lot>.txt`): a
 * folder or a file whose name starts with « pending ». Every file is extracted: nothing may be left, and a list that
 * comes back would hide the texts of the files it names from the scan.
 */
function pendingLeft(dir: string, base = dir): string[] {
  const left: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (/^pending/i.test(name)) left.push(relative(base, path).replace(/\\/g, '/'));
    else if (statSync(path).isDirectory()) left.push(...pendingLeft(path, base));
  }
  return left.sort();
}

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

  it('finds a text in the props this app shows, and a sentence in any prop of a component', () => {
    const source = [
      '<Row label="Son" desc="Question de Claude, fin de tour" hint="minutes, 0 = jamais" note="pour tous les projets" />',
      '<Dropdown caption="Mode" kind="todo" class="btn ghost" size="small" />',
      '<Thing message="Rien a faire ici" icon="⚙" />',
      '<div data-tip="deux mots"></div>',
    ].join('\n');
    expect(scan('src/A.svelte', source).map((f) => [f.line, f.text])).toEqual([
      [1, 'Son'],
      [1, 'Question de Claude, fin de tour'],
      [1, 'minutes, 0 = jamais'],
      [1, 'pour tous les projets'],
      [2, 'Mode'],
      [3, 'Rien a faire ici'],
    ]);
  });

  it('finds every call to the deprecated plural(), whose words have no accent', () => {
    const component = [
      '<script lang="ts">',
      "  const sub = $derived(plural(tickets.length, 'ticket', 'tickets'));",
      '</script>',
      '',
      "<p>{plural(n, 'ligne', 'lignes')} de plus</p>",
    ].join('\n');
    expect(scan('src/A.svelte', component).map((f) => [f.kind, f.line, f.text])).toEqual([
      ['string', 2, "plural(tickets.length, 'ticket', 'tickets')"],
      ['string', 5, "plural(n, 'ligne', 'lignes')"],
      ['text', 5, 'de plus'],
    ]);
    const module = "export const loops = (t: T) => `${plural(t.loops, 'boucle', 'boucles')} · ${t.cost}`;";
    expect(texts('src/lib/board.ts', module)).toEqual(["plural(t.loops, 'boucle', 'boucles')"]);
  });

  it('finds what is left of a list of files to extract, a folder or a file, wherever it is', () => {
    const dir = mkdtempSync(join(tmpdir(), 'l12-pending-'));
    try {
      mkdirSync(join(dir, 'fr'));
      writeFileSync(join(dir, 'fr', 'board.ts'), '');
      expect(pendingLeft(dir)).toEqual([]);
      mkdirSync(join(dir, 'pending'));
      writeFileSync(join(dir, 'pending', 'l6.txt'), 'src/A.svelte\n');
      writeFileSync(join(dir, 'fr', 'pending-l7.txt'), '');
      expect(pendingLeft(dir)).toEqual(['fr/pending-l7.txt', 'pending']);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
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

describe('the sources of the app', () => {
  const files = sources();
  const findings = files.flatMap((f) => scan(f, readFileSync(join(root, f), 'utf8')));

  it('have no list of files still to extract: every file is', () => {
    expect(pendingLeft(join(root, 'src/lib/i18n'))).toEqual([]);
  });

  it('have no text written in the code, in any file', () => {
    expect(notAllowed(findings, ALLOWED).map(said)).toEqual([]);
  });

  it('read the files of the app, so that a scan of nothing cannot pass', () => {
    expect(files).toContain('src/components/StatusBar.svelte');
    expect(files).toContain('src/lib/state.svelte.ts');
    expect(files.length).toBeGreaterThan(100);
  });

  it('keep only exceptions that still let a text through, each with its reason', () => {
    for (const a of ALLOWED) {
      expect({ ...a, why: a.why.trim() !== '' }).toEqual({ ...a, why: true });
      expect({ ...a, used: notAllowed(findings, [a]).length < findings.length }).toEqual({ ...a, used: true });
    }
  });
});
