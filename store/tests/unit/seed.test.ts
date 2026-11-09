import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { readPublicKey, type PublicKey } from '../../server/utils/minisign';
import { devSeed, fetchSeed, keysOf, parseList, type Entry } from '../../scripts/seed';

// The release's seed (ESC-338) against the fake store, what it published and signed: downloaded and checked as the app will
// check it, and a release that stops at the first gap.

const ROOT = path.resolve(import.meta.dirname, '..', '..', '..');
// The store's image holds store/ alone: these tests need the repository whole (the fake store, plugins/seed.json, the
// keys, a fixture's module), and are skipped without it.
const FAKE = path.join(ROOT, 'tests', 'fixtures', 'store', 'fake-store.mjs');
const whole = fs.existsSync(FAKE);
type Fake = { url: string; key: string; close: () => Promise<void> };
let startFakeStore: (port: number) => Promise<Fake>;
const TRELLO = 'dev.escouade.trello';
const sha256 = (b: Buffer | string) => createHash('sha256').update(b).digest('hex');

let fake: Fake | undefined;
let keys: PublicKey[];

beforeAll(async () => {
  if (!whole) return;
  ({ startFakeStore } = await import(pathToFileURL(FAKE).href));
  fake = await startFakeStore(0);
  keys = [readPublicKey(fake!.key)!];
  await post('/_fake/revocations', { entries: [] });
});
afterAll(() => fake?.close());

