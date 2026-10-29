import { mkdir, statfs } from 'node:fs/promises';
import type { PgBoss } from 'pg-boss';
import type pg from 'pg';
import type { Db } from './db';

// The service's state for this process: set up by server/plugins/boot.ts, read by /api/health.
export const service: {
  starting: boolean;
  pool: pg.Pool | null;
  db: Db | null;
  boss: PgBoss | null;
  archives: string;
  error: string | null;
} = { starting: true, pool: null, db: null, boss: null, archives: './.data/archives', error: null };

/** The share of the disk in use under `dir` (made if missing), or null if it cannot be read. */
export async function diskUse(dir: string): Promise<number | null> {
  try {
    await mkdir(dir, { recursive: true });
    const s = await statfs(dir);
    return s.blocks > 0 ? 1 - s.bavail / s.blocks : null;
  } catch {
    return null;
  }
}
