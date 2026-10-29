import { mkdir, statfs } from 'node:fs/promises';
import type { PgBoss } from 'pg-boss';
import type pg from 'pg';
import type { Db } from './db';
import type { PublicKey } from './minisign';
import type { Deps } from './submissions';

// The service's state for this process: set up by server/plugins/boot.ts, read by /api/health.
export const service: {
  starting: boolean;
  pool: pg.Pool | null;
  db: Db | null;
  boss: PgBoss | null;
  archives: string;
  error: string | null;
  /** What the submissions need (the App's reading of repositories, the archives' folder), or null when the App is off. */
  deps: Deps | null;
  /** The public keys a signed file must be signed by (`store/keys/*.pub`). */
  keys: PublicKey[];
} = { starting: true, pool: null, db: null, boss: null, archives: './.data/archives', error: null, deps: null, keys: [] };

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
