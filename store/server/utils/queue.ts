import { eq } from 'drizzle-orm';
import { PgBoss } from 'pg-boss';
import { serviceState } from '../db/schema';
import type { Db } from './db';

// The long tasks (reading a repository, fetching an archive, checking it) go through a queue kept in PostgreSQL
// (pg-boss): no Redis to host. One process serves the pages and works the queue; a worker of its own, later, is the
// same image with another command. The queue's own beat, every minute, tells /api/health that it moves.

export const BEAT = 'store.beat';
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

/** Starts the queue on `url`, works the beat and sends the first one at once. */
export async function startQueue(url: string, db: Db): Promise<PgBoss> {
  const boss = new PgBoss({ connectionString: url, schema: 'pgboss' });
  boss.on('error', (e) => console.error('[store] queue:', e instanceof Error ? e.message : e));
  await boss.start();
  await boss.createQueue(BEAT);
  await boss.work(BEAT, async () => {
    await noteBeat(db);
  });
  await boss.schedule(BEAT, '* * * * *');
  await boss.send(BEAT);
  return boss;
}
