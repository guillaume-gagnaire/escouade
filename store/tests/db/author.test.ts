import { mkdir, mkdtemp, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { eq } from 'drizzle-orm';
import pg from 'pg';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { accountInstallations, accounts, installationRepos, installations, installsDaily, plugins, versions } from '../../server/db/schema';
import { applyWebhook, type Account } from '../../server/utils/accounts';
import {
  answerTransfer,
  askTransfer,
  authorPlugins,
  cancelTransfer,
  claimPlugin,
  deletePlugin,
  dismissVersion,
  FREEZE_DAYS,
  pendingVersions,
  pluginDetail,
  pluginsInTheWay,
  purgeDeleted,
  repoClaims,
  restorePlugin,
  setAutoPublish,
  setListed,
  transferOffers,
  yankVersion,
} from '../../server/utils/author';
import { buildCatalog, forgetCatalog } from '../../server/utils/catalog';
import { authorPage, catalog, pluginPage } from '../../server/utils/listing';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import { publishVersion } from '../../server/utils/submissions';
import { sha256, versionFile } from '../../server/utils/versions';

// The author's space against a real PostgreSQL (TEST_DATABASE_URL alone, emptied first; skipped without it): the
// account's plugins and their states, the versions waiting for a click, taking a version back, off the list, the
// transfer and its freeze (offered, or the repository moved on GitHub), the deletion, its undoing and its purge 30 days
// later, the installs by week and by version (docs/STORE.md §2.2).
const url = process.env.TEST_DATABASE_URL;
const T0 = new Date('2026-10-29T10:00:00Z');
const DAY = 86_400_000;
const later = (days: number) => new Date(T0.getTime() + days * DAY);

describe.skipIf(!url)('the author’s space', () => {
  let pool: pg.Pool;
  let db: Db;
  let lea: Account;
  let sam: Account;
  let dir: string;

  const account = async (githubId: number, login: string) => (await db.insert(accounts).values({ githubId, login }).returning())[0]!;

  /** A version of `id`, frozen as STORE-3 leaves it, in `state`; its archive kept in `dir`. */
  const version = async (
    id: string,
    number: string,
    state = 'published',
    over: { owner?: Account; permissions?: Record<string, unknown>; commit?: string; archive?: string; checks?: unknown[] } = {},
  ) => {
    const manifest = {
      id,
      name: { en: `Name ${id}`, fr: `Nom ${id}` },
      version: number,
      license: 'MIT',
      escouade: { api: '1', min: '2.1.0' },
      permissions: over.permissions ?? { network: ['ntfy.sh'] },
    };
    const archive = over.archive ?? sha256(`${id}${number}`);
    const commit = over.commit ?? sha256(`commit ${id}${number}`).slice(0, 40);
    const file = versionFile({
      id,
      version: number,
      repository: 'https://github.com/lea/escouade-slack',
      tag: `v${number}`,
      commit,
      archive: { sha256: archive, size: 10 },
      binaries: {},
      manifest_sha256: sha256(JSON.stringify(manifest)),
      escouade: manifest.escouade,
    });
    await mkdir(join(dir, 'sha256', archive.slice(0, 2)), { recursive: true });
    await writeFile(join(dir, 'sha256', archive.slice(0, 2), `${archive}.tar.gz`), 'archive');
    const [v] = await db
      .insert(versions)
      .values({
        pluginId: id,
        accountId: (over.owner ?? lea).id,
        origin: state === 'ready' ? 'webhook' : 'author',
        installationId: 10,
        repoId: 77,
        repoFullName: 'lea/escouade-slack',
        tag: `v${number}`,
        commit,
        version: number,
        state,
        checks: over.checks ?? [],
        manifest,
        archiveSha256: archive,
        archiveSize: 10,
        binaries: {},
        versionJson: state === 'published' || state === 'ready' ? file.json : null,
        versionSha256: state === 'published' || state === 'ready' ? file.sha256 : null,
        createdAt: T0,
        publishedAt: state === 'published' ? T0 : null,
      })
      .returning();
    return v!;
  };
  const plugin = (id: string, owner = lea, repoId = 77) =>
    db.insert(plugins).values({ id, accountId: owner.id, repoId, repoFullName: 'lea/escouade-slack', categories: ['notifications'] });
  const count = (id: string, v: string, day: string, n: number, event = 'install') =>
    db.insert(installsDaily).values({ pluginId: id, version: v, day, event, count: n });

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
    await pool.query('truncate accounts, plugins, versions, installs_daily, installations, audit cascade');
    lea = await account(1, 'lea-martin');
    sam = await account(2, 'sroux');
    dir = await mkdtemp(join(tmpdir(), 'store-author-'));
    forgetCatalog();
  });
  afterAll(async () => {
    await pool?.end();
  });
  const journal = async () => (await pool.query('select action from audit order by id')).rows.map((r) => r.action);

  it('lists the account’s plugins with their state, latest version and installs, and the first versions refused', async () => {
    await plugin('dev.lea.slack');
    await version('dev.lea.slack', '1.1.0');
    await version('dev.lea.slack', '1.2.0');
    await plugin('dev.lea.teams');
    await db.update(plugins).set({ unlistedAt: T0 }).where(eq(plugins.id, 'dev.lea.teams'));
    await version('dev.lea.teams', '0.4.0');
    const failed = [
      {
        key: 'license',
        status: 'fail',
        detail: { fr: 'MIT contre GPL-3.0', en: 'MIT against GPL-3.0' },
        problems: [{ path: 'license', why: { fr: 'pas d’accord', en: 'no match' }, fix: { fr: 'accorde-les', en: 'match them' } }],
      },
    ];
    await version('dev.lea.mattermost', '0.1.0', 'rejected', { checks: failed });
    // Another's plugin and another's first version are not shown.
    await plugin('dev.sam.ntfy', sam, 78);
    await version('dev.sam.ntfy', '1.0.0', 'published', { owner: sam });
    await count('dev.lea.slack', '1.2.0', '2026-10-28', 5);
    await count('dev.lea.slack', '1.1.0', '2026-08-01', 7);
    await count('dev.lea.slack', '1.2.0', '2026-10-27', 3, 'update');

    const rows = await authorPlugins(db, lea, T0);
    expect(rows.map((r) => [r.id, r.state, r.version, r.installs, r.recent])).toEqual([
      ['dev.lea.slack', 'review', '1.2.0', 12, 5],
      ['dev.lea.teams', 'unlisted', '0.4.0', 0, 0],
      ['dev.lea.mattermost', 'rejected', '0.1.0', 0, 0],
    ]);
    const refused = rows[2]!;
    expect(refused.rejection).toMatchObject({ check: 'license', path: 'license', fix: { en: 'match them' } });
    expect(refused.submission).toBeTypeOf('number');
    expect(rows[0]!.name).toBe('Name dev.lea.slack');
  });

  it('shows a version found on a new tag with what it asks more, publishes it from the account or sets it aside', async () => {
    await plugin('dev.lea.slack');
    const old = await version('dev.lea.slack', '1.2.0');
    const next = await version('dev.lea.slack', '1.3.0', 'ready', { permissions: { network: ['ntfy.sh'], events: ['agent.created'] } });
    const [p] = await pendingVersions(db, lea, T0);
    expect(p).toMatchObject({
      id: next.id,
      plugin: 'dev.lea.slack',
      version: '1.3.0',
      previousCommit: old.commit,
      added: ['events:agent.created'],
    });
    expect(await pendingVersions(db, sam, T0)).toEqual([]);

    expect(await publishVersion(db, sam, next.id, {}, T0)).toEqual({ error: 'not-found' });
    expect(await publishVersion(db, lea, next.id, {}, T0)).toEqual({ ok: true });
    expect((await buildCatalog(db, T0)).plugins[0]!.latest).toBe('1.3.0');

    const again = await version('dev.lea.slack', '1.4.0', 'ready');
    expect(await dismissVersion(db, sam, again.id)).toEqual({ error: 'not-found' });
    expect(await dismissVersion(db, lea, again.id)).toEqual({ ok: true });
    expect(await pendingVersions(db, lea, T0)).toEqual([]);
    expect(await dismissVersion(db, lea, again.id)).toEqual({ error: 'not-ready' });
    expect(await journal()).toEqual(['version.published', 'version.dismissed']);
  });

  it('takes a version back: still in the catalogue, marked, no longer the latest; all taken back, no latest', async () => {
    await plugin('dev.lea.slack');
    const one = await version('dev.lea.slack', '1.0.0');
    const two = await version('dev.lea.slack', '2.0.0');
    expect(await yankVersion(db, sam, two.id, null, T0)).toEqual({ error: 'not-found' });
    expect(await yankVersion(db, lea, two.id, null, T0)).toEqual({ ok: true });
    expect(await yankVersion(db, lea, two.id, null, T0)).toEqual({ error: 'already' });
    let c = (await buildCatalog(db, T0)).plugins[0]!;
    expect(c.latest).toBe('1.0.0');
    expect(c.versions.map((v) => [v.version, v.yanked])).toEqual([
      ['2.0.0', true],
      ['1.0.0', false],
    ]);
    await yankVersion(db, lea, one.id, null, T0);
    forgetCatalog();
    c = (await buildCatalog(db, T0)).plugins[0]!;
    expect(c.latest).toBeNull();
    expect((await authorPlugins(db, lea, T0))[0]!.state).toBe('yanked');
    // A version waiting is not taken back: it is set aside.
    const ready = await version('dev.lea.slack', '3.0.0', 'ready');
    expect(await yankVersion(db, lea, ready.id, null, T0)).toEqual({ error: 'not-published' });
  });

  it('keeps the public pages in step: off the list or deleted, out of the lists but still readable; versions taken back marked', async () => {
    await plugin('dev.lea.slack');
    const one = await version('dev.lea.slack', '1.0.0');
    const two = await version('dev.lea.slack', '2.0.0');
    const listed = async () => (await catalog(db)).filter((e) => !e.builtin).map((e) => [e.id, e.version]);
    expect(await listed()).toEqual([['dev.lea.slack', '2.0.0']]);

    await yankVersion(db, lea, two.id, null, T0);
    expect(await listed()).toEqual([['dev.lea.slack', '1.0.0']]);
    let page = (await pluginPage(db, dir, 'dev.lea.slack'))!;
    expect(page.entry.version).toBe('1.0.0');
    expect(page.versions.map((v) => [v.version, v.yanked])).toEqual([
      ['2.0.0', true],
      ['1.0.0', false],
    ]);
    // All taken back: out of the lists, its page still readable.
    await yankVersion(db, lea, one.id, null, T0);
    expect(await listed()).toEqual([]);
    expect((await pluginPage(db, dir, 'dev.lea.slack'))!.entry.version).toBe('2.0.0');

    await db.update(versions).set({ yankedAt: null }).where(eq(versions.pluginId, 'dev.lea.slack'));
    await setListed(db, lea, 'dev.lea.slack', false, null, T0);
    expect(await listed()).toEqual([]);
    expect((await authorPage(db, 'lea-martin'))!.plugins).toEqual([]);
    page = (await pluginPage(db, dir, 'dev.lea.slack'))!;
    expect(page.status).toMatchObject({ unlisted: true, deletedAt: null });

    await setListed(db, lea, 'dev.lea.slack', true, null, T0);
    await deletePlugin(db, lea, 'dev.lea.slack', 'dev.lea.slack', null, T0);
    expect(await listed()).toEqual([]);
    expect((await pluginPage(db, dir, 'dev.lea.slack'))!.status).toMatchObject({ unlisted: false, deletedAt: T0.toISOString() });
    await restorePlugin(db, lea, 'dev.lea.slack', null, T0);

    await askTransfer(db, lea, 'dev.lea.slack', 'sroux', null, T0);
    await answerTransfer(db, sam, 'dev.lea.slack', true, null, T0);
    expect((await pluginPage(db, dir, 'dev.lea.slack'))!.status).toMatchObject({
      transferredFrom: 'lea-martin',
      transferredAt: T0.toISOString(),
    });
    expect((await authorPage(db, 'sroux'))!.plugins.map((e) => e.id)).toEqual(['dev.lea.slack']);
  });

  it('takes a plugin off the list and puts it back; lets new tags be published alone', async () => {
    await plugin('dev.lea.slack');
    await version('dev.lea.slack', '1.0.0');
    expect(await setListed(db, sam, 'dev.lea.slack', false, null, T0)).toEqual({ error: 'not-found' });
    expect(await setListed(db, lea, 'dev.lea.slack', false, null, T0)).toEqual({ ok: true });
    expect((await buildCatalog(db, T0)).plugins).toEqual([]);
    expect(await setListed(db, lea, 'dev.lea.slack', true, null, T0)).toEqual({ ok: true });
    expect((await buildCatalog(db, T0)).plugins).toHaveLength(1);
    await setAutoPublish(db, lea, 'dev.lea.slack', true, T0);
    expect((await authorPlugins(db, lea, T0))[0]!.autoPublish).toBe(true);
    expect(await journal()).toEqual(['plugin.unlisted', 'plugin.relisted', 'plugin.auto_publish_on']);
  });

  it('transfers a plugin to an account that accepts it, and freezes its publications 7 days', async () => {
    await plugin('dev.lea.slack');
    await version('dev.lea.slack', '1.0.0');
    const waiting = await version('dev.lea.slack', '1.1.0', 'ready');
    expect(await askTransfer(db, lea, 'dev.lea.slack', 'nobody', null, T0)).toEqual({ error: 'no-account' });
    expect(await askTransfer(db, lea, 'dev.lea.slack', 'Lea-Martin', null, T0)).toEqual({ error: 'self' });
    expect(await askTransfer(db, sam, 'dev.lea.slack', 'lea-martin', null, T0)).toEqual({ error: 'not-found' });
    expect(await askTransfer(db, lea, 'dev.lea.slack', '@SRoux', null, T0)).toEqual({ ok: true });
    expect((await authorPlugins(db, lea, T0))[0]!.transferTo).toBe('sroux');
    expect(await transferOffers(db, sam)).toMatchObject([{ id: 'dev.lea.slack', from: 'lea-martin' }]);
    // Offered, it is still Léa's: Sam can do nothing with it yet.
    expect(await setListed(db, sam, 'dev.lea.slack', false, null, T0)).toEqual({ error: 'not-found' });
    expect(await answerTransfer(db, lea, 'dev.lea.slack', true, null, T0)).toEqual({ error: 'not-found' });

    expect(await answerTransfer(db, sam, 'dev.lea.slack', true, null, T0)).toEqual({ ok: true });
    expect(await authorPlugins(db, lea, T0)).toEqual([]);
    const [row] = await authorPlugins(db, sam, T0);
    expect(row).toMatchObject({ id: 'dev.lea.slack', transferredFrom: 'lea-martin', transferTo: null });
    expect(row!.frozenUntil).toEqual(later(FREEZE_DAYS));
    expect((await buildCatalog(db, T0)).plugins[0]).toMatchObject({ author: 'sroux', transferred_at: T0.toISOString() });
    // The freeze: no publication for 7 days, then the click works.
    expect(await publishVersion(db, sam, waiting.id, {}, later(6))).toEqual({ error: 'frozen' });
    expect(await publishVersion(db, lea, waiting.id, {}, later(8))).toEqual({ error: 'not-found' });
    expect(await publishVersion(db, sam, waiting.id, {}, later(8))).toEqual({ ok: true });
    expect(await journal()).toContain('plugin.transferred');
  });

  it('refuses or cancels an offer: the plugin stays', async () => {
    await plugin('dev.lea.slack');
    await askTransfer(db, lea, 'dev.lea.slack', 'sroux', null, T0);
    expect(await answerTransfer(db, sam, 'dev.lea.slack', false, null, T0)).toEqual({ ok: true });
    expect(await transferOffers(db, sam)).toEqual([]);
    await askTransfer(db, lea, 'dev.lea.slack', 'sroux', null, T0);
    expect(await cancelTransfer(db, lea, 'dev.lea.slack', T0)).toEqual({ ok: true });
    expect(await answerTransfer(db, sam, 'dev.lea.slack', true, null, T0)).toEqual({ error: 'not-found' });
    expect((await authorPlugins(db, lea, T0))[0]!.frozenUntil).toBeNull();
  });

  it('follows a repository moved to another owner on GitHub: the new owner takes the plugin over, frozen 7 days', async () => {
    await plugin('dev.lea.slack');
    await version('dev.lea.slack', '1.0.0');
    await applyWebhook(
      db,
      'repository',
      'd-1',
      { action: 'transferred', repository: { id: 77, full_name: 'sroux/escouade-slack', private: false } },
      T0,
    );
    // Not yet: the new owner has not installed the App on it.
    expect(await repoClaims(db, sam)).toEqual([]);
    expect(await claimPlugin(db, sam, 'dev.lea.slack', null, T0)).toEqual({ error: 'not-found' });
    await db.insert(installations).values({ id: 20, ownerId: 2, ownerLogin: 'sroux', ownerType: 'User' });
    await db.insert(installationRepos).values({ installationId: 20, repoId: 77, fullName: 'sroux/escouade-slack' });
    await db.insert(accountInstallations).values({ accountId: sam.id, installationId: 20 });
    expect(await repoClaims(db, lea)).toEqual([]);
    expect(await repoClaims(db, sam)).toMatchObject([{ id: 'dev.lea.slack', repository: 'sroux/escouade-slack' }]);
    expect(await claimPlugin(db, sam, 'dev.lea.slack', null, T0)).toEqual({ ok: true });
    const [row] = await authorPlugins(db, sam, T0);
    expect(row).toMatchObject({ repository: 'sroux/escouade-slack', transferredFrom: 'lea-martin', repoMovedAt: null });
    expect(row!.frozenUntil).toEqual(later(FREEZE_DAYS));
    expect(await repoClaims(db, sam)).toEqual([]);
  });

  it('deletes a plugin once its id is typed, undoes it within 30 days, then purges it and keeps its id and numbers', async () => {
    await plugin('dev.lea.slack');
    const one = await version('dev.lea.slack', '1.0.0');
    // A second plugin of the same commit shares the archive: it stays.
    await plugin('dev.lea.other', lea, 79);
    await version('dev.lea.other', '1.0.0', 'published', { archive: one.archiveSha256! });
    await version('dev.lea.slack', '1.1.0');
    const waiting = await version('dev.lea.slack', '1.2.0', 'ready');

    expect(await pluginsInTheWay(db, lea.id)).toEqual(['dev.lea.other', 'dev.lea.slack']);
    expect(await deletePlugin(db, lea, 'dev.lea.slack', 'dev.lea', null, T0)).toEqual({ error: 'unconfirmed' });
    expect(await deletePlugin(db, sam, 'dev.lea.slack', 'dev.lea.slack', null, T0)).toEqual({ error: 'not-found' });
    expect(await deletePlugin(db, lea, 'dev.lea.slack', ' dev.lea.slack ', null, T0)).toEqual({ ok: true });
    expect((await buildCatalog(db, T0)).plugins.map((p) => p.id)).toEqual(['dev.lea.other']);
    expect((await authorPlugins(db, lea, T0)).find((r) => r.id === 'dev.lea.slack')).toMatchObject({
      state: 'deleted',
      purgeOn: later(30),
    });
    expect(await pluginsInTheWay(db, lea.id)).toEqual(['dev.lea.other']);
    expect(await publishVersion(db, lea, waiting.id, {}, T0)).toEqual({ error: 'deleted' });

    expect(await restorePlugin(db, lea, 'dev.lea.slack', null, T0)).toEqual({ ok: true });
    forgetCatalog();
    expect((await buildCatalog(db, T0)).plugins).toHaveLength(2);
    await deletePlugin(db, lea, 'dev.lea.slack', 'dev.lea.slack', null, T0);

    expect(await purgeDeleted(db, dir, later(29))).toEqual({ plugins: 0, files: 0 });
    expect(await purgeDeleted(db, dir, later(31))).toEqual({ plugins: 1, files: 2 });
    const left = (await readdir(join(dir, 'sha256'), { recursive: true })).filter((f) => f.endsWith('.tar.gz'));
    expect(left).toEqual([join(one.archiveSha256!.slice(0, 2), `${one.archiveSha256}.tar.gz`)]);
    const [kept] = await db.select().from(plugins).where(eq(plugins.id, 'dev.lea.slack'));
    expect(kept).toMatchObject({ accountId: lea.id, retiredVersions: ['1.0.0', '1.1.0', '1.2.0'] });
    expect(kept!.purgedAt).toEqual(later(31));
    expect(await db.select().from(versions).where(eq(versions.pluginId, 'dev.lea.slack'))).toEqual([]);
    expect(await authorPlugins(db, lea, later(31))).toHaveLength(1);
    expect(await restorePlugin(db, lea, 'dev.lea.slack', null, later(31))).toEqual({ error: 'not-found' });
    await rm(dir, { recursive: true, force: true });
  });

  it('counts the installs of a plugin by week and by version, updates apart', async () => {
    await plugin('dev.lea.slack');
    await version('dev.lea.slack', '1.0.0');
    await version('dev.lea.slack', '1.1.0');
    await count('dev.lea.slack', '1.0.0', '2026-08-01', 50);
    await count('dev.lea.slack', '1.0.0', '2026-10-29', 2);
    await count('dev.lea.slack', '1.1.0', '2026-10-23', 4);
    await count('dev.lea.slack', '1.1.0', '2026-10-22', 3);
    await count('dev.lea.slack', '1.1.0', '2026-10-28', 6, 'update');
    expect(await pluginDetail(db, sam, 'dev.lea.slack', T0)).toBeNull();
    const d = (await pluginDetail(db, lea, 'dev.lea.slack', T0))!;
    expect(d).toMatchObject({ installs: 59, updates: 6, recent: 9 });
    expect(d.weeks).toHaveLength(12);
    expect(d.weeks.at(-1)).toEqual({ installs: 6, updates: 6 });
    expect(d.weeks.at(-2)).toEqual({ installs: 3, updates: 0 });
    expect(d.weeks.slice(0, -2).every((w) => w.installs === 0)).toBe(true);
    expect(d.versions.map((v) => [v.version, v.installs, v.updates])).toEqual([
      ['1.1.0', 7, 6],
      ['1.0.0', 52, 0],
    ]);
  });
});
