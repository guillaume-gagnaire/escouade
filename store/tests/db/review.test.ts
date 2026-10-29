import { sql } from 'drizzle-orm';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';
import pg from 'pg';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { accounts, adminTokens, installsDaily, plugins, reports, signatures, versions } from '../../server/db/schema';
import { buildCatalog, forgetCatalog } from '../../server/utils/catalog';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import { catalog as listing, pluginPage } from '../../server/utils/listing';
import { readPublicKey } from '../../server/utils/minisign';
import {
  acceptSignature,
  adminTokensOf,
  closeReport,
  decide,
  depositRevocations,
  makeAdminToken,
  postReport,
  queueCount,
  reviewQueue,
  revokeAdminToken,
  settleHeld,
  tokenAdmin,
} from '../../server/utils/review';
import { REVOCATIONS_COMMENT } from '../../server/utils/revocation-list';
import { verifiedComment } from '../../server/utils/signature';
import { sha256, versionFile } from '../../server/utils/versions';
import { testKey } from '../fake/minisign.mjs';

// The review queue and what the review sends (docs/STORE.md §4.1, §4.3, §4.5, §4.6) against a real PostgreSQL
// (TEST_DATABASE_URL alone, emptied first; skipped without it): the administration tokens, the signatures checked with
// the public keys before they make a version verified, the queue's order, the answers to the author, the reports, the
// names too close, and the revocation lists. Keys made for the test, never a file of the repository.
const url = process.env.TEST_DATABASE_URL;
const T0 = new Date('2026-10-29T10:00:00Z');
const ADMIN = 101;
const env = { STORE_ADMINS: `${ADMIN}` };

/** A tar.gz of `files`, as GitHub sends one (a top folder). */
function tarGz(files: Record<string, string>): Buffer {
  const blocks: Buffer[] = [];
  for (const [name, body] of Object.entries(files)) {
    const h = Buffer.alloc(512);
    h.write(`top/${name}`, 0, 'utf8');
    h.write('0000644\0', 100);
    h.write('0000000\0', 108);
    h.write('0000000\0', 116);
    h.write(`${Buffer.byteLength(body).toString(8).padStart(11, '0')}\0`, 124);
    h.write('00000000000\0', 136);
    h.write('        ', 148);
    h.write('0', 156);
    h.write('ustar\0', 257);
    h.write('00', 263);
    let sum = 0;
    for (const b of h) sum += b;
    h.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148);
    const data = Buffer.from(body);
    blocks.push(h, data, Buffer.alloc((512 - (data.length % 512)) % 512));
  }
  blocks.push(Buffer.alloc(1024));
  return gzipSync(Buffer.concat(blocks));
}

