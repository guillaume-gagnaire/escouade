import { createError, readBody } from 'h3';
import { depositRevocations } from '../../../utils/review';
import { requireToken, tell } from '../../../utils/review-route';
import { service } from '../../../utils/service';

// POST /api/v1/review/revocations { json, minisig, notify } (an administration token): a revocation list signed on the
// user's machine (docs/STORE.md §4.6), taken in only signed by a key of the store and with a serial above the last.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireToken(event);
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  const done = await depositRevocations(db, account, service.keys, body, tell());
  if ('error' in done) throw createError({ statusCode: 422, statusMessage: done.error });
  return done;
});
