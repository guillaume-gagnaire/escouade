// @vitest-environment node
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { readVersions, setVersion } from './version.mjs';

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-version-'));
  fs.mkdirSync(path.join(root, 'src-tauri'));
  fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ name: 'x', version: '0.1.0', scripts: {} }, null, 2) + '\n');
  fs.writeFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), JSON.stringify({ productName: 'X', version: '0.1.0' }, null, 2) + '\n');
  fs.writeFileSync(
    path.join(root, 'src-tauri', 'Cargo.toml'),
    '[package]\nname = "x"\nversion = "0.1.0"\n\n[dependencies]\nserde = { version = "1" }\n',
  );
  return root;
}

describe('version script', () => {
  it('sets the same version in the three manifests without touching dependency versions', () => {
    const root = fixture();
    setVersion(root, '1.2.3');
    expect(readVersions(root)).toEqual({ package: '1.2.3', tauri: '1.2.3', cargo: '1.2.3' });
    expect(fs.readFileSync(path.join(root, 'src-tauri', 'Cargo.toml'), 'utf8')).toContain('serde = { version = "1" }');
  });

  it('keeps the lockfiles’ own entry of the app in step, and only that one', () => {
    const root = fixture();
    const npmLock = path.join(root, 'package-lock.json');
    const cargoLock = path.join(root, 'src-tauri', 'Cargo.lock');
    fs.writeFileSync(
      npmLock,
      JSON.stringify(
        { name: 'x', version: '0.1.0', packages: { '': { name: 'x', version: '0.1.0' }, 'node_modules/y': { version: '0.1.0' } } },
        null,
        2,
      ) + '\n',
    );
    fs.writeFileSync(cargoLock, '[[package]]\nname = "serde"\nversion = "0.1.0"\n\n[[package]]\nname = "x"\nversion = "0.1.0"\n');
    setVersion(root, '1.2.3');
    const lock = JSON.parse(fs.readFileSync(npmLock, 'utf8'));
    expect([lock.version, lock.packages[''].version, lock.packages['node_modules/y'].version]).toEqual(['1.2.3', '1.2.3', '0.1.0']);
    expect(fs.readFileSync(cargoLock, 'utf8')).toBe(
      '[[package]]\nname = "serde"\nversion = "0.1.0"\n\n[[package]]\nname = "x"\nversion = "1.2.3"\n',
    );
  });

  it('keeps the crate of escouade-plugin at the app’s version, in its manifest and in the lock', () => {
    const root = fixture();
    const crate = path.join(root, 'crates', 'escouade-plugin-cli', 'Cargo.toml');
    fs.mkdirSync(path.dirname(crate), { recursive: true });
    fs.writeFileSync(crate, '[package]\nname = "escouade-plugin-cli"\nversion = "0.1.0"\n');
    const cargoLock = path.join(root, 'src-tauri', 'Cargo.lock');
    fs.writeFileSync(
      cargoLock,
      '[[package]]\nname = "escouade-plugin-cli"\nversion = "0.1.0"\n\n[[package]]\nname = "x"\nversion = "0.1.0"\n',
    );
    setVersion(root, '1.2.3');
    expect(readVersions(root)).toEqual({ package: '1.2.3', tauri: '1.2.3', cargo: '1.2.3', cli: '1.2.3' });
    expect(fs.readFileSync(cargoLock, 'utf8')).toBe(
      '[[package]]\nname = "escouade-plugin-cli"\nversion = "1.2.3"\n\n[[package]]\nname = "x"\nversion = "1.2.3"\n',
    );
  });

  it('accepts a v-prefixed tag and rejects non-semver input', () => {
    const root = fixture();
    setVersion(root, 'v2.0.0-beta.1');
    expect(readVersions(root).cargo).toBe('2.0.0-beta.1');
    expect(() => setVersion(root, 'latest')).toThrow(/semver/);
  });
});
