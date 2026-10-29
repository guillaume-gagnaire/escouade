import { getQuery, sendRedirect } from 'h3';
import { verifyEmail } from '../../../utils/accounts';
import { accountsReady } from '../../../utils/session';

// The link of the e-mail: its address becomes the account's, checked. It works once, for 24 hours, from any browser
// (the token says which account); a wrong or old one changes nothing.
export default defineEventHandler(async (event) => {
  const { db } = accountsReady();
  const q = getQuery(event);
  const page = q.lang === 'en' ? '/en/account' : '/account';
  const account = typeof q.token === 'string' && q.token ? await verifyEmail(db, q.token) : null;
  return sendRedirect(event, `${page}?notice=${account ? 'email-verified' : 'email-expired'}`, 302);
});
