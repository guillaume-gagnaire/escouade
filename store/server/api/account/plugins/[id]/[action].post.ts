import { createError, getRouterParam, readBody, sendRedirect } from 'h3';
import {
  answerTransfer,
  askTransfer,
  cancelTransfer,
  claimPlugin,
  deletePlugin,
  restorePlugin,
  setAutoPublish,
  setListed,
  type Done,
} from '../../../../utils/author';
import { requireAccount, siteUrl } from '../../../../utils/session';

// What an author does of one of their plugins, from the forms of their account (docs/STORE.md, §2.2): take it off
// the list or put it back, let new tags be published alone, offer it to another account (or take the offer back),
// accept or refuse a plugin offered, take over one whose repository moved to them, delete it (its id typed back) or
// undo the deletion. Each answers with the account's page and a notice.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const id = getRouterParam(event, 'id') ?? '';
  const action = getRouterParam(event, 'action') ?? '';
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const how = { env: process.env, site: siteUrl() };
  const acts: Record<string, () => Promise<Done<string>>> = {
    unlist: () => setListed(db, account, id, false, how),
    relist: () => setListed(db, account, id, true, how),
    'auto-on': () => setAutoPublish(db, account, id, true),
    'auto-off': () => setAutoPublish(db, account, id, false),
    transfer: () => askTransfer(db, account, id, body.login, how),
    'cancel-transfer': () => cancelTransfer(db, account, id),
    accept: () => answerTransfer(db, account, id, true, how),
    refuse: () => answerTransfer(db, account, id, false, how),
    claim: () => claimPlugin(db, account, id, how),
    delete: () => deletePlugin(db, account, id, body.confirm, how),
    restore: () => restorePlugin(db, account, id, how),
  };
  const act = acts[action];
  if (!act) throw createError({ statusCode: 404, statusMessage: 'No such action' });
  const done = await act();
  const notice = 'error' in done ? `plugin-${done.error}` : `plugin-${action}`;
  const page = body.lang === 'en' ? '/en/account' : '/account';
  return sendRedirect(event, `${page}?plugin=${encodeURIComponent(id)}&notice=${notice}`, 303);
});
