import { archiveDir, invalidLines, missingLines } from '../utils/config';
import { migrateDb, migrationsDir, openDb } from '../utils/db';
import { startQueue } from '../utils/queue';
import { service } from '../utils/service';

// At start: say what is off for want of a variable, play the migrations, then start the queue. Until then
// /api/health answers 503 "starting", so that Coolify keeps the old container online. A failure is logged and
// shown by /api/health; the pages are still served.
export default defineNitroPlugin((nitro) => {
  const env = process.env;
  for (const line of [...missingLines(env), ...invalidLines(env)]) console.warn(line);
  service.archives = archiveDir(env);

  const url = (env.DATABASE_URL ?? '').trim();
  if (!url) {
    service.starting = false;
    return;
  }
  const { pool, db } = openDb(url);
  service.pool = pool;
  service.db = db;
  void (async () => {
    try {
      await migrateDb(db, migrationsDir(env));
      service.boss = await startQueue(url, db);
      console.info('[store] ready: migrations played, queue started');
    } catch (e) {
      service.error = e instanceof Error ? e.message : String(e);
      console.error('[store] start failed:', service.error);
    } finally {
      service.starting = false;
    }
  })();

  nitro.hooks.hook('close', async () => {
    await service.boss?.stop({ graceful: true, timeout: 5_000 }).catch(() => {});
    await pool.end().catch(() => {});
  });
});
