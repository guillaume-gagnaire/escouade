import { sql } from 'drizzle-orm';
import { judge, type Probe } from '../utils/health';
import { lastBeat } from '../utils/queue';
import { diskUse, service } from '../utils/service';

// GET /api/health: 200 when the database answers, the queue moves and the disk has room; 503 otherwise (or while
// starting). No secret, no address: the three checks and the status.
export default defineEventHandler(async (event) => {
  const probe: Probe = { starting: service.starting, db: null, beat: null, disk: await diskUse(service.archives) };
  // While starting, the tables may not be there yet: nothing to ask the database.
  if (service.db && !service.starting) {
    try {
      if (service.error) throw new Error(service.error);
      await service.db.execute(sql`select 1`);
      probe.db = { ok: true };
      probe.beat = await lastBeat(service.db);
    } catch (e) {
      console.error('[store] health: database down:', e instanceof Error ? e.message : e);
      probe.db = { ok: false };
    }
  }
  const health = judge(probe, new Date());
  setResponseStatus(event, health.status === 'ok' ? 200 : 503);
  setHeader(event, 'cache-control', 'no-store');
  return health;
});
