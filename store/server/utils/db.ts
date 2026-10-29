import { drizzle, type NodePgDatabase } from 'drizzle-orm/node-postgres';
import { migrate } from 'drizzle-orm/node-postgres/migrator';
import pg from 'pg';
import * as schema from '../db/schema';

export type Db = NodePgDatabase<typeof schema>;

/** The pool and its Drizzle handle, for one database. */
export function openDb(url: string): { pool: pg.Pool; db: Db } {
  const pool = new pg.Pool({ connectionString: url, max: 10 });
  // An idle client that loses its server (a restart of store-db) must not bring the process down.
  pool.on('error', (e) => console.error('[store] database:', e.message));
  return { pool, db: drizzle(pool, { schema }) };
}

/** Where the migrations are read from: server/db/migrations, from the store's folder (the image's /app too). */
export const migrationsDir = (env: Record<string, string | undefined>) => env.MIGRATIONS_DIR || './server/db/migrations';

/** Plays the migrations not yet played, in order; they only add (docs/STORE.md, §1.3). */
export async function migrateDb(db: Db, folder: string): Promise<void> {
  await migrate(db, { migrationsFolder: folder });
}
