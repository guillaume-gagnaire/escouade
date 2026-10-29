// What /api/health says (docs/STORE.md, §1.4): the database answers, the queue moves, the disk of the archives is
// under 80 %. Coolify reads it to put a new container online (or keep the old one) and to restart a stuck one; the
// probe outside the server reads it too.

/** The queue counts as stuck when its beat (every minute) is older than this. */
export const BEAT_LATE_MS = 3 * 60_000;
/** The disk counts as full from this share on. */
export const DISK_FULL = 0.8;

export interface Probe {
  /** The service is still starting (migrations, queue): not ready, not broken. */
  starting: boolean;
  /** null when no database is configured. Its error goes to the log, not here: it can name an internal address. */
  db: { ok: boolean } | null;
  /** The queue's last beat, null if none yet. */
  beat: Date | null;
  /** The share of the archives' disk in use, null if it could not be read. */
  disk: number | null;
}

export interface Health {
  status: 'ok' | 'starting' | 'down';
  checks: { database: string; queue: string; disk: string };
}

const percent = (x: number) => `${Math.round(x * 100)} %`;

export function judge(p: Probe, now: Date): Health {
  const database = p.starting ? 'starting' : p.db === null ? 'missing' : p.db.ok ? 'ok' : 'down';
  const late = p.beat === null || now.getTime() - p.beat.getTime() > BEAT_LATE_MS;
  const queue = p.db?.ok !== true ? 'unknown' : !late ? 'ok' : p.beat === null ? 'no beat yet' : `late since ${p.beat.toISOString()}`;
  const disk = p.disk === null ? 'unknown' : p.disk < DISK_FULL ? `ok (${percent(p.disk)})` : `full (${percent(p.disk)})`;
  const ok = database === 'ok' && queue === 'ok' && p.disk !== null && p.disk < DISK_FULL;
  return { status: ok ? 'ok' : p.starting ? 'starting' : 'down', checks: { database, queue, disk } };
}
