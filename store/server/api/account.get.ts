import { setResponseHeader } from 'h3';
import { accountView } from '../utils/accounts';
import { mailOn } from '../utils/mail';
import { requireAccount } from '../utils/session';

// What the account's page shows: the author, their address, the App's installations they manage, and whether the
// store can send an e-mail at all.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  return { ...(await accountView(db, account)), mail: mailOn(process.env) };
});
