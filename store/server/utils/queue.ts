import { eq } from 'drizzle-orm';
import { PgBoss } from 'pg-boss';
import { serviceState } from '../db/schema';
import { purge } from './accounts';
import type { Db } from './db';
import { checkVersion, onTag, rereadTags, type Deps } from './submissions';

// The long tasks (reading a repository, fetching an archive, checking it) go through a queue kept in PostgreSQL
// (pg-boss): no Redis to host. One process serves the pages and works the queue; a worker of its own, later, is the
// same image with another command. The queue's own beat, every minute, tells /api/health that it moves.

export const BEAT = 'store.beat';
/** Once a night, what has outlived its time goes: sessions, tokens, links, deliveries, journal (docs/STORE.md, §2.3). */
export const PURGE = 'store.purge';
/** The checks of a version submitted (`{ id }`), one at a time per worker: each holds an archive in memory. */
export const CHECK = 'store.check';
/** A tag the App's webhooks saw (`{ repoId, fullName, installationId, tag }`). */
export const TAG = 'store.tag';
/** Once a week, the tags of the published versions read again: a tag moved without a webhook is caught. */
export const REREAD = 'store.reread';
/** The key of `service_state` where the beat is noted. */
export const BEAT_KEY = 'queue.beat';

/** Notes that the queue just did a job. */
export async function noteBeat(db: Db, at = new Date()): Promise<void> {
  await db.insert(serviceState).values({ key: BEAT_KEY, at }).onConflictDoUpdate({ target: serviceState.key, set: { at } });
}

/** When the queue last did a job, or null if it never has. */
export async function lastBeat(db: Db): Promise<Date | null> {
  const rows = await db.select({ at: serviceState.at }).from(serviceState).where(eq(serviceState.key, BEAT_KEY));
  return rows[0]?.at ?? null;
}

/**
 * Starts the queue on `url`, works the beat and sends the first one at once, and plans the nightly purge. With
 * `deps` (the App is set up), it also works the submissions: the checks, the tags of the webhooks, the weekly read.
 */
export async function startQueue(url: string, db: Db, deps: Deps | null): Promise<PgBoss> {
  const boss = new PgBoss({ connectionString: url, schema: 'pgboss' });
  boss.on('error', (e) => console.error('[store] queue:', e instanceof Error ? e.message : e));
  await boss.start();
  await boss.createQueue(BEAT);
  await boss.work(BEAT, async () => {
    await noteBeat(db);
  });
  await boss.schedule(BEAT, '* * * * *');
  await boss.send(BEAT);
  await boss.createQueue(PURGE);
  await boss.work(PURGE, async () => {
    console.info('[store] purged:', JSON.stringify(await purge(db)));
  });
  await boss.schedule(PURGE, '40 4 * * *');
  if (!deps) return boss;

  await boss.createQueue(CHECK);
  await boss.work<{ id: number }>(CHECK, { batchSize: 1 }, async ([job]) => {
    if (job) await checkVersion(db, deps, job.data.id);
  });
  await boss.createQueue(TAG);
  await boss.work<{ repoId: number; fullName: string; installationId: number | null; tag: string }>(TAG, async ([job]) => {
    if (!job) return;
    const { check } = await onTag(db, deps, job.data);
    if (check) await boss.send(CHECK, { id: check });
  });
  await boss.createQueue(REREAD);
  await boss.work(REREAD, async () => {
    console.info('[store] tags read again:', JSON.stringify(await rereadTags(db, deps)));
  });
  await boss.schedule(REREAD, '20 5 * * 1');
  return boss;
}
