import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { runChecks, verdict, type CheckKey, type CheckResult, type Snapshot } from '../../server/utils/checks';
import { readPublicKey } from '../../server/utils/minisign';
import { checkVersionSignature, verifiedComment } from '../../server/utils/signature';
import { readArchive } from '../../server/utils/tar';
import { sha256, versionFile } from '../../server/utils/versions';
import { testKey } from '../fake/minisign.mjs';
import { tarball } from '../fake/plugins.mjs';
import { exampleFiles } from '../fake/review-store.mjs';

// The trial of STORE-10 (docs/STORE.md §9), played against the store's own code with a test key: PLG-10's examples
// submitted to the ten checks, then their version.json signed and checked as the store checks a signature. The review
// is played by plugin-review.test.ts and review-admin.test.ts, the install and the revocation in the app by
// src-tauri/src/plugins_store_tests.rs and tests/e2e/store.spec.ts; the real trial waits for the user (§9.1).

const ROOT = fileURLToPath(new URL('../../../plugin-examples/', import.meta.url));
const EXAMPLES = ['high-contrast', 'discord-webhook', 'slack', 'team-rules'];
const COMMIT = 'e5a1c0de'.padEnd(40, '0');

type Files = Record<string, string | Buffer>;

function snapshot(repo: string, files: Files, license: string | null = 'MIT'): Snapshot {
  const gz = tarball(files, { top: `${repo.replace('/', '-')}-e5a1c0d`, commit: COMMIT });
  return {
    repository: repo,
    tag: 'v1.0.0',
    commit: COMMIT,
    archive: { size: gz.length, read: readArchive(gz) },
    license,
    binaries: [],
    releaseAssets: null,
    existing: null,
    taken: [],
    latest: null,
    others: [],
  };
}

/** A folder's files as git would hold them (no node_modules: the examples have no lock file nor install). */
function folder(dir: string): Files {
  const out: Files = {};
  const walk = (d: string) => {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      if (f === 'node_modules') continue;
      if (statSync(p).isDirectory()) walk(p);
      else out[relative(dir, p).replace(/\\/g, '/')] = readFileSync(p);
    }
  };
  walk(dir);
  return out;
}

/** An example in a repository of its own: its folder at the root, a README, the LICENSE, and `repository` set. */
function ownRepository(name: string): { repo: string; files: Files } {
  const repo = `escouade-dev/plugin-${name}`;
  const files = exampleFiles(name) as Files;
  const m = JSON.parse(String(files['escouade-plugin.json']));
  files['escouade-plugin.json'] = JSON.stringify({ ...m, repository: `https://github.com/${repo}` }, null, 2);
  return { repo, files };
}

const failed = (results: CheckResult[]): CheckKey[] => results.filter((r) => r.status === 'fail').map((r) => r.key);

describe('the examples of PLG-10 submitted to the store', () => {
  it('as the repository plugin-examples/ itself: refused, the store reads one plugin at the root of a repository', () => {
    const results = runChecks(snapshot('escouade-dev/escouade', folder(ROOT)));
    expect(failed(results)).toContain('manifest');
    expect(verdict(results)).toBe('rejected');
  });

  it.each(EXAMPLES)('%s as its folder alone: no repository, no README, no LICENSE', (name) => {
    const results = runChecks(snapshot(`escouade-dev/plugin-${name}`, folder(join(ROOT, name)), null));
    expect(failed(results)).toEqual(expect.arrayContaining(['repository', 'license', 'readme']));
    expect(failed(results)).not.toContain('manifest');
  });

  it.each(EXAMPLES)('%s in a repository of its own passes the ten checks, and its signed version.json checks', (name) => {
    const { repo, files } = ownRepository(name);
    const snap = snapshot(repo, files);
    const results = runChecks(snap);
    expect(results.filter((r) => r.status !== 'pass')).toEqual([]);
    expect(verdict(results)).toBe('ready');

    const m = JSON.parse(String(files['escouade-plugin.json']));
    const gz = tarball(files, { top: `${repo.replace('/', '-')}-e5a1c0d`, commit: COMMIT });
    const { json } = versionFile({
      id: m.id,
      version: m.version,
      repository: `https://github.com/${repo}`,
      tag: 'v1.0.0',
      commit: COMMIT,
      archive: { sha256: sha256(gz), size: gz.length },
      binaries: {},
      manifest_sha256: sha256(files['escouade-plugin.json']),
      escouade: m.escouade,
    });
    const key = testKey();
    const pub = readPublicKey(key.pub)!;
    const minisig = key.sign(json, verifiedComment(m.id, m.version, '2026-10-30'));
    expect(checkVersionSignature(json, minisig, [pub], m.id, m.version)).toMatchObject({ date: '2026-10-30' });
    // The same signature does not stand for another version, another key, or a key the list revokes.
    expect(checkVersionSignature(json, minisig, [pub], m.id, '9.9.9')).toHaveProperty('error');
    expect(checkVersionSignature(json, minisig, [readPublicKey(testKey().pub)!], m.id, m.version)).toHaveProperty('error');
    expect(checkVersionSignature(json, minisig, [pub], m.id, m.version, [pub.id])).toHaveProperty('error');
    expect(checkVersionSignature(json.replace('1.0.0', '1.0.1'), minisig, [pub], m.id, m.version)).toHaveProperty('error');
  });
});
