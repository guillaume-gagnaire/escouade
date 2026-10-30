import { setResponseHeader } from 'h3';
import { reviewQueue } from '../../../utils/review';
import { requireToken } from '../../../utils/review-route';
import { service } from '../../../utils/service';

// GET /api/v1/review/queue (an administration token): the review queue, for the script on the user's machine.
export default defineEventHandler(async (event) => {
  const { db } = await requireToken(event);
  setResponseHeader(event, 'cache-control', 'no-store');
  return reviewQueue(db, service.archives, service.keys);
});
