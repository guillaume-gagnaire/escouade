import { createError, readBody, setResponseHeader } from 'h3';
import { makeAdminToken } from '../../utils/review';
import { requireAdmin } from '../../utils/review-route';

// POST /api/review/tokens { label }: makes an administration token for the review's script (docs/STORE.md §4.1). The
// token is in this answer alone, shown once by the page; the base keeps its hash. Never logged.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAdmin(event);
  setResponseHeader(event, 'cache-control', 'no-store');
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  const made = await makeAdminToken(db, account, String(body.label ?? ''));
  if ('error' in made) throw createError({ statusCode: 400, statusMessage: made.error });
  return made;
});
