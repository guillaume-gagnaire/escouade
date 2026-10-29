import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { REVOCATIONS_COMMENT } from '../../server/utils/revocation-list';
import { testKey } from '../fake/minisign.mjs';

// The copy of the revocation list on escouade.dev (scripts/copy-revocations.ts), run as the workflow runs it, against
// a store faked here: copied only signed and above the copy's serial, byte for byte with its signature; the store
// down leaves the copy as it is.
const run = promisify(execFile);
const key = testKey();
const list = (serial: number) => {
  const json = JSON.stringify({ serial, issued_at: '2026-10-29T10:00:00Z', entries: [] });
  return { json, minisig: key.sign(json, `${REVOCATIONS_COMMENT} ${serial} 2026-10-29T10:00:00Z`) };
};

describe('the copy of the revocation list', () => {
  let server: Server;
  let base: string;
  let dir: string;
  let served: { json: string; minisig: string } | null | 'down';

  const copy = async (keys = join(dir, 'keys')) => {
    try {
      const { stdout } = await run(
        process.execPath,
        ['node_modules/tsx/dist/cli.mjs', 'scripts/copy-revocations.ts', '--store', base, '--out', join(dir, 'out'), '--keys', keys],
        { env: { ...process.env, GITHUB_OUTPUT: '' } },
      );
      return { code: 0, out: stdout };
    } catch (e) {
      const x = e as { code: number; stdout: string; stderr: string };
      return { code: x.code, out: x.stdout + x.stderr };
    }
  };
  const copied = () => readFile(join(dir, 'out', 'revocations.json'), 'utf8').catch(() => null);

  beforeAll(async () => {
    server = createServer((req, res) => {
      if (served === 'down') return res.writeHead(502).end();
      const body =
        req.url === '/api/v1/revocations.json' ? served?.json : req.url === '/api/v1/revocations.json.minisig' ? served?.minisig : null;
      if (body == null) return res.writeHead(404).end();
      res.writeHead(200).end(body);
    });
    await new Promise<void>((done) => server.listen(0, '127.0.0.1', done));
    base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  });
  beforeEach(async () => {
    if (dir) await rm(dir, { recursive: true, force: true });
    dir = await mkdtemp(join(tmpdir(), 'revocations-copy-'));
    await mkdir(join(dir, 'out'));
    await mkdir(join(dir, 'keys'));
    await writeFile(join(dir, 'keys', 'store.pub'), key.pub);
  });
  afterAll(async () => {
    server?.close();
    if (dir) await rm(dir, { recursive: true, force: true });
  });

  it('copies a signed list byte for byte with its signature, then only a higher serial', async () => {
    served = list(3);
    expect((await copy()).out).toContain('changed=true');
    expect(await copied()).toBe(served.json);
    expect(await readFile(join(dir, 'out', 'revocations.json.minisig'), 'utf8')).toBe(served.minisig);
    expect((await copy()).out).toContain('changed=false');
    served = list(4);
    expect((await copy()).out).toContain('changed=true');
    expect(await copied()).toBe(served.json);
  }, 60_000);

  it('refuses an older list, or one signed by another key, and fails so that GitHub tells', async () => {
    served = list(5);
    await copy();
    served = list(2);
    const older = await copy();
    expect(older.code).toBe(1);
    expect(older.out).toContain('serial 2 is not above 5');
    const json = JSON.stringify({ serial: 9, issued_at: '2026-10-29T10:00:00Z', entries: [] });
    served = { json, minisig: testKey().sign(json, `${REVOCATIONS_COMMENT} 9`) };
    const forged = await copy();
    expect(forged.code).toBe(1);
    expect(forged.out).toContain('not signed by a key of the store');
    expect(JSON.parse((await copied())!).serial).toBe(5);
  }, 60_000);

  it('leaves the copy as it is when the store is down, has no list, or no key is there', async () => {
    served = 'down';
    expect(await copy()).toMatchObject({ code: 0, out: expect.stringContaining('changed=false') });
    served = null;
    expect(await copy()).toMatchObject({ code: 0, out: expect.stringContaining('no revocation list yet') });
    served = list(3);
    expect(await copy(join(dir, 'nowhere'))).toMatchObject({ code: 0, out: expect.stringContaining('No public key') });
    expect(await copied()).toBeNull();
  }, 60_000);
});
