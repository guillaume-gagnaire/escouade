// @vitest-environment node
// The kit that goes to crates.io (docs/PLUGINS.md §8.6.3): what a crate must hold to be published, and the way the plugins
// reach it. Nothing here publishes: `cargo publish --dry-run` is the CI's, and the real one the maintainer's.
import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = path.resolve(__dirname, '..');
const CRATES = ['escouade-plugin-protocol', 'escouade-plugin-kit', 'escouade-acp', 'escouade-voice'];
const read = (...parts: string[]) => fs.readFileSync(path.join(ROOT, ...parts), 'utf8');
const field = (toml: string, name: string) => new RegExp(`^${name} = (.+)$`, 'm').exec(toml)?.[1];
const list = (value: string | undefined): string[] => JSON.parse(value ?? '[]');

/** The dependencies on the kit's crates of a manifest, each line whole. */
const kitLines = (toml: string) => toml.split('\n').filter((l) => CRATES.some((c) => l.startsWith(`${c} = `)));

describe.each(CRATES)('%s', (name) => {
  const toml = read('crates', name, 'Cargo.toml');

  it('says all crates.io and docs.rs show', () => {
    expect(field(toml, 'version')).toMatch(/^"0\.1\.\d+"$/);
    expect(field(toml, 'license')).toBe('"MIT"');
    expect(field(toml, 'repository')).toBe('"https://github.com/guillaume-gagnaire/escouade"');
    expect(field(toml, 'documentation')).toBe(`"https://docs.rs/${name}"`);
    expect(field(toml, 'readme')).toBe('"README.md"');
    expect(field(toml, 'description')).toBeTruthy();
    expect(toml).toContain('[package.metadata.docs.rs]');
  });

  it('keeps to the limits of crates.io', () => {
    const keywords = list(field(toml, 'keywords'));
    expect(keywords.length).toBeGreaterThan(0);
    expect(keywords.length).toBeLessThanOrEqual(5);
    for (const k of keywords) expect(k).toMatch(/^[a-z][a-z0-9_-]{0,19}$/);
    const categories = list(field(toml, 'categories'));
    expect(categories.length).toBeGreaterThan(0);
    expect(categories.length).toBeLessThanOrEqual(5);
  });

  it('ships its licence text and its README, which is its documentation', () => {
    expect(read('crates', name, 'LICENSE')).toMatch(/^MIT License/);
    expect(read('crates', name, 'README.md')).toContain('MIT');
    expect(read('crates', name, 'src/lib.rs')).toContain('#![doc = include_str!("../README.md")]');
    expect(list(field(toml, 'include'))).toEqual(expect.arrayContaining(['src/**/*.rs', 'Cargo.toml', 'README.md', 'LICENSE']));
  });

  // With both: `cargo publish` strips the path, and the version alone would have it look for the crate on crates.io
  // before it is there (`[patch.crates-io]` does not reach the packaging).
  it('reaches the others by a version, which is all that gets published', () => {
    for (const line of kitLines(toml)) expect(line).toMatch(/^(\S+) = \{ path = "\.\.\/\1", version = "0\.1\.0" \}$/);
  });
});

describe('the workspace', () => {
  const root = read('src-tauri', 'Cargo.toml');

  it('brings the three crates back to their folders for development', () => {
    const patch = /^\[patch\.crates-io\]\n((?:.+\n)+)/m.exec(root)?.[1] ?? '';
    for (const name of CRATES) expect(patch).toContain(`${name} = { path = "../crates/${name}" }`);
  });

  it('has every plugin depend on them by version, as the store will read it', () => {
    const plugins = path.join(ROOT, 'plugins');
    const manifests = fs
      .readdirSync(plugins)
      .map((d) => path.join(plugins, d, 'Cargo.toml'))
      .filter((f) => fs.existsSync(f));
    expect(manifests.length).toBeGreaterThan(0);
    const lines = manifests.flatMap((f) => kitLines(fs.readFileSync(f, 'utf8')));
    expect(lines.length).toBeGreaterThan(0);
    for (const line of lines) expect(line).toMatch(/^\S+ = "0\.1\.\d+"$/);
  });
});
