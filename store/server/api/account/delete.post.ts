import { readBody, sendRedirect } from 'h3';
import { deleteAccount } from '../../utils/accounts';
import { revokeToken } from '../../utils/github';
import { clearSessionCookie, requireAccount } from '../../utils/session';

// Deletes the account, once its login is typed back: its sessions, its address and its installations' links go;
// the GitHub tokens still valid are revoked at GitHub; the journal keeps the deletion one year.
export default defineEventHandler(async (event) => {
  const { db, app, secret, account } = await requireAccount(event);
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const page = body.lang === 'en' ? '/en/account' : '/account';
  if (typeof body.confirm !== 'string' || body.confirm.trim() !== account.login)
    return sendRedirect(event, `${page}?notice=delete-unconfirmed`, 303);
  const tokens = await deleteAccount(db, account, secret);
  clearSessionCookie(event);
  await Promise.all(tokens.map((t) => revokeToken(app, t)));
  return sendRedirect(event, `${page}?notice=deleted`, 303);
});
