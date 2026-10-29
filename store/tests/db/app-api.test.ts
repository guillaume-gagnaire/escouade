import { gunzipSync } from 'node:zlib';
import pg from 'pg';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { accounts, plugins, versions } from '../../server/db/schema';
import { refuseDisabled } from '../../server/utils/app-api';
import { buildCatalog, catalogBytes, CATALOG_TTL, forgetCatalog } from '../../server/utils/catalog';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import { countInstall } from '../../server/utils/installs';
import { readPublicKey } from '../../server/utils/minisign';
import { REVOCATIONS_COMMENT } from '../../server/utils/revocation-list';
import { acceptRevocations, lastRevocations } from '../../server/utils/revocations';
import { sha256, versionFile } from '../../server/utils/versions';
import { testKey } from '../fake/minisign.mjs';

// The app's API against a real PostgreSQL (TEST_DATABASE_URL alone, emptied first; skipped without it): the catalogue
// (published versions only, compatibility, what each asks for, the counts of the days before, the versions the
// revocation list disables left out), the counter of installations, and the revocation list taken in only signed
// (docs/STORE.md §1.2, §4.6, §5.4, §8.1).
const url = process.env.TEST_DATABASE_URL;
const T0 = new Date('2026-10-29T10:00:00Z');

describe.skipIf(!url)('the app’s API', () => {
  let pool: pg.Pool;
  let db: Db;
  let account: number;
  const key = testKey();
  const keys = [readPublicKey(key.pub)!];

  /** A version of `id`, frozen as STORE-3 leaves it, in `state`. */
  const version = async (id: string, number: string, state = 'published', over: Record<string, unknown> = {}) => {
    const manifest = {
      id,
      name: { en: `Name ${id}`, fr: `Nom ${id}` },
      description: 'Sends a notification.',
      version: number,
      license: 'MIT',
      escouade: { api: '1', min: '2.1.0', ...(number.startsWith('2') ? { max: '2.9' } : {}) },
      permissions: { network: ['ntfy.sh'] },
      ...(number === '1.0.0' ? {} : { run: { command: 'bin/ntfy' } }),
      contributes: { webhooks: [{ id: 'n' }], settings: [] },
      ...over,
    };
    const file = versionFile({
      id,
      version: number,
      repository: 'https://github.com/sroux/escouade-ntfy',
      tag: `v${number}`,
      commit: 'c'.repeat(40),
      archive: { sha256: sha256(`${id}${number}`), size: 1234 },
      binaries: {},
      manifest_sha256: sha256(JSON.stringify(manifest)),
      escouade: manifest.escouade,
    });
    await db.insert(versions).values({
      pluginId: id,
      accountId: account,
      installationId: 1,
      repoId: 77,
      repoFullName: 'sroux/escouade-ntfy',
      tag: `v${number}`,
      commit: 'c'.repeat(40),
      version: number,
      state,
      manifest,
      archiveSha256: sha256(`${id}${number}`),
      archiveSize: 1234,
      binaries: number === '2.0.0' ? { 'linux-x64': { name: 'ntfy-linux-x64', sha256: 'b'.repeat(64), size: 9, attested: true } } : {},
      versionJson: file.json,
      versionSha256: file.sha256,
      publishedAt: state === 'published' ? new Date(T0.getTime() - 3_600_000 + Number(number[0]) * 60_000) : null,
    });
    return file;
  };
  const plugin = (id: string) =>
    db.insert(plugins).values({
      id,
      accountId: account,
      repoId: 77,
      repoFullName: 'sroux/escouade-ntfy',
      categories: ['notifications'],
      keywords: ['ntfy'],
    });
  const revocations = (serial: number, entries: unknown[]) => {
    const json = JSON.stringify({ serial, issued_at: T0.toISOString(), entries });
    return { json, minisig: key.sign(json, `${REVOCATIONS_COMMENT} ${serial} ${T0.toISOString()}`) };
  };

  beforeAll(async () => {
    const admin = new pg.Client({ connectionString: url });
    await admin.connect();
    await admin.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await admin.end();
    ({ pool, db } = openDb(url!));
    await migrateDb(db, './server/db/migrations');
  });
  beforeEach(async () => {
    await pool.query('truncate accounts, plugins, versions, installs_daily, revocation_lists cascade');
    [{ id: account }] = (await db.insert(accounts).values({ githubId: 4242, login: 'sroux' }).returning({ id: accounts.id })) as [
      { id: number },
    ];
    forgetCatalog();
  });
  afterAll(async () => {
    await pool?.end();
  });

  it('lists the published versions alone, the highest release as the latest, with what each needs and asks', async () => {
    await plugin('io.github.sroux.ntfy');
    await version('io.github.sroux.ntfy', '1.0.0');
    const v2 = await version('io.github.sroux.ntfy', '2.0.0');
    await version('io.github.sroux.ntfy', '2.1.0-beta.1');
    await version('io.github.sroux.ntfy', '2.2.0', 'ready');
    await version('io.github.sroux.ntfy', '2.3.0', 'rejected');
    const c = await buildCatalog(db, T0);
    expect(c.format).toBe(1);
    expect(c.plugins).toHaveLength(1);
    const p = c.plugins[0]!;
    expect(p).toMatchObject({
      id: 'io.github.sroux.ntfy',
      name: { en: 'Name io.github.sroux.ntfy', fr: 'Nom io.github.sroux.ntfy' },
      author: 'sroux',
      repository: 'https://github.com/sroux/escouade-ntfy',
      license: 'MIT',
      categories: ['notifications'],
      keywords: ['ntfy'],
      installs: 0,
      latest: '2.0.0',
    });
    expect(p.versions.map((v) => v.version)).toEqual(['2.1.0-beta.1', '2.0.0', '1.0.0']);
    const two = p.versions[1]!;
    expect(two).toMatchObject({
      sha256: v2.sha256,
      escouade: { api: '1', min: '2.1.0', max: '2.9' },
      verified: false,
      permissions: { network: ['ntfy.sh'] },
      run: true,
      contributes: ['settings', 'webhooks'],
      platforms: ['linux-x64'],
      files: {
        version_json: '/api/v1/plugins/io.github.sroux.ntfy/2.0.0/version.json',
        archive: '/api/v1/plugins/io.github.sroux.ntfy/2.0.0/archive.tar.gz',
        binaries: { 'linux-x64': '/api/v1/plugins/io.github.sroux.ntfy/2.0.0/binaries/linux-x64' },
      },
    });
    expect(p.versions[2]).toMatchObject({ run: false, platforms: [], escouade: { api: '1', min: '2.1.0' } });
    expect(p.versions[2]!.escouade).not.toHaveProperty('max');
    // The fingerprint is the SHA-256 of the bytes version.json is served as.
    const [row] = (await pool.query("select version_json from versions where version = '2.0.0'")).rows;
    expect(sha256(row.version_json)).toBe(two.sha256);
    expect(c.revocations).toBeNull();
  });

  it('counts the installations of published versions alone, the days before today in the catalogue', async () => {
    await plugin('io.github.sroux.ntfy');
    await version('io.github.sroux.ntfy', '1.0.0');
    await version('io.github.sroux.ntfy', '2.2.0', 'ready');
    const yesterday = new Date(T0.getTime() - 86_400_000);
    expect(await countInstall(db, { id: 'io.github.sroux.ntfy', version: '1.0.0', event: 'install' }, yesterday)).toBe(true);
    expect(await countInstall(db, { id: 'io.github.sroux.ntfy', version: '1.0.0', event: 'install' }, yesterday)).toBe(true);
    expect(await countInstall(db, { id: 'io.github.sroux.ntfy', version: '1.0.0', event: 'update' }, yesterday)).toBe(true);
    expect(await countInstall(db, { id: 'io.github.sroux.ntfy', version: '1.0.0', event: 'install' }, T0)).toBe(true);
    expect(await countInstall(db, { id: 'io.github.sroux.ntfy', version: '2.2.0', event: 'install' }, T0)).toBe(false);
    expect(await countInstall(db, { id: 'dev.nobody.here', version: '1.0.0', event: 'install' }, T0)).toBe(false);
    const rows = (await pool.query('select plugin_id, version, day::text, event, count from installs_daily order by day, event')).rows;
    // A counter by day and event, and nothing else: no address, no time, no id.
    expect(rows).toEqual([
      { plugin_id: 'io.github.sroux.ntfy', version: '1.0.0', day: '2026-10-28', event: 'install', count: 2 },
      { plugin_id: 'io.github.sroux.ntfy', version: '1.0.0', day: '2026-10-28', event: 'update', count: 1 },
      { plugin_id: 'io.github.sroux.ntfy', version: '1.0.0', day: '2026-10-29', event: 'install', count: 1 },
    ]);
    const c = await buildCatalog(db, T0);
    expect(c.plugins[0]!.installs).toBe(3);
    expect(c.installs_until).toBe('2026-10-28');
    expect(c.updated_at).toBe('2026-10-29T09:01:00.000Z');
    // The next day, today's installation joins the count, and the catalogue says it changed at midnight.
    const next = await buildCatalog(db, new Date(T0.getTime() + 86_400_000));
    expect(next.plugins[0]!.installs).toBe(4);
    expect(next.installs_until).toBe('2026-10-29');
    expect(next.updated_at).toBe('2026-10-30T00:00:00.000Z');
  });

  it('takes a revocation list in only signed, its serial only going up, and drops what it disables', async () => {
    await plugin('io.github.sroux.ntfy');
    await plugin('io.github.sroux.gone');
    await version('io.github.sroux.ntfy', '1.0.0');
    await version('io.github.sroux.ntfy', '2.0.0');
    await version('io.github.sroux.gone', '1.0.0');
    const reason = { fr: 'Envoie les messages ailleurs.', en: 'Sends the messages elsewhere.' };
    const entries = [
      { id: 'io.github.sroux.ntfy', versions: ['2.0.0'], action: 'disable', reason, date: '2026-10-29' },
      { id: 'io.github.sroux.ntfy', versions: '*', action: 'unverify', reason, date: '2026-10-29' },
      { id: 'io.github.sroux.gone', versions: '*', action: 'disable', reason, date: '2026-10-29' },
    ];
    const l3 = revocations(3, entries);
    // Signed by another key, or not at all: refused, nothing kept.
    const other = testKey();
    expect(await acceptRevocations(db, keys, l3.json, other.sign(l3.json, `${REVOCATIONS_COMMENT} 3`))).toEqual({
      error: 'not signed by a key of the store',
    });
    expect(await acceptRevocations(db, [], l3.json, l3.minisig)).toEqual({ error: 'not signed by a key of the store' });
    expect(await lastRevocations(db)).toBeNull();
    expect(await acceptRevocations(db, keys, l3.json, l3.minisig, T0)).toEqual({ serial: 3 });
    // A list replayed, or an older one, does not lift a revocation.
    expect(await acceptRevocations(db, keys, l3.json, l3.minisig)).toEqual({ error: 'serial 3 is not above 3' });
    const l2 = revocations(2, []);
    expect(await acceptRevocations(db, keys, l2.json, l2.minisig)).toEqual({ error: 'serial 2 is not above 3' });
    const last = await lastRevocations(db);
    expect(last).toMatchObject({ serial: 3, json: l3.json, minisig: l3.minisig });

    const c = await buildCatalog(db, T0);
    expect(c.revocations).toEqual({ serial: 3 });
    expect(c.plugins.map((p) => p.id)).toEqual(['io.github.sroux.ntfy']);
    expect(c.plugins[0]!.versions.map((v) => v.version)).toEqual(['1.0.0']);
    expect(c.plugins[0]!.latest).toBe('1.0.0');
    expect(c.plugins[0]!.versions[0]!.unverified).toEqual(reason);
    expect(c.updated_at).toBe(T0.toISOString());
    // Its files are no longer handed out: a 410 that says why; the others still are.
    await expect(refuseDisabled(db, 'io.github.sroux.ntfy', '2.0.0')).rejects.toMatchObject({ statusCode: 410, data: { reason } });
    await expect(refuseDisabled(db, 'io.github.sroux.gone', '1.0.0')).rejects.toMatchObject({ statusCode: 410 });
    await expect(refuseDisabled(db, 'io.github.sroux.ntfy', '1.0.0')).resolves.toBeUndefined();

    const l4 = revocations(4, []);
    expect(await acceptRevocations(db, keys, l4.json, l4.minisig)).toEqual({ serial: 4 });
    forgetCatalog();
    expect((await buildCatalog(db, T0)).plugins.map((p) => p.id)).toEqual(['io.github.sroux.gone', 'io.github.sroux.ntfy']);
  });

  it('serves the same bytes and ETag until something changes, compressed, built again after a minute', async () => {
    await plugin('io.github.sroux.ntfy');
    await version('io.github.sroux.ntfy', '1.0.0');
    const a = await catalogBytes(db, T0);
    expect(a.etag).toBe(`"${sha256(a.body)}"`);
    expect(gunzipSync(a.gzip).toString()).toBe(a.body);
    await version('io.github.sroux.ntfy', '2.0.0');
    // Within the minute, the built one; after it, the new version, a new ETag.
    expect((await catalogBytes(db, new Date(T0.getTime() + 1000))).etag).toBe(a.etag);
    const b = await catalogBytes(db, new Date(T0.getTime() + CATALOG_TTL));
    expect(b.etag).not.toBe(a.etag);
    // Built again with nothing changed: the same bytes, the same ETag (no time of building in them).
    forgetCatalog();
    expect((await catalogBytes(db, new Date(T0.getTime() + 2 * CATALOG_TTL))).etag).toBe(b.etag);
  });
});
