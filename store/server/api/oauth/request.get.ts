import { createError, getQuery, setResponseHeader } from 'h3';
import { requestView } from '../../utils/oauth';
import { requireAccount } from '../../utils/session';

// What the consent page shows of a client's request (ESC-317): the client in its own words, the account, the scopes
// it asked and those the account gets (`review` to the administrators alone).
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  const view = await requestView(db, process.env, account, getQuery(event).request);
  if (!view) throw createError({ statusCode: 404, statusMessage: 'No such request' });
  return view;
});
