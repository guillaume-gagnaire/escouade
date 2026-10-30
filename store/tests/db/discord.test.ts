import { eq } from 'drizzle-orm';
import pg from 'pg';
import type { PgBoss } from 'pg-boss';
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { accounts, discordQuota, plugins, versions } from '../../server/db/schema';
import { forgetCatalog } from '../../server/utils/catalog';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import { announce, deliver, deliverSummary, PER_AUTHOR_DAY, setOutbox } from '../../server/utils/discord';
import { readPublicKey } from '../../server/utils/minisign';
import { DISCORD, DISCORD_SUMMARY, startQueue } from '../../server/utils/queue';
import { fileReport } from '../../server/utils/reports';
import { acceptSignature, depositRevocations } from '../../server/utils/review';
import { REVOCATIONS_COMMENT } from '../../server/utils/revocation-list';
import { verifiedComment } from '../../server/utils/signature';
import { onTag, publishVersion, type Deps } from '../../server/utils/submissions';
import { versionFile } from '../../server/utils/versions';
import { fakeDiscord, type FakeDiscord } from '../fake/discord.mjs';
import { testKey } from '../fake/minisign.mjs';

/** The e-mails the store sends, kept instead of sent. */
const mails = vi.hoisted(() => [] as { to: string; subject: string }[]);
vi.mock('../../server/utils/mail', async (actual) => ({
  ...(await actual<typeof import('../../server/utils/mail')>()),
  sendMail: async (_env: unknown, mail: { to: string; subject: string }) => (mails.push(mail), true),
}));

// The announcements on Discord through the real queue (docs/STORE.md §4.7, STORE-12), against a real PostgreSQL
// (TEST_DATABASE_URL alone, emptied first; skipped without it) and a fake Discord: each event from where it happens
// (a publication, a report, a tag moved, a signature, a revocation list) to the right channel in the right language,
// the author's click that never waits for Discord, a 429 waited for, the limit per author and per day and its summary,
// a channel without its webhook that gets nothing, and the webhooks' addresses in no log and no job of the queue.
const url = process.env.TEST_DATABASE_URL;
const SITE = 'https://store.escouade.dev';
const T0 = new Date('2026-10-29T10:00:00Z');

/** Waits until `ok()` holds, 20 s at most (the queue's workers poll every 2 s by default). */
async function until(ok: () => boolean, ms = 20_000) {
  const end = Date.now() + ms;
  while (!ok()) {
    if (Date.now() > end) throw new Error('timed out');
    await new Promise((r) => setTimeout(r, 100));
  }
}

