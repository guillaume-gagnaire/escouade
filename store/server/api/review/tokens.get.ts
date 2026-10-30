import { setResponseHeader } from 'h3';
import { adminTokensOf } from '../../utils/review';
import { requireAdmin } from '../../utils/review-route';

// GET /api/review/tokens: the administration tokens the administrator holds (their label and start, never the token).
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAdmin(event);
  setResponseHeader(event, 'cache-control', 'no-store');
  return { tokens: await adminTokensOf(db, account) };
});
