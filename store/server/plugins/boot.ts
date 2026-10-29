import { archiveDir, invalidLines, keysDir, missingLines } from '../utils/config';
import { migrateDb, migrationsDir, openDb } from '../utils/db';
import { githubApp } from '../utils/github';
import { startQueue } from '../utils/queue';
import { loadKeys } from '../utils/revocations';
import { service } from '../utils/service';
import { githubSource } from '../utils/source';

// At start: say what is off for want of a variable, play the migrations, then start the queue. Until then
// /api/health answers 503 "starting", so that Coolify keeps the old container online. A failure is logged and
// shown by /api/health; the pages are still served.
export default defineNitroPlugin((nitro) => {
  const env = process.env;
  for (const line of [...missingLines(env), ...invalidLines(env)]) console.warn(line);
  service.archives = archiveDir(env);
  void loadKeys(keysDir(env)).then((keys) => {
    service.keys = keys;
    if (!keys.length) console.warn(`[store] no public key in ${keysDir(env)}: no revocation list can be taken in`);
  });
  const app = githubApp(env);
  service.deps = app
    ? {
        source: (installationId) => githubSource(app, installationId),
        archives: service.archives,
        env,
        site: (env.NUXT_PUBLIC_SITE_URL ?? '').trim().replace(/\/+$/, '') || 'https://store.escouade.dev',
      }
    : null;

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
      service.boss = await startQueue(url, db, service.deps);
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
