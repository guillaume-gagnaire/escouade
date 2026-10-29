import { createError, getRouterParam, readBody, sendRedirect } from 'h3';
import { decide, settleHeld } from '../../../../utils/review';
import { requireAdmin, tell } from '../../../../utils/review-route';

// What the administrator decides from the page /review (docs/STORE.md §4.3), by its forms: ask changes of a version
// or refuse it (it stays published, unverified; its author is told why), let a version held for a name too close pass
// or refuse it. « Vérifier » is not here: the signature is made on the user's machine.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAdmin(event);
  const id = Number(getRouterParam(event, 'id'));
  const action = getRouterParam(event, 'action') ?? '';
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  if (!Number.isSafeInteger(id)) throw createError({ statusCode: 404, statusMessage: 'No such version' });
  const acts: Record<string, () => Promise<{ ok: true } | { error: string }>> = {
    changes: () => decide(db, account, id, 'changes', body.message, tell()),
    refuse: () => decide(db, account, id, 'refused', body.message, tell()),
    release: () => settleHeld(db, account, id, 'release', body.message, tell()),
    'refuse-held': () => settleHeld(db, account, id, 'refuse', body.message, tell()),
  };
  const act = acts[action];
  if (!act) throw createError({ statusCode: 404, statusMessage: 'No such action' });
  const done = await act();
  const tab = action === 'release' || action === 'refuse-held' ? 'names' : 'queue';
  const notice = 'error' in done ? `review-${done.error}` : `review-${action}`;
  const page = body.lang === 'en' ? '/en/review' : '/review';
  const keep = 'error' in done ? `&v=${id}` : '';
  return sendRedirect(event, `${page}?tab=${tab}${keep}&notice=${notice}`, 303);
});
