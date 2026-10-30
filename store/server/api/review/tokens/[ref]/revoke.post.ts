import { createError, getRouterParam } from 'h3';
import { revokeAdminToken } from '../../../../utils/review';
import { requireAdmin } from '../../../../utils/review-route';

// POST /api/review/tokens/<ref>/revoke: the token stops working at once.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAdmin(event);
  if (!(await revokeAdminToken(db, account, getRouterParam(event, 'ref') ?? '')))
    throw createError({ statusCode: 404, statusMessage: 'No such token' });
  return { ok: true };
});
