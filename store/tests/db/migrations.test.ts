import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import pg from 'pg';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import { lastBeat, noteBeat, startQueue } from '../../server/utils/queue';

// Against a real PostgreSQL (the CI's service, PostgreSQL 17): the migrations played from an empty base, then the
// queue. The base is emptied first: it is named by TEST_DATABASE_URL alone, never by DATABASE_URL, which Coolify also
// hands to the image's build (where these tests run) and which names the store's own base. Without it, skipped.
const url = process.env.TEST_DATABASE_URL;

describe.skipIf(!url)('the database', () => {
  let pool: pg.Pool;
  let db: Db;

  beforeAll(async () => {
    const admin = new pg.Client({ connectionString: url });
    await admin.connect();
    await admin.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await admin.end();
    ({ pool, db } = openDb(url!));
  });
  afterAll(async () => {
    await pool?.end();
  });

  it('plays the migrations on an empty base, and again without harm', async () => {
    await migrateDb(db, './server/db/migrations');
    await migrateDb(db, './server/db/migrations');
    const { rows } = await pool.query("select table_name from information_schema.tables where table_schema = 'public' order by 1");
    expect(rows.map((r) => r.table_name)).toEqual([
      'account_installations',
      'accounts',
      'admin_tokens',
      'audit',
      'discord_quota',
      'email_tokens',
      'installation_repos',
      'installations',
      'installs_daily',
      'plugins',
      'reports',
      'revocation_lists',
      'service_state',
      'sessions',
      'signatures',
      'versions',
      'webhook_deliveries',
    ]);
  });

  it('gives the plugins and versions already there the root as their folder (ESC-313, migration 0009)', async () => {
    // The base as it was before 0009, a plugin and its version in it, then 0009 played.
    const before = await mkdtemp(join(tmpdir(), 'store-migrations-'));
    try {
      await cp('./server/db/migrations', before, { recursive: true });
      const journal = JSON.parse(await readFile(join(before, 'meta/_journal.json'), 'utf8'));
      journal.entries = journal.entries.filter((e: { idx: number }) => e.idx < 9);
      await writeFile(join(before, 'meta/_journal.json'), JSON.stringify(journal));
      const admin = new pg.Client({ connectionString: url });
      await admin.connect();
      await admin.query(
        'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
      );
      await admin.end();
      await migrateDb(db, before);
      await pool.query(`insert into plugins (id, repo_id, repo_full_name) values ('io.github.sam.old', 1, 'sam/old')`);
      await pool.query(
        `insert into versions (plugin_id, installation_id, repo_id, repo_full_name, tag, commit) values ('io.github.sam.old', 1, 1, 'sam/old', 'v1.0.0', 'c')`,
      );
      await migrateDb(db, './server/db/migrations');
      expect((await pool.query('select path, source_from from plugins')).rows).toEqual([{ path: '', source_from: null }]);
      expect((await pool.query('select path from versions')).rows).toEqual([{ path: '' }]);
    } finally {
      await rm(before, { recursive: true, force: true });
    }
  });

  it('keeps the last beat of the queue', async () => {
    expect(await lastBeat(db)).toBeNull();
    const at = new Date('2026-10-29T10:00:00Z');
    await noteBeat(db, at);
    await noteBeat(db, new Date(at.getTime() + 60_000));
    expect((await lastBeat(db))?.toISOString()).toBe('2026-10-29T10:01:00.000Z');
  });

  it('starts the queue, whose first beat lands at once', async () => {
    const before = Date.now();
    const boss = await startQueue(url!, db, null);
    try {
      await expect.poll(async () => (await lastBeat(db))?.getTime() ?? 0, { timeout: 15_000 }).toBeGreaterThanOrEqual(before - 1000);
    } finally {
      await boss.stop({ graceful: false, timeout: 1000 });
    }
  });
});
