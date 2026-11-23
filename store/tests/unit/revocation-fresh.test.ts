import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { readPublicKey } from '../../server/utils/minisign';
import {
  attest,
  checkFreshness,
  createFreshnessKey,
  FRESH_COMMENT,
  minisignWith,
  publicOf,
  readFreshnessKey,
} from '../../server/utils/revocation-fresh';
import { REVOCATIONS_COMMENT } from '../../server/utils/revocation-list';

// The attestation of freshness of the revocation list (audit SEC-15, ESC-596): made by the freshness key, read as the app
// reads it (tests/fixtures/plugins/revocations-fresh.json is checked here and by the app's tests), and the weekly step of
// the workflow (scripts/attest-revocations.ts) run as the workflow runs it, with and without its secret. No network.
const run = promisify(execFile);
const vectors = JSON.parse(readFileSync(join(__dirname, '../../../tests/fixtures/plugins/revocations-fresh.json'), 'utf8'));

describe('the attestation of freshness', () => {
  it('holds for its own list alone, and the app’s vectors check here too', () => {
    const fresh = [readPublicKey(vectors.freshKey)!];
    expect(checkFreshness(vectors.list, 3, vectors.good.json, vectors.good.minisig, fresh)).toEqual({ validAt: '2026-11-16T06:00:00Z' });
    expect(checkFreshness(vectors.list, 3, vectors.other.json, vectors.other.minisig, fresh)).toEqual({
      error: 'it attests another list (its fingerprint)',
    });
    expect(checkFreshness(vectors.list, 3, vectors.serial4.json, vectors.serial4.minisig, fresh)).toHaveProperty('error');
    expect(checkFreshness(vectors.list, 3, vectors.byListKey.json, vectors.byListKey.minisig, fresh)).toEqual({
      error: 'not signed by a freshness key',
    });
  });

  it('is made with a key that never prints its secret, and its comment says the serial and the date', () => {
    const k = createFreshnessKey();
    expect(k.secret).toMatch(/^escouade-freshness-key-v1 [0-9A-F]{16}\n-----BEGIN PRIVATE/);
    expect(readPublicKey(k.publicKey)?.id).toBe(k.id);
    const key = readFreshnessKey(k.secret)!;
    const list = JSON.stringify({ serial: 7, issued_at: '2026-11-01T00:00:00Z', entries: [] });
    const a = attest(key, list, 7, new Date('2026-11-23T06:00:12.345Z'));
    expect(a.minisig).toContain(`trusted comment: ${FRESH_COMMENT} 7 2026-11-23T06:00:12Z\n`);
    expect(checkFreshness(list, 7, a.json, a.minisig, [publicOf(key)])).toEqual({ validAt: '2026-11-23T06:00:12Z' });
    expect(readFreshnessKey('nope')).toBeNull();
  });
});

describe('the weekly step of revocations.yml', () => {
  let dir: string;
  const listKey = createFreshnessKey();
  const signer = readFreshnessKey(listKey.secret)!;
  const freshKey = createFreshnessKey();

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), 'revocations-attest-'));
    for (const d of ['out', 'keys', 'fresh']) await mkdir(join(dir, d));
    // The revocation key (ESC-592): the third parties', which signs the lists.
    await writeFile(join(dir, 'keys', 'third-party.pub'), listKey.publicKey);
    await writeFile(join(dir, 'fresh', 'freshness.pub'), freshKey.publicKey);
    const json = `${JSON.stringify({ serial: 5, issued_at: '2026-11-01T00:00:00Z', entries: [] }, null, 2)}\n`;
    await writeFile(join(dir, 'out', 'revocations.json'), json);
    await writeFile(
      join(dir, 'out', 'revocations.json.minisig'),
      minisignWith(signer, json, `${REVOCATIONS_COMMENT} 5 2026-11-01T00:00:00Z`),
    );
  });
  afterEach(() => rm(dir, { recursive: true, force: true }));

  const step = async (secret: string) => {
    const summary = join(dir, 'summary.md');
    try {
      const { stdout } = await run(
        process.execPath,
        [
          'node_modules/tsx/dist/cli.mjs',
          'scripts/attest-revocations.ts',
          '--out',
          join(dir, 'out'),
          '--keys',
          join(dir, 'keys'),
          '--fresh',
          join(dir, 'fresh'),
        ],
        { env: { ...process.env, GITHUB_OUTPUT: '', GITHUB_STEP_SUMMARY: summary, REVOCATIONS_FRESHNESS_KEY: secret } },
      );
      return { code: 0, out: stdout, summary: await readFile(summary, 'utf8').catch(() => '') };
    } catch (e) {
      const x = e as { code: number; stdout: string; stderr: string };
      return { code: x.code, out: x.stdout + x.stderr, summary: '' };
    }
  };

  it('without its secret, signs nothing, fails nothing, and says so in its summary', async () => {
    const r = await step('');
    expect(r.code).toBe(0);
    expect(r.out).toContain('changed=false');
    expect(r.summary).toContain('republished as it was signed, without an attestation of freshness');
    await expect(readFile(join(dir, 'out', 'revocations-fresh.json'))).rejects.toThrow();
  });

  it('with it, attests the list copied, which the app then takes', async () => {
    const r = await step(freshKey.secret);
    expect(r.code, r.out).toBe(0);
    expect(r.out).toContain('changed=true');
    expect(r.out).not.toContain('BEGIN PRIVATE');
    const list = await readFile(join(dir, 'out', 'revocations.json'), 'utf8');
    const json = await readFile(join(dir, 'out', 'revocations-fresh.json'), 'utf8');
    const minisig = await readFile(join(dir, 'out', 'revocations-fresh.json.minisig'), 'utf8');
    expect(checkFreshness(list, 5, json, minisig, [readPublicKey(freshKey.publicKey)!])).toHaveProperty('validAt');
  });

  it('attests only a list of the revocation key: one of the first hand’s key is refused', async () => {
    await rm(join(dir, 'keys', 'third-party.pub'));
    await writeFile(join(dir, 'keys', 'store.pub'), listKey.publicKey);
    const r = await step(freshKey.secret);
    expect(r.code).toBe(1);
    expect(r.out).toContain('which signs no revocation list');
    await expect(readFile(join(dir, 'out', 'revocations-fresh.json'))).rejects.toThrow();
  });

  it('refuses a secret that does not read, and a list that does not hold', async () => {
    expect((await step('not a key')).code).toBe(1);
    await writeFile(join(dir, 'out', 'revocations.json'), '{"serial":5}');
    expect((await step(freshKey.secret)).code).toBe(1);
  });
});