describe.skipIf(!url)('the review queue and the signatures', () => {
  let pool: pg.Pool;
  let db: Db;
  let author: number;
  let admin: typeof accounts.$inferSelect;
  const archives = mkdtempSync(join(tmpdir(), 'store-review-'));
  const key = testKey();
  const backup = testKey();
  const stranger = testKey();
  const keys = [readPublicKey(key.pub)!, readPublicKey(backup.pub)!];

  /** A plugin of the author. */
  const plugin = (id: string) =>
    db.insert(plugins).values({ id, accountId: author, repoId: 77, repoFullName: 'sroux/escouade-ntfy', categories: ['notifications'] });

  /** A version of `id`, frozen as STORE-3 leaves it; its archive kept on the volume. */
  const version = async (
    id: string,
    number: string,
    o: {
      state?: string;
      review?: boolean;
      network?: string[];
      files?: Record<string, string>;
      binaries?: Record<string, unknown>;
      at?: Date;
    } = {},
  ) => {
    const manifest = {
      id,
      name: { en: `Name ${id}`, fr: `Nom ${id}` },
      description: 'Sends a notification.',
      version: number,
      license: 'MIT',
      escouade: { api: '1', min: '2.1.0' },
      permissions: { network: o.network ?? ['ntfy.sh'] },
    };
    const gz = tarGz(o.files ?? { 'escouade-plugin.json': JSON.stringify(manifest), 'README.md': `# ${id} ${number}\n` });
    const archiveSha = sha256(gz);
    mkdirSync(join(archives, 'sha256', archiveSha.slice(0, 2)), { recursive: true });
    writeFileSync(join(archives, 'sha256', archiveSha.slice(0, 2), `${archiveSha}.tar.gz`), gz);
    const file = versionFile({
      id,
      version: number,
      repository: 'https://github.com/sroux/escouade-ntfy',
      tag: `v${number}`,
      commit: 'c'.repeat(40),
      archive: { sha256: archiveSha, size: gz.length },
      binaries: {},
      manifest_sha256: sha256(JSON.stringify(manifest)),
      escouade: manifest.escouade,
    });
    const [row] = await db
      .insert(versions)
      .values({
        pluginId: id,
        accountId: author,
        installationId: 1,
        repoId: 77,
        repoFullName: 'sroux/escouade-ntfy',
        tag: `v${number}`,
        commit: 'c'.repeat(40),
        version: number,
        state: o.state ?? 'published',
        review: o.review ?? true,
        manifest,
        archiveSha256: archiveSha,
        archiveSize: gz.length,
        binaries: o.binaries ?? {},
        versionJson: file.json,
        versionSha256: file.sha256,
        publishedAt: (o.state ?? 'published') === 'published' ? (o.at ?? T0) : null,
        createdAt: o.at ?? T0,
      })
      .returning({ id: versions.id });
    return { ...file, versionId: row!.id };
  };
  const sign = (json: string, id: string, number: string, by = key) => by.sign(json, verifiedComment(id, number, '2026-10-29'));
  const revocations = (serial: number, entries: unknown[], extra: Record<string, unknown> = {}, by = key) => {
    const json = JSON.stringify({ serial, issued_at: T0.toISOString(), entries, ...extra });
    return { json, minisig: by.sign(json, `${REVOCATIONS_COMMENT} ${serial} ${T0.toISOString()}`) };
  };

  beforeAll(async () => {
    const c = new pg.Client({ connectionString: url });
    await c.connect();
    await c.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await c.end();
    ({ pool, db } = openDb(url!));
    await migrateDb(db, './server/db/migrations');
  });
  beforeEach(async () => {
    await pool.query('truncate accounts, plugins, versions, installs_daily, revocation_lists, reports, signatures, admin_tokens cascade');
    [{ id: author }] = (await db.insert(accounts).values({ githubId: 4242, login: 'sroux' }).returning({ id: accounts.id })) as [
      { id: number },
    ];
    [admin] = await db.insert(accounts).values({ githubId: ADMIN, login: 'gagnaire' }).returning();
    forgetCatalog();
  });
  afterAll(async () => {
    await pool?.end();
  });

  describe('the administration tokens', () => {
    it('are shown once, kept as their hash, and act only for an administrator', async () => {
      const made = await makeAdminToken(db, admin, 'Mon poste', T0);
      if ('error' in made) throw new Error(made.error);
      expect(made.token).toMatch(/^esc_admin_[A-Za-z0-9_-]{43}$/);
      const [row] = await db.select().from(adminTokens);
      expect(row!.id).toBe(sha256(made.token));
      expect(JSON.stringify(row)).not.toContain(made.token);
      expect((await tokenAdmin(db, env, `Bearer ${made.token}`))?.id).toBe(admin.id);
      // No longer an administrator: the token does nothing.
      expect(await tokenAdmin(db, { STORE_ADMINS: '' }, `Bearer ${made.token}`)).toBeNull();
      expect(await tokenAdmin(db, env, made.token)).toBeNull();
      expect(await tokenAdmin(db, env, `Bearer ${made.token}x`)).toBeNull();
      const [line] = await adminTokensOf(db, admin);
      expect(line).toMatchObject({ label: 'Mon poste', hint: `${made.token.slice(0, 14)}…` });
      expect(await revokeAdminToken(db, admin, line!.ref)).toBe(true);
      expect(await tokenAdmin(db, env, `Bearer ${made.token}`)).toBeNull();
      expect(await adminTokensOf(db, admin)).toEqual([]);
    });

    it('are five at most, and need a label', async () => {
      expect(await makeAdminToken(db, admin, '   ')).toEqual({ error: 'label' });
      for (let i = 0; i < 5; i++) expect('token' in (await makeAdminToken(db, admin, `t${i}`))).toBe(true);
      expect(await makeAdminToken(db, admin, 'one more')).toEqual({ error: 'too-many' });
    });
  });

  describe('a signature', () => {
    it('makes the version verified in the catalogue and on the pages, with its summary, once checked', async () => {
      await plugin('io.github.sroux.ntfy');
      const v1 = await version('io.github.sroux.ntfy', '1.0.0');
      const summary = { fr: '- Réseau vers ntfy.sh seulement', en: '- Network to ntfy.sh only' };
      const done = await acceptSignature(
        db,
        admin,
        keys,
        { id: 'io.github.sroux.ntfy', version: '1.0.0', minisig: sign(v1.json, 'io.github.sroux.ntfy', '1.0.0'), summary },
        null,
        T0,
      );
      expect(done).toMatchObject({ ok: true, keyId: key.id, signedAt: '2026-10-29T00:00:00.000Z' });
      const c = await buildCatalog(db, T0);
      const entry = c.plugins[0]!.versions[0]!;
      expect(entry).toMatchObject({
        verified: true,
        review: { summary },
        files: { signature: '/api/v1/plugins/io.github.sroux.ntfy/1.0.0/version.json.minisig' },
      });
      const [e] = (await listing(db)).filter((x) => x.id === 'io.github.sroux.ntfy');
      expect(e!.trust).toBe('verified');
      const page = await pluginPage(db, archives, 'io.github.sroux.ntfy');
      expect(page!.latest!.signature).toMatchObject({ keyId: key.id, summary });
      expect(page!.versions[0]!.trust).toBe('verified');
      const [row] = await db.select().from(versions);
      expect(row!.reviewState).toBe('verified');
      // The backup key signs too (§4.5): the app embeds both.
      const v2 = await version('io.github.sroux.ntfy', '1.1.0');
      expect(
        await acceptSignature(
          db,
          admin,
          keys,
          { id: 'io.github.sroux.ntfy', version: '1.1.0', minisig: sign(v2.json, 'io.github.sroux.ntfy', '1.1.0', backup) },
          null,
        ),
      ).toMatchObject({ ok: true, keyId: backup.id });
    });

    it('is refused when it is not the store’s key, not these bytes, or names another version', async () => {
      await plugin('io.github.sroux.ntfy');
      const v1 = await version('io.github.sroux.ntfy', '1.0.0');
      const v2 = await version('io.github.sroux.ntfy', '1.1.0');
      const at = (minisig: string, number = '1.0.0') =>
        acceptSignature(db, admin, keys, { id: 'io.github.sroux.ntfy', version: number, minisig }, null);
      expect(await at(sign(v1.json, 'io.github.sroux.ntfy', '1.0.0', stranger))).toMatchObject({ error: 'signature' });
      expect(await at(sign(`${v1.json} `, 'io.github.sroux.ntfy', '1.0.0'))).toMatchObject({ error: 'signature' });
      // A verified version.json served under another number: its comment does not name it.
      expect(await at(sign(v1.json, 'io.github.sroux.ntfy', '1.0.0'), '1.1.0')).toMatchObject({ error: 'signature' });
      expect(await at(sign(v2.json, 'io.github.sroux.ntfy', '1.0.0'), '1.1.0')).toMatchObject({
        error: 'signature',
        detail: expect.stringContaining('trusted comment'),
      });
      expect(await at(key.sign(v1.json, 'escouade-verified-v1 io.github.sroux.ntfy@1.0.0'))).toMatchObject({ error: 'signature' });
      expect(await at('not a signature')).toMatchObject({ error: 'signature' });
      expect(
        await acceptSignature(
          db,
          admin,
          [],
          { id: 'io.github.sroux.ntfy', version: '1.0.0', minisig: sign(v1.json, 'io.github.sroux.ntfy', '1.0.0') },
          null,
        ),
      ).toEqual({
        error: 'no-keys',
      });
      expect(await at(sign(v1.json, 'io.github.sroux.ntfy', '1.0.0'), '9.9.9')).toEqual({ error: 'not-found' });
      expect(await db.select().from(signatures)).toEqual([]);
      expect((await buildCatalog(db, T0)).plugins[0]!.versions.every((v) => !v.verified)).toBe(true);
    });

    it('is refused for a binary without a provenance attestation, a disabled version, or a revoked key', async () => {
      await plugin('io.github.sroux.ntfy');
      const bin = { 'linux-x64': { name: 'ntfy', sha256: 'b'.repeat(64), size: 9, attested: false } };
      const v1 = await version('io.github.sroux.ntfy', '1.0.0', { binaries: bin });
      expect(
        await acceptSignature(
          db,
          admin,
          keys,
          { id: 'io.github.sroux.ntfy', version: '1.0.0', minisig: sign(v1.json, 'io.github.sroux.ntfy', '1.0.0') },
          null,
        ),
      ).toEqual({ error: 'unattested', detail: 'linux-x64' });
      const v2 = await version('io.github.sroux.ntfy', '1.1.0');
      const list = revocations(1, [], { keys_revoked: [key.id] }, backup);
      expect(await depositRevocations(db, admin, keys, list, null)).toEqual({ serial: 1 });
      expect(
        await acceptSignature(
          db,
          admin,
          keys,
          { id: 'io.github.sroux.ntfy', version: '1.1.0', minisig: sign(v2.json, 'io.github.sroux.ntfy', '1.1.0') },
          null,
        ),
      ).toEqual({ error: 'signature', detail: 'signed by a key the revocation list revokes' });
      const off = revocations(
        2,
        [{ id: 'io.github.sroux.ntfy', versions: ['1.1.0'], action: 'disable', reason: { fr: 'x', en: 'x' }, date: '2026-10-29' }],
        {},
        backup,
      );
      expect(await depositRevocations(db, admin, keys, off, null)).toEqual({ serial: 2 });
      expect(
        await acceptSignature(
          db,
          admin,
          keys,
          { id: 'io.github.sroux.ntfy', version: '1.1.0', minisig: sign(v2.json, 'io.github.sroux.ntfy', '1.1.0', backup) },
          null,
        ),
      ).toEqual({ error: 'disabled' });
    });

    it('loses its seal by an unverify entry, or when its key is revoked, and keeps it through a plugin’s purge', async () => {
      await plugin('io.github.sroux.ntfy');
      const v1 = await version('io.github.sroux.ntfy', '1.0.0');
      await acceptSignature(
        db,
        admin,
        keys,
        { id: 'io.github.sroux.ntfy', version: '1.0.0', minisig: sign(v1.json, 'io.github.sroux.ntfy', '1.0.0') },
        null,
      );
      const reason = { fr: 'Envoie le jeton à un autre hôte', en: 'Sends the token to another host' };
      const list = revocations(1, [{ id: 'io.github.sroux.ntfy', versions: '*', action: 'unverify', reason, date: '2026-10-29' }]);
      expect(await depositRevocations(db, admin, keys, list, null)).toEqual({ serial: 1 });
      const entry = (await buildCatalog(db, T0)).plugins[0]!.versions[0]!;
      expect(entry).toMatchObject({ verified: false, unverified: reason });
      expect(entry.files.signature).toBeUndefined();
      expect(await depositRevocations(db, admin, keys, revocations(2, [], { keys_revoked: [key.id] }, backup), null)).toEqual({
        serial: 2,
      });
      expect((await buildCatalog(db, T0)).plugins[0]!.versions[0]!.verified).toBe(false);
      await pool.query('delete from versions');
      expect(await db.select().from(signatures)).toHaveLength(1);
    });
  });

  describe('the revocation lists', () => {
    it('go up by serial, and the emergency withdrawal takes a version out of the catalogue', async () => {
      await plugin('io.github.sroux.ntfy');
      await version('io.github.sroux.ntfy', '1.0.0');
      await version('io.github.sroux.ntfy', '1.1.0');
      const reason = { fr: 'Malveillant', en: 'Malicious' };
      const off = revocations(3, [{ id: 'io.github.sroux.ntfy', versions: ['1.1.0'], action: 'disable', reason, date: '2026-10-29' }]);
      expect(await depositRevocations(db, admin, keys, off, null)).toEqual({ serial: 3 });
      expect((await buildCatalog(db, T0)).plugins[0]!.versions.map((v) => v.version)).toEqual(['1.0.0']);
      expect(await depositRevocations(db, admin, keys, revocations(3, []), null)).toMatchObject({
        error: expect.stringContaining('not above 3'),
      });
      expect(await depositRevocations(db, admin, keys, revocations(2, []), null)).toMatchObject({
        error: expect.stringContaining('not above'),
      });
      expect(await depositRevocations(db, admin, keys, revocations(4, [], {}, stranger), null)).toEqual({
        error: 'not signed by a key of the store',
      });
      expect(await depositRevocations(db, admin, [], revocations(4, []), null)).toMatchObject({
        error: expect.stringContaining('no public key'),
      });
      expect(await depositRevocations(db, admin, keys, { json: 1, minisig: 2 }, null)).toMatchObject({ error: expect.any(String) });
    });
  });

  describe('the queue', () => {
    it('lists the versions asked for review, the most installed first then the oldest, with what grew since the last verified', async () => {
      await plugin('io.github.sroux.ntfy');
      await plugin('io.github.sroux.teams');
      await plugin('io.github.sroux.goose');
      const base = await version('io.github.sroux.ntfy', '1.0.0', { at: new Date('2026-10-01T00:00:00Z') });
      await acceptSignature(
        db,
        admin,
        keys,
        { id: 'io.github.sroux.ntfy', version: '1.0.0', minisig: sign(base.json, 'io.github.sroux.ntfy', '1.0.0') },
        null,
      );
      await version('io.github.sroux.ntfy', '1.1.0', {
        network: ['ntfy.sh', 'api.example.com'],
        files: { 'escouade-plugin.json': '{}', 'README.md': '# a\nnew line\n', 'src/x.ts': 'x\n' },
        at: new Date('2026-10-27T00:00:00Z'),
      });
      await version('io.github.sroux.teams', '0.4.0', { at: new Date('2026-10-26T00:00:00Z') });
      await version('io.github.sroux.goose', '0.2.1', { at: new Date('2026-10-20T00:00:00Z') });
      await version('io.github.sroux.goose', '0.3.0', { review: false, at: new Date('2026-10-28T00:00:00Z') });
      await db
        .insert(installsDaily)
        .values({ pluginId: 'io.github.sroux.ntfy', version: '1.0.0', day: '2026-10-20', event: 'install', count: 214 });
      const q = await reviewQueue(db, archives, keys);
      expect(q.toReview.map((v) => `${v.id}@${v.version}`)).toEqual([
        'io.github.sroux.ntfy@1.1.0',
        'io.github.sroux.goose@0.2.1',
        'io.github.sroux.teams@0.4.0',
      ]);
      const up = q.toReview[0]!;
      expect(up).toMatchObject({
        previous: '1.0.0',
        base: { version: '1.0.0' },
        installs: 214,
        command: 'npm run plugin-review -- io.github.sroux.ntfy@1.1.0',
      });
      expect(up.permissions.filter((p) => p.isNew).map((p) => p.mono)).toEqual([expect.stringContaining('api.example.com')]);
      expect(up.diff).toMatchObject({ files: 3, paths: ['README.md', 'escouade-plugin.json', 'src/x.ts'] });
      expect(up.diff!.added).toBeGreaterThan(0);
      expect(q.toReview[1]).toMatchObject({ previous: null, base: null, diff: null });
      expect(q.done.map((v) => `${v.id}@${v.version}:${v.state}`)).toEqual(['io.github.sroux.ntfy@1.0.0:verified']);
      expect(q.keys).toBe(2);
      expect(await queueCount(db)).toBe(3);
    });

    it('takes the report of the script, and the answers of the page move a version to « Faites » and tell why', async () => {
      await plugin('io.github.sroux.ntfy');
      const v = await version('io.github.sroux.ntfy', '1.0.0');
      expect(
        await postReport(db, admin, { id: 'io.github.sroux.ntfy', version: '1.0.0', verdict: 'Vérifier', report: '# Rapport\n' }),
      ).toEqual({ ok: true });
      expect(await postReport(db, admin, { id: 'io.github.sroux.ntfy', version: '1.0.0', verdict: 'Peut-être', report: 'x' })).toEqual({
        error: 'verdict',
      });
      expect(await postReport(db, admin, { id: 'io.github.sroux.ntfy', version: '1.0.0', report: 'x'.repeat(300 * 1024) })).toEqual({
        error: 'report',
      });
      expect(await postReport(db, admin, { id: 'nope', version: '1.0.0', report: 'x' })).toEqual({ error: 'not-found' });
      let q = await reviewQueue(db, archives, keys);
      expect(q.toReview[0]!.report).toMatchObject({ verdict: 'Vérifier', text: '# Rapport\n' });
      expect(await decide(db, admin, v.versionId, 'changes', 'court', null)).toEqual({ error: 'message' });
      expect(await decide(db, admin, v.versionId, 'changes', 'Déclare api.example.com dans permissions.network.', null)).toEqual({
        ok: true,
      });
      q = await reviewQueue(db, archives, keys);
      expect(q.toReview).toEqual([]);
      expect(q.done[0]).toMatchObject({ state: 'changes', message: 'Déclare api.example.com dans permissions.network.' });
      // A signature afterwards verifies it all the same.
      await acceptSignature(
        db,
        admin,
        keys,
        { id: 'io.github.sroux.ntfy', version: '1.0.0', minisig: sign(v.json, 'io.github.sroux.ntfy', '1.0.0') },
        null,
      );
      expect(await decide(db, admin, v.versionId, 'refused', 'Trop tard pour refuser celle-ci.', null)).toEqual({ error: 'verified' });
    });

    it('lists the reports, several « malicious » first, and answers their sender when closed', async () => {
      await plugin('io.github.sroux.ntfy');
      await plugin('io.github.sroux.teams');
      await version('io.github.sroux.ntfy', '1.0.0');
      await version('io.github.sroux.teams', '1.0.0');
      const report = (pluginId: string, reason: string, at: string) =>
        db
          .insert(reports)
          .values({ pluginId, reason, details: 'x'.repeat(30), email: 'a@b.c', createdAt: new Date(at) })
          .returning({ id: reports.id });
      const [first] = await report('io.github.sroux.teams', 'license', '2026-10-20T00:00:00Z');
      await report('io.github.sroux.ntfy', 'malicious', '2026-10-27T00:00:00Z');
      await report('io.github.sroux.ntfy', 'malicious', '2026-10-28T00:00:00Z');
      const q = await reviewQueue(db, archives, keys);
      expect(q.reports.map((r) => `${r.id}:${r.reason}`)).toEqual([
        'io.github.sroux.ntfy:malicious',
        'io.github.sroux.ntfy:malicious',
        'io.github.sroux.teams:license',
      ]);
      expect(q.reports[0]).toMatchObject({ open: 2, malicious: 2 });
      expect(JSON.stringify(q.reports)).not.toContain('a@b.c');
      expect(await closeReport(db, admin, first!.id, 'dismissed', 'La licence est bien MIT.', null)).toEqual({ ok: true });
      expect(await closeReport(db, admin, first!.id, 'upheld', '', null)).toEqual({ error: 'not-found' });
      const [closed] = await db
        .select()
        .from(reports)
        .where(sql`id = ${first!.id}`);
      expect(closed).toMatchObject({ state: 'dismissed', answer: 'La licence est bien MIT.' });
      expect((await reviewQueue(db, archives, keys)).counts.reports).toBe(2);
    });

    it('holds the names too close until a person lets them pass or refuses them', async () => {
      const held = await version('dev.q7x.slak-notify', '0.0.1', { state: 'held' });
      await db
        .update(versions)
        .set({ checks: [{ key: 'name', status: 'hold', detail: { fr: 'proche de Slack', en: 'close to Slack' } }] })
        .where(sql`id = ${held.versionId}`);
      const other = await version('dev.q7x.slak-notify2', '0.0.1', { state: 'held' });
      const q = await reviewQueue(db, archives, keys);
      expect(q.held.map((h) => h.id)).toEqual(['dev.q7x.slak-notify', 'dev.q7x.slak-notify2']);
      expect(q.held[0]!.why).toEqual({ fr: 'proche de Slack', en: 'close to Slack' });
      expect(await settleHeld(db, admin, held.versionId, 'release', '', null)).toEqual({ ok: true });
      expect(await settleHeld(db, admin, other.versionId, 'refuse', '', null)).toEqual({ error: 'message' });
      expect(await settleHeld(db, admin, other.versionId, 'refuse', 'Se fait passer pour Slack.', null)).toEqual({ ok: true });
      const states = await db.select({ id: versions.pluginId, state: versions.state }).from(versions).orderBy(versions.id);
      expect(states).toEqual([
        { id: 'dev.q7x.slak-notify', state: 'ready' },
        { id: 'dev.q7x.slak-notify2', state: 'rejected' },
      ]);
      expect((await reviewQueue(db, archives, keys)).held).toEqual([]);
    });
  });
});
