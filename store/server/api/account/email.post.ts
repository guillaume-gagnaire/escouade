import { readBody, sendRedirect } from 'h3';
import { askEmail, removeEmail } from '../../utils/accounts';
import { emailCheckMail, sendMail } from '../../utils/mail';
import { requireAccount, siteUrl } from '../../utils/session';

// The account's address for the notices, from its form: a link is sent to check it, and the address is kept once the
// link is followed. Without SMTP_URL the link does not leave, and the page says so. `action=remove` forgets it.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const lang = body.lang === 'en' ? 'en' : 'fr';
  const page = lang === 'en' ? '/en/account' : '/account';
  const back = (notice: string) => sendRedirect(event, `${page}?notice=${notice}`, 303);

  if (body.action === 'remove') {
    await removeEmail(db, account);
    return back('email-removed');
  }
  const asked = await askEmail(db, account, body.email);
  if ('error' in asked) return back(`email-${asked.error}`);
  const link = `${siteUrl()}/api/account/email/verify?${new URLSearchParams({ token: asked.token, lang })}`;
  const sent = await sendMail(process.env, emailCheckMail(lang, asked.email, link));
  return back(sent ? 'email-sent' : 'email-not-sent');
});
