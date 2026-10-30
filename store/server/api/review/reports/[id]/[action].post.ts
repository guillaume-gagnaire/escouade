import { createError, getRouterParam, readBody, sendRedirect } from 'h3';
import { closeReport } from '../../../../utils/review';
import { requireAdmin } from '../../../../utils/review-route';

// Closes a report from the page /review (docs/STORE.md §4.6): upheld (the review acts on the plugin) or dismissed;
// its sender is answered at the address they gave.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAdmin(event);
  const id = Number(getRouterParam(event, 'id'));
  const action = getRouterParam(event, 'action') ?? '';
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  if (!Number.isSafeInteger(id) || (action !== 'upheld' && action !== 'dismissed'))
    throw createError({ statusCode: 404, statusMessage: 'No such action' });
  const done = await closeReport(db, account, id, action, body.answer, process.env);
  const notice = 'error' in done ? `review-${done.error}` : `review-report-${action}`;
  const page = body.lang === 'en' ? '/en/review' : '/review';
  return sendRedirect(event, `${page}?tab=reports&notice=${notice}`, 303);
});
