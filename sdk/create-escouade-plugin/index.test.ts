// @vitest-environment node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { validateText } from '../../schemas/plugin-manifest.mjs';
import { create, SDK_VERSION } from './index.mjs';

// npm create escouade-plugin (ESC-181, PLG-9): each template written whole, its manifest valid. The template's own
// tests, its build, validate and the conformance bench run in CI (sdk/scripts/template-ci.mjs).

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'create-plugin-'));

describe('create-escouade-plugin', () => {
  it('depends on the version of @escouade-dev/plugin the repository holds', () => {
    const sdk = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'plugin', 'package.json'), 'utf8'));
    expect(SDK_VERSION).toBe(`^${sdk.version}`);
  });

  it('writes a TypeScript program with its tests, a valid manifest and an executable program', () => {
    const folder = path.join(tmp(), 'Mes Règles');
    const written = create({ folder, kind: 'program', id: 'dev.test.rules', name: 'Règles "de" l’équipe', sdk: 'file:../sdk' });
    const names = written.map((f) => path.relative(folder, f).split(path.sep).join('/')).sort();
    expect(names).toEqual([
      '.gitignore',
      'LICENSE',
      'README.md',
      'escouade-plugin.json',
      'package.json',
      'plugin',
      'plugin.cmd',
      'src/main.ts',
      'src/plugin.ts',
      'test/plugin.test.ts',
      'tsconfig.json',
    ]);
    const manifest = fs.readFileSync(path.join(folder, 'escouade-plugin.json'), 'utf8');
    expect(validateText(manifest)).toMatchObject({ ok: true, ignored: [] });
    expect(JSON.parse(manifest)).toMatchObject({ id: 'dev.test.rules', name: 'Règles "de" l’équipe', run: { command: 'plugin' } });
    const pkg = JSON.parse(fs.readFileSync(path.join(folder, 'package.json'), 'utf8'));
    expect(pkg.name).toBe('mes-regles');
    expect(pkg.devDependencies['@escouade-dev/plugin']).toBe('file:../sdk');
    if (process.platform !== 'win32') expect(fs.statSync(path.join(folder, 'plugin')).mode & 0o111).not.toBe(0);
    expect(fs.readFileSync(path.join(folder, 'LICENSE'), 'utf8')).toContain(`Copyright (c) ${new Date().getFullYear()}`);
  });

  it('writes a declarative theme that runs nothing', () => {
    const folder = path.join(tmp(), 'my-theme');
    create({ folder, kind: 'declarative' });
    const manifest = JSON.parse(fs.readFileSync(path.join(folder, 'escouade-plugin.json'), 'utf8'));
    expect(validateText(JSON.stringify(manifest)).ok).toBe(true);
    expect(manifest.run).toBeUndefined();
    expect(manifest.id).toMatch(/^dev\.[a-z0-9-]+\.my-theme$/);
    expect(manifest.contributes.themes[0].name).toBe('My theme');
  });

  it('refuses a folder that is not empty, an id that is not one, a kind it does not know', () => {
    const folder = tmp();
    fs.writeFileSync(path.join(folder, 'x'), '');
    expect(() => create({ folder })).toThrow(/not empty/);
    expect(() => create({ folder: path.join(tmp(), 'a'), id: 'Nope' })).toThrow(/--id Nope/);
    expect(() => create({ folder: path.join(tmp(), 'a'), kind: 'wasm' })).toThrow(/--kind wasm/);
  });
});
