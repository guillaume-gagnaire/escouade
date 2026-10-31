import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { runChecks, verdict, type CheckKey, type CheckResult, type Snapshot } from '../../server/utils/checks';
import { packFolder, scope } from '../../server/utils/folder';
import { readPublicKey } from '../../server/utils/minisign';
import { checkVersionSignature, verifiedComment } from '../../server/utils/signature';
import { readArchive, type ArchiveRead } from '../../server/utils/tar';
import { sha256, versionFile } from '../../server/utils/versions';
import { testKey } from '../fake/minisign.mjs';
import { tarball } from '../fake/plugins.mjs';

// The trial of STORE-10 (docs/STORE.md §9), played against the store's own code with a test key: Escouade's own
// plugins (plugins/<id>/ of the app's repository, ESC-314) submitted from their folder to the ten checks, as the
// worker reads them (the folder alone, the root's README and LICENSE standing in), then their version.json signed and
// checked as the store checks a signature. The review is played by plugin-review.test.ts and review-admin.test.ts, the
// install and the revocation in the app by src-tauri/src/plugins_store_tests.rs and tests/e2e/store.spec.ts; the real
// trial waits for the chief (§9.1).

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const PLUGINS = ['high-contrast', 'discord', 'slack', 'team-rules'];
const APP = 'guillaume-gagnaire/escouade';
const COMMIT = 'e5a1c0de'.padEnd(40, '0');

type Files = Record<string, string | Buffer>;

/** A folder's files as git holds them, under `prefix` (no node_modules nor what npm leaves: they are not committed). */
function folder(dir: string, prefix = ''): Files {
  const out: Files = {};
  const walk = (d: string) => {
    for (const f of readdirSync(d)) {
      const p = join(d, f);
      if (f === 'node_modules' || f === 'package-lock.json' || f === 'packs') continue;
      if (statSync(p).isDirectory()) walk(p);
      else out[prefix + relative(dir, p).replace(/\\/g, '/')] = readFileSync(p);
    }
  };
  walk(dir);
  return out;
}

/** The app's repository as the store reads it: its README and LICENSE (GPL-3.0) at the root, and plugins/. */
const repository: Files = {
  'README.md': readFileSync(join(ROOT, 'README.md')),
  LICENSE: readFileSync(join(ROOT, 'LICENSE')),
  ...folder(join(ROOT, 'plugins'), 'plugins/'),
};
const whole: ArchiveRead = readArchive(tarball(repository, { top: `guillaume-gagnaire-escouade-${COMMIT.slice(0, 7)}`, commit: COMMIT }));

/** The snapshot of the folder `path` of `repo`, as the worker builds it; `firstParty` as it finds it by GitHub's id. */
function snapshot(repo: string, path: string, tag: string, over: Partial<Snapshot> = {}): Snapshot {
  const read = scope(whole, path);
  const gz = packFolder(read, 'top', COMMIT);
  return {
    repository: repo,
    path,
    root: path ? whole : null,
    pathLink: null,
    tag,
    commit: COMMIT,
    archive: { size: gz.length, read },
    // What GitHub detects at the root: the app's license.
    license: 'GPL-3.0',
    binaries: [],
    releaseAssets: null,
    existing: null,
    taken: [],
    latest: null,
    others: [],
    ...over,
  };
}

const failed = (results: CheckResult[]): CheckKey[] => results.filter((r) => r.status === 'fail').map((r) => r.key);
const manifestOf = (name: string) => JSON.parse(readFileSync(join(ROOT, 'plugins', name, 'escouade-plugin.json'), 'utf8'));

describe('Escouade’s own plugins submitted to the store from their folder (ESC-314)', () => {
  it('are dev.escouade.<folder> 1.1.0, their repository the folder of the app’s, each with its README, LICENSE and tests', () => {
    for (const name of PLUGINS) {
      const m = manifestOf(name);
      expect(m.id).toBe(`dev.escouade.${name}`);
      expect(m.version).toBe('1.1.0');
      expect(m.repository).toBe(`https://github.com/${APP}/tree/main/plugins/${name}`);
      for (const f of ['README.md', 'LICENSE', 'CHANGELOG.md', 'escouade-plugin.json'])
        expect(repository[`plugins/${name}/${f}`], f).toBeTruthy();
      expect(Object.keys(repository).some((p) => p.startsWith(`plugins/${name}/test/`))).toBe(true);
    }
  });

  it('as the folder plugins/ itself: refused, it holds no manifest', () => {
    const results = runChecks(snapshot(APP, 'plugins', 'plugins/v1.1.0', { firstParty: true }));
    expect(failed(results)).toContain('manifest');
    expect(verdict(results)).toBe('rejected');
  });

  it.each(PLUGINS)('%s from its folder of the app’s repository passes the ten checks, and its signed version.json checks', (name) => {
    const path = `plugins/${name}`;
    const tag = `${path}/v1.1.0`;
    const snap = snapshot(APP, path, tag, { firstParty: true });
    const results = runChecks(snap);
    expect(results.filter((r) => r.status !== 'pass')).toEqual([]);
    expect(verdict(results)).toBe('ready');
    // Its own LICENSE (MIT, by its SPDX line) and README, not the app's.
    expect(results.find((r) => r.key === 'license')!.detail.en).toContain('MIT');

    const m = manifestOf(name);
    const gz = packFolder(snap.archive.read, 'top', COMMIT);
    const { json } = versionFile({
      id: m.id,
      version: m.version,
      repository: `https://github.com/${APP}`,
      path,
      tag,
      commit: COMMIT,
      archive: { sha256: sha256(gz), size: gz.length },
      binaries: {},
      manifest_sha256: sha256(readFileSync(join(ROOT, path, 'escouade-plugin.json'))),
      escouade: m.escouade,
    });
    const key = testKey();
    const pub = readPublicKey(key.pub)!;
    const minisig = key.sign(json, verifiedComment(m.id, m.version, '2026-10-31'));
    expect(checkVersionSignature(json, minisig, [pub], m.id, m.version)).toMatchObject({ date: '2026-10-31' });
    // The same signature does not stand for another version, another key, or a key the list revokes.
    expect(checkVersionSignature(json, minisig, [pub], m.id, '9.9.9')).toHaveProperty('error');
    expect(checkVersionSignature(json, minisig, [readPublicKey(testKey().pub)!], m.id, m.version)).toHaveProperty('error');
    expect(checkVersionSignature(json, minisig, [pub], m.id, m.version, [pub.id])).toHaveProperty('error');
    expect(checkVersionSignature(json.replace(`"${m.version}"`, '"9.9.9"'), minisig, [pub], m.id, m.version)).toHaveProperty('error');
  });

  it.each(PLUGINS)('%s from another repository (a fork, a copy): its id is refused as today', (name) => {
    const path = `plugins/${name}`;
    const results = runChecks(snapshot('lea-martin/escouade', path, `${path}/v1.1.0`, { firstParty: false }));
    const id = results.find((r) => r.key === 'id')!;
    expect(id.status).toBe('fail');
    expect(id.detail.en).toBe(`dev.escouade.${name} is reserved`);
    expect(id.problems[0]!.fix.en).toContain('io.github.<your-login>');
    expect(verdict(results)).toBe('rejected');
  });
});
