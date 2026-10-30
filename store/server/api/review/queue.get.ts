import { setResponseHeader } from 'h3';
import { reviewQueue } from '../../utils/review';
import { requireAdmin } from '../../utils/review-route';
import { service } from '../../utils/service';

// GET /api/review/queue: the review queue for the page /review (an administrator signed in).
export default defineEventHandler(async (event) => {
  const { db } = await requireAdmin(event);
  setResponseHeader(event, 'cache-control', 'no-store');
  return reviewQueue(db, service.archives, service.keys);
});
