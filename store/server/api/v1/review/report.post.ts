import { createError, readBody } from 'h3';
import { postReport } from '../../../utils/review';
import { requireToken } from '../../../utils/review-route';

// POST /api/v1/review/report { id, version, verdict, report } (an administration token): the review's report, for the
// queue's page; the administrators alone read it.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireToken(event);
  const done = await postReport(db, account, ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>);
  if ('error' in done) throw createError({ statusCode: done.error === 'not-found' ? 404 : 400, statusMessage: done.error });
  return done;
});
