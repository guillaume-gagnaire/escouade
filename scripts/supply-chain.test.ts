// @vitest-environment node
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { TREE, withTesting } from './no-testing.mjs';
import { sums } from './release-sums.mjs';

// SEC-04, SEC-16 and SEC-18 of the security audit of 2026-11 (ESC-594): the advisories of the store and the site, the
// SHA-256 of the installers, the `testing` feature and the Apple secrets, read in the workflows as text (the repository
// has no YAML parser at its root).
const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const workflow = (name: string) => fs.readFileSync(path.join(ROOT, '.github', 'workflows', name), 'utf8');

/** The steps of a workflow's job: each `- name:` or `- uses:` at the steps' indent, with its lines. */
function steps(text: string, job: string): { name: string; body: string }[] {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((l) => l === `  ${job}:`);
  expect(start, `the job ${job}`).toBeGreaterThan(-1);
  const end = lines.findIndex((l, i) => i > start && /^ {2}[A-Za-z0-9_-]+:\s*$/.test(l));
  const body = lines.slice(start + 1, end === -1 ? undefined : end);
  const found: { name: string; body: string }[] = [];
  for (const line of body) {
    if (/^ {6}- /.test(line)) found.push({ name: line.trim(), body: '' });
    if (found.length) found[found.length - 1].body += `${line}\n`;
  }
  // The comments above a step belong to it, not to the one before.
  for (const s of found) s.body = s.body.replace(/(\n(\s*#.*)?)+\n?$/, '\n');
  return found;
}

describe('the testing feature (SEC-18)', () => {
  // The shape of `cargo tree -e normal,build,features --prefix none`.
  const tree = [
    'escouade v2.2.6 (C:\\escouade\\src-tauri)',
    'escouade-model v2.2.6 (C:\\escouade\\crates\\escouade-model)',
    'escouade-model feature "default"',
    'serde feature "derive"',
    'escouade-mcp feature "testing"',
    'escouade-base feature "testing" (*)',
    'escouade-mcp feature "testing" (*)',
    'tokio feature "test-util"',
  ].join('\n');

  it('names each crate that has it on, once', () => {
    expect(withTesting(tree)).toEqual(['escouade-base', 'escouade-mcp']);
  });

  it('finds none in a tree without it', () => {
    expect(withTesting('escouade v2.2.6\nescouade-model feature "default"\nrcgen feature "testing-helpers"\n')).toEqual([]);
  });

  it('reads the normal and build dependencies of every target, never the dev ones', () => {
    const edges = TREE[TREE.indexOf('-e') + 1].split(',');
    expect(edges).toEqual(expect.arrayContaining(['normal', 'build', 'features']));
    expect(edges).not.toContain('dev');
    expect(edges).not.toContain('all');
    expect(TREE).toEqual(expect.arrayContaining(['--workspace', '--target', 'all', '--locked']));
  });

  it('is checked by the CI and by the release before the build', () => {
    expect(workflow('ci.yml')).toContain('run: node scripts/no-testing.mjs');
    const build = steps(workflow('release.yml'), 'build');
    const check = build.findIndex((s) => s.body.includes('node scripts/no-testing.mjs'));
    const first = build.findIndex((s) => s.body.includes('tauri-apps/tauri-action@'));
    expect(check).toBeGreaterThan(-1);
    expect(check).toBeLessThan(first);
  });
});

describe('the SHA-256 of the installers (SEC-16)', () => {
  it('are written as sha256sum writes them, by name', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'esc-sums-'));
    try {
      fs.writeFileSync(path.join(dir, 'Escouade_2.2.6_x64-setup.exe'), 'setup');
      fs.writeFileSync(path.join(dir, 'Escouade_2.2.6_amd64.AppImage'), 'appimage');
      fs.mkdirSync(path.join(dir, 'sub'));
      const sha = (s: string) => crypto.createHash('sha256').update(s).digest('hex');
      expect(await sums(dir)).toEqual([
        `${sha('appimage')}  Escouade_2.2.6_amd64.AppImage`,
        `${sha('setup')}  Escouade_2.2.6_x64-setup.exe`,
      ]);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it('are put in the release before it is published, and said in its notes', () => {
    const text = workflow('release.yml');
    const publish = steps(text, 'publish');
    const sumsAt = publish.findIndex((s) => s.body.includes('node scripts/release-sums.mjs'));
    const publishAt = publish.findIndex((s) => s.name === '- name: Publish');
    expect(sumsAt).toBeGreaterThan(-1);
    expect(sumsAt).toBeLessThan(publishAt);
    expect(publish[sumsAt].body).toContain('assets?name=SHA256SUMS.txt');
    expect(publish[sumsAt].body).toContain('sha256sum -c');
    expect(text).toContain('Their SHA-256 are in `SHA256SUMS.txt`');
  });
});

describe('the Apple secrets (SEC-18)', () => {
  it('are given to the step that builds and signs the macOS app, and to no other', () => {
    const build = steps(workflow('release.yml'), 'build');
    const holders = build.filter((s) => /secrets\.APPLE_/.test(s.body));
    expect(holders.map((s) => s.name)).toEqual(['- name: Build, sign (Developer ID) and upload to the draft']);
    expect(holders[0].body).toContain("if: runner.os == 'macOS' && env.APPLE_SIGNING == 'true'");
    // Never written into the job's environment.
    expect(workflow('release.yml')).not.toMatch(/APPLE_[A-Z_]+=.*GITHUB_ENV/);
    expect(workflow('release.yml')).not.toContain('$v=${!v}');
  });

  it('only say to the job whether there is a certificate', () => {
    const text = workflow('release.yml');
    const env = /\n {4}env:\n((?: {6}.*\n)+)/.exec(text.slice(text.indexOf('\n  build:\n')));
    expect(env?.[1]).toContain("APPLE_SIGNING: ${{ secrets.APPLE_CERTIFICATE != '' }}");
  });

  it('leave the two forms of the build alike', () => {
    const build = steps(workflow('release.yml'), 'build').filter((s) => s.body.includes('tauri-apps/tauri-action@'));
    expect(build).toHaveLength(2);
    const withs = build.map((s) =>
      s.body.slice(s.body.indexOf('        with:')).replace(/ \$\{\{ steps\.windows-signing\.outputs\.args \}\}/, ''),
    );
    expect(withs[0]).toBe(withs[1]);
    for (const s of build) {
      expect(s.body).toContain('TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}');
      expect(s.body).toContain('CARGO_PROFILE_RELEASE_LTO: thin');
    }
    expect(build[1].body).toContain("if: runner.os != 'macOS' || env.APPLE_SIGNING != 'true'");
  });
});

describe('the advisories of the store and the site (SEC-04)', () => {
  const text = workflow('advisories.yml');

  it('fail the job from high up, and any of the app', () => {
    expect(text).toMatch(/- dir: \.\n {12}level: low/);
    expect(text).toMatch(/- dir: store\n {12}level: high/);
    expect(text).toMatch(/- dir: website\n {12}level: high/);
    expect(text).toContain('run: npm audit --omit=dev --audit-level=${{ matrix.level }}');
    expect(text).not.toContain('continue-on-error');
  });

  it('have their delays written', () => {
    const doc = fs.readFileSync(path.join(ROOT, 'docs', 'SECURITY-DEPENDENCIES.md'), 'utf8');
    expect(doc).toContain('## Délais de correction');
    expect(doc).toMatch(/Store .* \| un avis élevé ou critique \(`--audit-level=high`\) \| 7 jours/);
  });
});
