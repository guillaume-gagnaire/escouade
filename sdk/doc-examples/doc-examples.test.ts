// @vitest-environment node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { validateText } from '../../schemas/plugin-manifest.mjs';

// Every example of code of the plugins' pages compiles or passes the manifest's checks (ESC-342). What the pages quote from
// these two plugins is held to their files by website/tests/docs.test.ts; `rust/` is compiled and benched by
// sdk/scripts/template-ci.mjs, on the crates of crates.io, which a unit test here should not download.

const ROOT = path.join(__dirname, '..', '..');
const PAGES = ['fr', 'en'].flatMap((lang) => {
  const folder = path.join(ROOT, 'website', 'content', 'docs', lang, 'plugins');
  return fs.readdirSync(folder).map((file) => ({
    page: `${lang}/${file}`,
    text: fs.readFileSync(path.join(folder, file), 'utf8').replace(/\r\n/g, '\n'),
  }));
});

/** Each fenced block of a page: its language, its code, and the line before it. */
const blocks = (text: string) =>
  [...text.matchAll(/^(.*)\n\n```(\w*)\n([\s\S]*?)\n```$/gm)].map(([, before, lang, code]) => ({ before, lang, code }));
const QUOTED = /^`(?:plugins|sdk)\/[^`\s]+`/;

/** A piece of a manifest, its top-level keys written as they are: an object, or `null` for what is none. */
function fragment(code: string): Record<string, unknown> | null {
  for (const text of [code, `{${code}}`, `{${code.replace(/,\s*$/, '')}}`]) {
    try {
      const value: unknown = JSON.parse(text);
      if (value && typeof value === 'object' && !Array.isArray(value)) return value as Record<string, unknown>;
    } catch {
      // The next form.
    }
  }
  return null;
}

/** What a manifest's piece is when it is one: what a whole manifest around it would be. */
function asManifest(piece: Record<string, unknown>): Record<string, unknown> | null {
  const base = { id: 'dev.example.page', name: 'Page', version: '1.0.0', escouade: { api: '1', min: '2.2.0' } };
  if ('id' in piece && 'escouade' in piece) return piece;
  if ('accounts' in piece || 'workflowReaders' in piece) return { ...base, contributes: piece };
  if (['permissions', 'contributes', 'limits', 'run', 'activation'].some((key) => key in piece)) return { ...base, ...piece };
  return null;
}

describe('the examples of the plugins’ pages', () => {
  it('shows TypeScript and Rust only under the path of a file that compiles', () => {
    for (const { page, text } of PAGES)
      for (const { before, lang, code } of blocks(text))
        if (['ts', 'rust', 'toml'].includes(lang))
          expect(before, `${page}: a block of ${lang} without its path, a copy no compiler reads:\n${code}`).toMatch(QUOTED);
  });

  it('holds every piece of a manifest to the checks of validate', () => {
    let seen = 0;
    for (const { page, text } of PAGES)
      for (const { before, lang, code } of blocks(text)) {
        // Lines of a file: that file's manifest is what is judged, below and by its plugin's own checks.
        if (lang !== 'json' || QUOTED.test(before)) continue;
        const piece = fragment(code);
        const manifest = piece && asManifest(piece);
        if (!manifest) continue;
        seen++;
        const judged = validateText(JSON.stringify(manifest), { builtin: true });
        expect(judged.ok, `${page}:\n${code}\n${JSON.stringify(judged)}`).toBe(true);
      }
    expect(seen).toBeGreaterThan(3);
  });

  it('keeps the two plugins of the examples valid, the same but for their program', () => {
    const read = (kind: string) => fs.readFileSync(path.join(__dirname, kind, 'escouade-plugin.json'), 'utf8');
    for (const kind of ['rust', 'ts']) expect(validateText(read(kind))).toMatchObject({ ok: true, ignored: [] });
    const { run: wasm, ...rust } = JSON.parse(read('rust'));
    const { run: js, ...ts } = JSON.parse(read('ts'));
    expect(rust).toEqual(ts);
    expect([wasm, js]).toEqual([{ wasm: 'Cargo.toml' }, { js: 'dist/plugin.mjs' }]);
  });

  it('typechecks the TypeScript of the pages against the SDK', () => {
    const tsc = path.join(ROOT, 'node_modules', 'typescript', 'bin', 'tsc');
    const r = spawnSync(process.execPath, [tsc, '-p', path.join(__dirname, 'ts', 'tsconfig.json')], { encoding: 'utf8' });
    expect(r.stdout + r.stderr).toBe('');
    expect(r.status).toBe(0);
  }, 60_000);
});
