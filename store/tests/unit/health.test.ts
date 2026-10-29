import { describe, expect, it } from 'vitest';
import { BEAT_LATE_MS, judge } from '../../server/utils/health';

const now = new Date('2026-10-29T10:00:00Z');
const recent = new Date(now.getTime() - 30_000);

describe('/api/health', () => {
  it('is ok when the database answers, the queue beat recently and the disk has room', () => {
    expect(judge({ starting: false, db: { ok: true }, beat: recent, disk: 0.42 }, now)).toEqual({
      status: 'ok',
      checks: { database: 'ok', queue: 'ok', disk: 'ok (42 %)' },
    });
  });

  it('is down when the queue has not beat for too long', () => {
    const late = new Date(now.getTime() - BEAT_LATE_MS - 1);
    const h = judge({ starting: false, db: { ok: true }, beat: late, disk: 0.1 }, now);
    expect(h.status).toBe('down');
    expect(h.checks.queue).toBe(`late since ${late.toISOString()}`);
  });

  it('is down when the disk of the archives reaches 80 %', () => {
    const h = judge({ starting: false, db: { ok: true }, beat: recent, disk: 0.8 }, now);
    expect(h.status).toBe('down');
    expect(h.checks.disk).toBe('full (80 %)');
  });

  it('is down when the disk cannot be read', () => {
    expect(judge({ starting: false, db: { ok: true }, beat: recent, disk: null }, now).status).toBe('down');
  });

  it('is down without a database, and says it is missing', () => {
    expect(judge({ starting: false, db: null, beat: null, disk: 0.1 }, now)).toEqual({
      status: 'down',
      checks: { database: 'missing', queue: 'unknown', disk: 'ok (10 %)' },
    });
  });

  it('says a database down without its error, which could name an internal address', () => {
    const h = judge({ starting: false, db: { ok: false }, beat: null, disk: 0.1 }, now);
    expect(h.checks.database).toBe('down');
    expect(h.status).toBe('down');
  });

  it('is starting, not down, while the migrations play', () => {
    expect(judge({ starting: true, db: null, beat: null, disk: 0.1 }, now)).toEqual({
      status: 'starting',
      checks: { database: 'starting', queue: 'unknown', disk: 'ok (10 %)' },
    });
  });

  it('waits for the first beat of the queue', () => {
    expect(judge({ starting: false, db: { ok: true }, beat: null, disk: 0.1 }, now).checks.queue).toBe('no beat yet');
  });
});
