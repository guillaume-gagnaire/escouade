// Exporte du catalogue d'Escouade (`src/lib/i18n`) les textes que la maquette montre, pour les techniques qui ne lisent
// pas le TypeScript (Flutter : un asset JSON). Tauri, Capacitor et React Native appellent `t()` d'Escouade directement.
//
//   node mobile-study/shared/texts/export-texts.mjs   (esbuild vient du node_modules du dépôt)
import { build } from 'esbuild';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '../../..');
export const KEYS = [
  'composer.permission.title',
  'composer.permission.allow',
  'composer.permission.deny',
  'composer.permission.allowed',
  'composer.permission.denied',
  'nav.titleBar.waiting',
  'conv.markdown.plainText',
  'common.copy',
];

const dir = mkdtempSync(join(tmpdir(), 'esc445-texts-'));
const entry = join(dir, 'entry.ts');
const i18n = join(root, 'src/lib/i18n/index.ts').replace(/\\/g, '/');
writeFileSync(entry, `export { tIn } from '${i18n}';`);
const out = join(dir, 'out.mjs');
// Le catalogue lit `$state` (une rune de Svelte) dans locale.svelte.ts : hors de Svelte, une simple valeur suffit.
await build({
  entryPoints: [entry],
  bundle: true,
  format: 'esm',
  platform: 'node',
  outfile: out,
  logLevel: 'error',
  banner: { js: 'globalThis.$state = (v) => v;' },
});
const { tIn } = await import(pathToFileURL(out).href);
const texts = {};
for (const lang of ['fr', 'en']) {
  texts[lang] = {};
  for (const k of KEYS) texts[lang][k] = tIn(lang, k, { engine: '{engine}' });
}
rmSync(dir, { recursive: true, force: true });
writeFileSync(join(here, 'escouade-texts.json'), JSON.stringify(texts, null, 2) + '\n');
console.log(texts.fr);