describe.skipIf(!url)('the announcements on Discord', () => {
  let pool: pg.Pool;
  let db: Db;
  let boss: PgBoss;
  let discord: FakeDiscord;
  let env: Record<string, string>;
  let author: typeof accounts.$inferSelect;
  let admin: typeof accounts.$inferSelect;
  const logs: string[] = [];
  const key = testKey();
  const keys = [readPublicKey(key.pub)!];

  /** A version of `id` by the author, as STORE-3 leaves it after its checks (`ready`) or published. */
  const version = async (id: string, number: string, o: { state?: string; network?: string[]; name?: string } = {}) => {
    const manifest = {
      id,
      name: { fr: o.name ?? `Nom ${id}`, en: o.name ?? `Name ${id}` },
      description: { fr: 'Envoie une notification.', en: 'Sends a notification.' },
      version: number,
      permissions: { network: o.network ?? ['ntfy.sh'] },
    };
    const file = versionFile({
      id,
      version: number,
      repository: 'https://github.com/sroux/escouade-ntfy',
      tag: `v${number}`,
      commit: 'c'.repeat(40),
      archive: { sha256: 'a'.repeat(64), size: 10 },
      binaries: {},
      manifest_sha256: 'b'.repeat(64),
      escouade: { api: '1', min: '2.1.0' },
    });
    const state = o.state ?? 'published';
    const [row] = await db
      .insert(versions)
      .values({
        pluginId: id,
        accountId: author.id,
        installationId: 1,
        repoId: 77,
        repoFullName: 'sroux/escouade-ntfy',
        tag: `v${number}`,
        commit: 'c'.repeat(40),
        version: number,
        state,
        manifest,
        binaries: {},
        versionJson: file.json,
        versionSha256: file.sha256,
        publishedAt: state === 'published' ? T0 : null,
      })
      .returning();
    return row!;
  };
  const plugin = (id: string, accountId = author.id) =>
    db
      .insert(plugins)
      .values({ id, accountId, repoId: 77, repoFullName: 'sroux/escouade-ntfy', installationId: 1, categories: ['notifications'] });
  const titles = (channel: string) => discord.of(channel).map((b) => b.embeds[0].title);

  beforeAll(async () => {
    const c = new pg.Client({ connectionString: url });
    await c.connect();
    await c.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await c.end();
    ({ pool, db } = openDb(url!));
    await migrateDb(db, './server/db/migrations');
    discord = await fakeDiscord();
    env = {
      STORE_DISCORD_ADMIN_WEBHOOK: discord.hook('admin'),
      STORE_DISCORD_PUBLIC_WEBHOOK_FR: discord.hook('fr'),
      STORE_DISCORD_PUBLIC_WEBHOOK_EN: discord.hook('en'),
    };
    for (const level of ['error', 'warn', 'info', 'log'] as const)
      vi.spyOn(console, level).mockImplementation((...a: unknown[]) => void logs.push(a.map(String).join(' ')));
    boss = await startQueue(url!, db, null, { env, site: SITE });
  });
  beforeEach(async () => {
    await pool.query('truncate accounts, plugins, versions, reports, signatures, revocation_lists, discord_quota cascade');
    [author] = await db.insert(accounts).values({ githubId: 4242, login: 'sroux' }).returning();
    [admin] = await db.insert(accounts).values({ githubId: 101, login: 'gagnaire' }).returning();
    discord.reset();
    forgetCatalog();
  });
  afterAll(async () => {
    setOutbox(null);
    await boss?.stop({ graceful: false, timeout: 2_000 });
    // No address of a webhook in the log, nor in the queue's jobs (their data and their errors): the host alone.
    const { rows } = await pool.query(
      "select data::text || coalesce(output::text, '') as t from pgboss.job where name like 'store.discord%'",
    );
    expect(rows.length).toBeGreaterThan(0);
    for (const r of rows) expect(r.t).not.toMatch(/tok3n|api\/webhooks/);
    vi.restoreAllMocks();
    for (const l of logs) expect(l).not.toMatch(/tok3n|api\/webhooks/);
    await pool?.end();
    await discord?.close();
  });

  it('publishes a first version without waiting for Discord: the new plugin to both public channels, the review to the administrators', async () => {
    const v = await version('io.github.sroux.ntfy', '1.0.0', { state: 'ready' });
    // Discord answers in 3 s: the click does not wait for it.
    discord.plan([
      { status: 204, delay: 3000 },
      { status: 204, delay: 3000 },
      { status: 204, delay: 3000 },
    ]);
    const t0 = Date.now();
    const done = await publishVersion(db, author, v.id, { categories: ['notifications'], review: 'on' }, T0);
    expect(done).toEqual({ ok: true });
    expect(Date.now() - t0).toBeLessThan(1000);
    await until(() => discord.received.length === 3);
    expect(titles('fr')).toEqual(['Nouveau plugin']);
    expect(titles('en')).toEqual(['New plugin']);
    expect(titles('admin')).toEqual(['Version à revoir']);
    const [fr] = discord.of('fr');
    expect(fr.allowed_mentions).toEqual({ parse: [] });
    expect(fr.embeds[0].url).toBe(`${SITE}/plugins/io.github.sroux.ntfy`);
    expect(fr.embeds[0].fields[1].value).toBe('Non vérifié : pas encore relu par Escouade');
    expect(discord.of('en')[0].embeds[0].description).toContain('Name io\\.github\\.sroux\\.ntfy');
    const review = discord.of('admin')[0].embeds[0];
    expect(review.url).toBe(`${SITE}/review`);
    expect(review.fields.find((f: { name: string }) => f.name === 'Commande').value).toContain(
      'npm run plugin-review -- io.github.sroux.ntfy@1.0.0',
    );
  });

  it('publishes a submission even when Discord is down', async () => {
    const v = await version('io.github.sroux.down', '1.0.0', { state: 'ready' });
    discord.plan(Array.from({ length: 10 }, () => ({ status: 503 })));
    expect(await publishVersion(db, author, v.id, { categories: ['tools'], review: 'off' }, T0)).toEqual({ ok: true });
    const [row] = await db.select().from(versions).where(eq(versions.id, v.id));
    expect(row!.state).toBe('published');
    // Its two messages wait to be tried again, later (the queue's retries); taken off, not to land in the next tests.
    const waiting = async () =>
      (await pool.query("select count(*)::int as n from pgboss.job where name = $1 and state = 'retry'", [DISCORD])).rows[0].n;
    const end = Date.now() + 20_000;
    while ((await waiting()) < 2 && Date.now() < end) await new Promise((r) => setTimeout(r, 200));
    expect(await waiting()).toBe(2);
    await pool.query("delete from pgboss.job where name = $1 and state = 'retry'", [DISCORD]);
    discord.reset();
  });

  it('announces a new version to the public, with what grew since the last verified to the administrators', async () => {
    await plugin('io.github.sroux.ntfy');
    const first = await version('io.github.sroux.ntfy', '1.0.0');
    const json = first.versionJson!;
    expect(
      await acceptSignature(
        db,
        admin,
        keys,
        {
          id: 'io.github.sroux.ntfy',
          version: '1.0.0',
          minisig: key.sign(json, verifiedComment('io.github.sroux.ntfy', '1.0.0', '2026-10-29')),
        },
        null,
        T0,
      ),
    ).toMatchObject({ ok: true });
    await until(() => discord.received.length === 2);
    expect(titles('fr')).toEqual(['Vérifié par Escouade']);
    expect(titles('en')).toEqual(['Verified by Escouade']);
    discord.reset();
    const next = await version('io.github.sroux.ntfy', '1.1.0', { state: 'ready', network: ['ntfy.sh', 'evil.example'] });
    expect(await publishVersion(db, author, next.id, { review: 'on' }, T0)).toEqual({ ok: true });
    await until(() => discord.received.length === 3);
    expect(titles('fr')).toEqual(['Nouvelle version']);
    expect(titles('en')).toEqual(['New version']);
    const review = discord.of('admin')[0].embeds[0];
    const grown = review.fields.find((f: { name: string }) => f.name.startsWith('Depuis la dernière vérifiée (1.0.0)'));
    expect(grown.value).toBe('• Se connecte à \\: evil\\.example');
  });

  it('tells the administrators of a report, without its sender nor its words', async () => {
    await plugin('io.github.sroux.ntfy');
    await version('io.github.sroux.ntfy', '1.0.0');
    const filed = await fileReport(
      db,
      { reason: 'malicious', details: 'Il envoie mes clés à https://evil.example @everyone', email: 'temoin@example.org', faith: 'on' },
      { pluginId: 'io.github.sroux.ntfy', version: '1.0.0', lang: 'fr' },
    );
    expect('id' in filed).toBe(true);
    await until(() => discord.of('admin').length === 1);
    const s = JSON.stringify(discord.of('admin')[0]);
    expect(s).toContain('Signalement');
    expect(s).toContain('Malveillant');
    expect(s).not.toContain('temoin');
    expect(s).not.toContain('evil');
    expect(discord.of('fr')).toEqual([]);
  });

  it('tells the administrators of a tag moved after its version was published, and of a name too close', async () => {
    await plugin('io.github.sroux.ntfy');
    await version('io.github.sroux.ntfy', '1.0.0');
    const deps = {
      source: () => ({ resolve: async () => ({ commit: 'd'.repeat(40), date: null }) }),
      archives: '.',
      env: {},
      site: SITE,
    } as unknown as Deps;
    expect(await onTag(db, deps, { repoId: 77, fullName: 'sroux/escouade-ntfy', installationId: 1, tag: 'v1.0.0' }, T0)).toMatchObject({
      moved: expect.any(Number),
    });
    await until(() => discord.of('admin').length === 1);
    expect(discord.of('admin')[0].embeds[0].fields.find((f: { name: string }) => f.name === 'Commit').value).toBe('`ccccccc` → `ddddddd`');
    // A name too close: held, said by the checks (submissions.ts) as soon as they hold it.
    const [other] = await db.insert(accounts).values({ githubId: 77, login: 'other' }).returning();
    await plugin('io.github.other.ntfy', other!.id);
    const held = await version('io.github.sroux.ntffy', '1.0.0', { state: 'held', name: 'ntffy' });
    await announce({ kind: 'held', version: held.id });
    await until(() => discord.of('admin').length === 2);
    const close = discord.of('admin')[1].embeds[0];
    expect(close.title).toBe('Nom trop proche');
    expect(close.fields.find((f: { name: string }) => f.name === 'Proche de').value).toContain('io.github.other.ntfy');
  });

  /** A revocation list of `entries`, signed with the test key. */
  const list = (serial: number, entries: unknown[]) => {
    const json = JSON.stringify({ serial, issued_at: T0.toISOString(), entries });
    return { json, minisig: key.sign(json, `${REVOCATIONS_COMMENT} ${serial} ${T0.toISOString()}`) };
  };
  const entry = (action: 'disable' | 'unverify') => ({
    id: 'io.github.sroux.ntfy',
    versions: ['1.0.0'],
    action,
    reason: { fr: 'Envoie des secrets.', en: 'Sends secrets.' },
    date: '2026-10-29',
  });

  // A removal and a badge withdrawn go to the administrators and to the public, in each language with its public
  // reason, --quiet or not: --quiet only spares the author's e-mail (docs/STORE.md §4.6, §4.7, ESC-302).
  for (const [action, toAdmins, fr, en] of [
    ['disable', 'Retrait d’urgence fait', 'Plugin retiré', 'Plugin taken off'],
    ['unverify', 'Badge retiré', 'Badge retiré', 'Badge withdrawn'],
  ] as const)
    for (const quiet of [false, true])
      it(`tells ${action}${quiet ? ' --quiet' : ''} to the administrators and to the public, the author ${quiet ? 'not' : 'by e-mail'}`, async () => {
        await db.update(accounts).set({ email: 'sroux@example.com', emailVerifiedAt: T0 }).where(eq(accounts.id, author.id));
        await plugin('io.github.sroux.ntfy');
        await version('io.github.sroux.ntfy', '1.0.0');
        mails.length = 0;
        const how = { env, site: SITE };
        expect(await depositRevocations(db, admin, keys, { ...list(1, [entry(action)]), notify: !quiet }, how, T0)).toEqual({ serial: 1 });
        await until(() => discord.received.length === 3);
        expect(titles('admin')).toEqual([toAdmins]);
        const fields: { name: string; value: string }[] = discord.of('admin')[0].embeds[0].fields;
        expect(fields.find((f) => f.name === 'Raison publique')!.value).toBe('Envoie des secrets\\.');
        expect(fields.find((f) => f.name === 'Auteur')!.value).toBe(quiet ? 'non prévenu (--quiet)' : 'prévenu');
        expect(discord.of('fr')[0].embeds[0]).toMatchObject({ title: fr, fields: [{ name: 'Raison', value: 'Envoie des secrets\\.' }] });
        expect(discord.of('en')[0].embeds[0]).toMatchObject({ title: en, fields: [{ name: 'Reason', value: 'Sends secrets\\.' }] });
        expect(mails.map((m) => m.to)).toEqual(quiet ? [] : ['sroux@example.com']);
        // The next list keeps the entry: it is not told again, on Discord nor by e-mail.
        discord.reset();
        mails.length = 0;
        expect(await depositRevocations(db, admin, keys, list(2, [entry(action)]), how, T0)).toEqual({ serial: 2 });
        await new Promise((r) => setTimeout(r, 2500));
        expect(discord.received).toHaveLength(0);
        expect(mails).toEqual([]);
      });

  it('waits for the Retry-After of a 429, in the queue', async () => {
    await plugin('io.github.sroux.ntfy');
    const v = await version('io.github.sroux.ntfy', '1.0.0');
    discord.plan([{ status: 429, retryAfter: 1 }]);
    const t0 = Date.now();
    await announce({ kind: 'review', version: v.id });
    await until(() => discord.of('admin').length === 1);
    expect(Date.now() - t0).toBeGreaterThanOrEqual(1000);
  });

  it('holds the limit per author and per day: past it, one summary at the end of the day', async () => {
    await plugin('io.github.sroux.ntfy');
    const ids = [];
    for (let i = 0; i < PER_AUTHOR_DAY + 2; i++) ids.push((await version('io.github.sroux.ntfy', `1.0.${i}`)).id);
    for (const id of ids) await announce({ kind: 'new-version', version: id });
    // Every job worked: each channel counted all of them.
    const counted = async () => (await db.select().from(discordQuota)).reduce((n, r) => n + r.sent, 0);
    const end = Date.now() + 80_000;
    while ((await counted()) < 2 * (PER_AUTHOR_DAY + 2) && Date.now() < end) await new Promise((r) => setTimeout(r, 200));
    expect(discord.of('fr')).toHaveLength(PER_AUTHOR_DAY);
    const rows = await db.select().from(discordQuota);
    expect(rows.map((r) => [r.channel, r.sent, r.skipped.length]).sort()).toEqual([
      ['en', PER_AUTHOR_DAY + 2, 2],
      ['fr', PER_AUTHOR_DAY + 2, 2],
    ]);
    // One summary per channel, planned for the next midnight (UTC).
    const { rows: planned } = await pool.query('select data, start_after from pgboss.job where name = $1', [DISCORD_SUMMARY]);
    expect(planned.map((p) => p.data.channel).sort()).toEqual(['en', 'fr']);
    for (const p of planned) expect(new Date(p.start_after).getUTCHours()).toBe(0);
    discord.reset();
    expect(await deliverSummary(db, planned[0].data, { env, site: SITE })).toBe('sent');
    const sum = discord.received[0]!.body.embeds[0];
    expect(sum.url).toBe(`${SITE}${planned[0].data.channel === 'en' ? '/en' : ''}/authors/sroux`);
    expect(sum.description).toMatch(/2/);
  }, 90_000);

  it('sends nothing to a channel without its webhook', async () => {
    await plugin('io.github.sroux.ntfy');
    const v = await version('io.github.sroux.ntfy', '1.0.0');
    expect(
      await deliver(
        db,
        { channel: 'en', event: { kind: 'new-version', version: v.id } },
        { env: { ...env, STORE_DISCORD_PUBLIC_WEBHOOK_EN: '' }, site: SITE },
      ),
    ).toBe('off');
    // And the store makes no job for it.
    setOutbox({ send: (job) => boss.send(DISCORD, job), env: { STORE_DISCORD_ADMIN_WEBHOOK: env.STORE_DISCORD_ADMIN_WEBHOOK! } });
    await announce({ kind: 'new-plugin', version: v.id });
    await new Promise((r) => setTimeout(r, 3000));
    expect(discord.received).toEqual([]);
  });

  it('says nothing to the public of a plugin taken off the list by its author', async () => {
    await plugin('io.github.sroux.ntfy');
    await db.update(plugins).set({ unlistedAt: T0 });
    const v = await version('io.github.sroux.ntfy', '1.0.0');
    expect(await deliver(db, { channel: 'fr', event: { kind: 'new-version', version: v.id } }, { env, site: SITE })).toBe('gone');
    expect(discord.received).toEqual([]);
  });
});
