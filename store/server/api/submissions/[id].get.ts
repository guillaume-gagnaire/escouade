import { createError, getRouterParam, setResponseHeader } from 'h3';
import { requireAccount } from '../../utils/session';
import { versionId } from '../../utils/submit-route';
import { versionView } from '../../utils/submissions';

// A version as its author follows it: the checks as they run, then where it stands. Another account's is a 404.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  const view = await versionView(db, account, versionId(getRouterParam(event, 'id')));
  if (!view) throw createError({ statusCode: 404, statusMessage: 'No such version' });
  return view;
});
