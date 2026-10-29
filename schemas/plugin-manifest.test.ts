import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { message, validate, validateText } from './plugin-manifest.mjs';

// The store's library on the cases it shares with the app's reader (src-tauri/src/plugins_tests.rs): the same fields
// wrong, for the same reasons, and the same fields ignored.

interface Case {
  name: string;
  builtin?: boolean;
  manifest: unknown;
  files?: Record<string, { png?: [number, number]; size?: number; text?: string }>;
  errors: string[];
  ignored: string[];
}

const file = path.join(__dirname, '..', 'tests', 'fixtures', 'plugins', 'manifests.json');
const { cases } = JSON.parse(fs.readFileSync(file, 'utf8')) as { cases: Case[] };

/** The bytes of a case's file: a PNG's header of its width and height, padded to its size; or its text. */
function bytesOf(f: { png?: [number, number]; size?: number; text?: string }): Uint8Array {
  if (f.text !== undefined) return Uint8Array.from(f.text, (c) => c.charCodeAt(0));
  const b = new Uint8Array(Math.max(f.size ?? 64, 33));
  b.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]);
  const view = new DataView(b.buffer);
  view.setUint32(16, f.png![0]);
  view.setUint32(20, f.png![1]);
  b.set([8, 6, 0, 0, 0], 24);
  return b;
}

/** The reader of a case's folder, if it has files. */
const folderOf = (c: Case) => (c.files ? (p: string) => (c.files![p] ? bytesOf(c.files![p]!) : null) : undefined);

describe('plugin-manifest', () => {
  it('has the shared cases', () => expect(cases.length).toBeGreaterThanOrEqual(10));

  for (const c of cases) {
    it(c.name, () => {
      const r = validate(c.manifest, { builtin: c.builtin, file: folderOf(c) });
      const errors = r.errors.map((e) => `${e.path} ${e.code}`);
      expect(errors.sort()).toEqual([...c.errors].sort());
      expect(r.ignored.sort()).toEqual([...c.ignored].sort());
      expect(r.ok).toBe(c.errors.length === 0);
    });
  }

  it('says each field and why in the app’s words, in both languages', () => {
    const r = validate({
      id: 'Slack',
      name: 'Slack',
      version: '1.2.0',
      escouade: { api: 'v1', min: '2.1.0' },
      contributes: { settings: [{ key: 'a', type: 'colour', title: 'A' }] },
    });
    expect(r.errors.map((e) => message(e, 'fr'))).toEqual([
      "`id` n'est pas un nom de domaine inversé en minuscules (`dev.exemple.slack`).",
      '`escouade.api` n\'est pas une version majeure de l\'API des plugins (`"1"`).',
      "`contributes.settings[0].type` vaut l'un de : string, number, boolean, select.",
    ]);
    expect(message(r.errors[0], 'en')).toBe('`id` is no reversed domain name in lower case (`dev.exemple.slack`).');
    expect(message(validate({}).errors[0], 'fr')).toBe('`id` manque.');
    expect(message(validateText('[]').errors[0], 'fr')).toBe('Le manifeste devrait être un objet.');
    expect(validateText('{ id: ').errors).toEqual([{ path: '', code: 'type', expected: 'object' }]);
    const icon = (png: [number, number], size?: number) =>
      validate(
        { id: 'dev.exemple.logo', name: 'Logo', version: '1.0.0', escouade: { api: '1', min: '2.2.0' }, icon: './icon.png' },
        { file: (p) => (p === 'icon.png' ? bytesOf({ png, size }) : null) },
      ).errors.map((e) => message(e, 'fr'));
    expect(icon([512, 512])).toEqual([]);
    expect(icon([200, 100], 2_000_000)).toEqual([
      '`icon` pèse plus de 1 Mo.',
      "`icon` n'est pas carrée (un PNG carré de 256 px au moins, 512 conseillé).",
      '`icon` fait moins de 256 px de côté (512 conseillé).',
    ]);
    const svg = validate({ icon: 'logo.svg' }).errors.find((e) => e.path === 'icon')!;
    expect(message(svg, 'en')).toBe('`icon` is no PNG (no SVG nor JPEG: a square PNG of 256 px at least).');
  });
});

describe('plugin-1.json', () => {
  const schema = JSON.parse(fs.readFileSync(path.join(__dirname, 'plugin-1.json'), 'utf8'));
  const pattern = (p: string) => new RegExp(p, 'u');

  it('holds patterns an editor can read, that say what the library says', () => {
    const inner = pattern(schema.properties.run.properties.command.pattern);
    for (const p of ['bin/x', 'bin\\x.exe', 'a..b/c']) expect(inner.test(p), p).toBe(true);
    for (const p of ['../x', 'bin/../../x', '/usr/bin/x', '\\x', 'C:\\x', 'bin\\..']) {
      expect(inner.test(p), p).toBe(false);
    }
    const id = pattern(schema.properties.id.pattern);
    expect(id.test('dev.exemple.slack')).toBe(true);
    expect(id.test('Slack')).toBe(false);
    expect(id.test('slack')).toBe(false);
    const version = pattern(schema.properties.version.pattern);
    expect(version.test('0.1.0-beta.1')).toBe(true);
    expect(version.test('1.2')).toBe(false);
    expect(schema.required).toEqual(['id', 'name', 'version', 'escouade']);
    const icon = pattern(schema.properties.icon.pattern);
    for (const p of ['icon.png', 'assets/Logo.PNG', './store/icon.png']) expect(icon.test(p), p).toBe(true);
    for (const p of ['icon.svg', '../icon.png', '/icon.png', 'C:\\icon.png', 'https://exemple.dev/icon.png', 'icon.png.svg']) {
      expect(icon.test(p), p).toBe(false);
    }
  });
});
