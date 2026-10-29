import { and, eq, sql } from 'drizzle-orm';
import { installsDaily, versions } from '../db/schema';
import type { Db } from './db';

// The counter of installations (docs/STORE.md, §5.4): the app sends `{ id, version, event }` only if its user
// accepted the statistics, without any id of the installation; the store adds one to the day's counter of that
// version and keeps nothing else. A version the store does not publish is not counted.

export type InstallEvent = 'install' | 'update';

/** The body of `POST /api/v1/installs`, read strictly: the three keys and nothing else. */
export function readInstall(body: unknown): { id: string; version: string; event: InstallEvent } | null {
  if (!body || typeof body !== 'object' || Array.isArray(body)) return null;
  const o = body as Record<string, unknown>;
  if (Object.keys(o).some((k) => !['id', 'version', 'event'].includes(k))) return null;
  const { id, version, event } = o;
  if (typeof id !== 'string' || !/^[a-z0-9][a-z0-9.-]{0,127}$/.test(id)) return null;
  if (typeof version !== 'string' || !/^[0-9A-Za-z.+-]{1,64}$/.test(version)) return null;
  if (event !== 'install' && event !== 'update') return null;
  return { id, version, event };
}

/** Adds one to the day's counter of a published version; false if the store publishes no such version. */
export async function countInstall(db: Db, what: { id: string; version: string; event: InstallEvent }, now = new Date()): Promise<boolean> {
  const [v] = await db
    .select({ id: versions.id })
    .from(versions)
    .where(and(eq(versions.pluginId, what.id), eq(versions.version, what.version), eq(versions.state, 'published')))
    .limit(1);
  if (!v) return false;
  await db
    .insert(installsDaily)
    .values({ pluginId: what.id, version: what.version, day: now.toISOString().slice(0, 10), event: what.event, count: 1 })
    .onConflictDoUpdate({
      target: [installsDaily.pluginId, installsDaily.version, installsDaily.day, installsDaily.event],
      set: { count: sql`${installsDaily.count} + 1` },
    });
  return true;
}