const post = (p: string, body: unknown) =>
  fetch(`${fake!.url}${p}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
const temp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'seed-'));
const fingerprint = async (id: string, version: string) =>
  sha256(Buffer.from(await (await fetch(`${fake!.url}/api/v1/plugins/${id}/${version}/version.json`)).arrayBuffer()));
const entry = async (id: string, version: string): Promise<Entry> => ({ id, version, sha256: await fingerprint(id, version) });
const run = async (entries: Entry[], over: Partial<Parameters<typeof fetchSeed>[0]> = {}) => {
  const to = temp();
  const said = await fetchSeed({ entries, to, store: fake!.url, mirror: `${fake!.url}/mirror`, keys, app: '2.2.0', ...over });
  return { to, said };
};

describe.skipIf(!whole)('the seed of a release', () => {
  it('holds the bytes the store published, with their signature and the list of the day', async () => {
    const { to, said } = await run([await entry(TRELLO, '2.2.0')]);
    expect(fs.readdirSync(path.join(to, TRELLO)).sort()).toEqual(['archive.tar.gz', 'plugin.wasm', 'version.json', 'version.json.minisig']);
    for (const name of fs.readdirSync(path.join(to, TRELLO))) {
      const served = await fetch(`${fake!.url}/api/v1/plugins/${TRELLO}/2.2.0/${name}`);
      expect(fs.readFileSync(path.join(to, TRELLO, name)).equals(Buffer.from(await served.arrayBuffer()))).toBe(true);
    }
    expect(fs.existsSync(path.join(to, 'revocations.json.minisig'))).toBe(true);
    expect(said.at(-1)).toMatch(/revocation list no\. \d/);
  });

  it('asks nothing of the store while plugins/seed.json is empty, and keeps the README', async () => {
    const to = temp();
    fs.writeFileSync(path.join(to, 'README.md'), 'x');
    fs.writeFileSync(path.join(to, 'old.json'), 'x');
    const said = await fetchSeed({ entries: [], to, store: 'http://127.0.0.1:1', mirror: 'http://127.0.0.1:1', keys: [], app: '2.2.0' });
    expect(said).toEqual([]);
    expect(fs.readdirSync(to)).toEqual(['README.md']);
  });

  it('stops at a version the store has not published', async () => {
    await expect(run([{ id: TRELLO, version: '9.9.9', sha256: 'a'.repeat(64) }])).rejects.toThrow(/the chef publishes first/);
  });

  it('stops at a version.json that is not the one the list names', async () => {
    await expect(run([{ id: TRELLO, version: '2.2.0', sha256: 'a'.repeat(64) }])).rejects.toThrow(/not the one plugins\/seed.json names/);
  });

  it('stops at a version that is not signed', async () => {
    await expect(run([await entry('dev.escouade.slack', '1.1.0')])).rejects.toThrow(/is not signed/);
  });

  it('stops at a signature that does not hold with the keys, or signed for another version', async () => {
    const other = readPublicKey(await testPub())!;
    await expect(run([await entry(TRELLO, '2.2.0')], { keys: [other] })).rejects.toThrow(/not signed by a key/);
    await expect(run([await entry('dev.exemple.forged', '1.0.0')])).rejects.toThrow(/signed for another version/);
  });

  it('stops at an archive changed after its version.json', async () => {
    await expect(run([await entry('dev.escouade.seed-tampered', '1.0.0')])).rejects.toThrow(/its archive is not the one/);
  });

  it('stops at a version this Escouade cannot run', async () => {
    await expect(run([await entry('dev.escouade.seed-later', '1.0.0')])).rejects.toThrow(/asks for Escouade 99\.0\.0/);
  });

  it('stops at a version the list revokes', async () => {
    await post('/_fake/revocations', {
      entries: [{ id: TRELLO, versions: ['2.2.0'], action: 'disable', reason: { fr: 'Non.', en: 'No.' }, date: '2026-11-05' }],
    });
    await expect(run([await entry(TRELLO, '2.2.0')])).rejects.toThrow(/is revoked \(disable\): No\./);
    await post('/_fake/revocations', { entries: [] });
  });

  it('stops without a revocation list to carry', async () => {
    const down = 'http://127.0.0.1:1';
    const bare = await startFakeStore(0);
    try {
      const id = TRELLO;
      const e = { id, version: '2.2.0', sha256: await sha(bare.url, id) };
      await expect(run([e], { store: bare.url, mirror: down, keys: [readPublicKey(bare.key)!] })).rejects.toThrow(/no revocation list/);
    } finally {
      await bare.close();
    }
  });

  it('reads the list and the keys the repository holds', async () => {
    // The seed of the release (ESC-354): plugins of the first hand, each once, each with its version and the SHA-256 of
    // its version.json.
    const list = parseList(fs.readFileSync(path.join(ROOT, 'plugins', 'seed.json'), 'utf8'));
    expect(list.length).toBeGreaterThan(0);
    for (const e of list)
      expect(e).toMatchObject({ id: expect.stringMatching(/^dev\.escouade\./), version: expect.stringMatching(/^\d+\.\d+\.\d+$/) });
    expect(new Set(list.map((e) => e.id)).size).toBe(list.length);
    expect(() => parseList('{"format":2,"plugins":[]}')).toThrow(/format 1/);
    expect(() => parseList('{"format":1,"plugins":[{"id":"x"}]}')).toThrow(/SHA-256/);
    expect((await keysOf(path.join(ROOT, 'store', 'keys'))).map((k) => k.id).sort()).toEqual(['177096D57E2EC0C8', 'E0516468FFDCBDD5']);
  });
});

const sha = async (url: string, id: string) =>
  sha256(Buffer.from(await (await fetch(`${url}/api/v1/plugins/${id}/2.2.0/version.json`)).arrayBuffer()));
const testPub = async () => (await import('../fake/minisign.mjs')).testKey().pub;

describe.skipIf(!whole)('the seed of a build of development', () => {
  it('is made from a folder and its module, not signed, its list beside it', async () => {
    const to = temp();
    const folder = path.join(ROOT, 'tests', 'fixtures', 'plugins', 'reactor');
    const [e] = await devSeed([folder], to);
    expect(e!.id).toBe('dev.test.reactor');
    const dir = path.join(to, e!.id);
    expect(fs.readdirSync(dir).sort()).toEqual(['archive.tar.gz', 'plugin.wasm', 'version.json']);
    expect(sha256(fs.readFileSync(path.join(dir, 'version.json')))).toBe(e!.sha256);
    const v = JSON.parse(fs.readFileSync(path.join(dir, 'version.json'), 'utf8'));
    expect(v.archive.sha256).toBe(sha256(fs.readFileSync(path.join(dir, 'archive.tar.gz'))));
    expect(v.wasm.size).toBe(fs.statSync(path.join(folder, 'plugin.wasm')).size);
    expect(parseList(fs.readFileSync(path.join(to, 'seed.json'), 'utf8'))).toEqual([e]);
  });

  it('says a folder without its module', async () => {
    const folder = temp();
    fs.writeFileSync(path.join(folder, 'escouade-plugin.json'), '{"id":"dev.test.x","version":"1.0.0"}');
    await expect(devSeed([folder], temp())).rejects.toThrow(/no plugin.wasm/);
  });
});
