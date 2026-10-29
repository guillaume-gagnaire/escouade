import { deleteCookie, getCookie, getQuery, sendRedirect } from 'h3';
import { signIn } from '../../../utils/accounts';
import { exchangeCode, getUser, userInstallations } from '../../../utils/github';
import { sameSecret } from '../../../utils/tokens';
import { accountsReady, callbackUrl, localPath, setSessionCookie, STATE_COOKIE } from '../../../utils/session';

// GitHub sends the browser back here with a code, after a sign-in or an installation of the App. The state must be
// the one this browser left with. An installation started on GitHub itself comes back without it: the store then
// signs in again, with a state of its own, rather than trust a code it did not ask for.
export default defineEventHandler(async (event) => {
  const { db, app, secret } = accountsReady();
  const q = getQuery(event);
  const raw = getCookie(event, STATE_COOKIE);
  deleteCookie(event, STATE_COOKIE, { httpOnly: true, secure: true, sameSite: 'lax', path: '/' });
  let saved: { s?: string; to?: string; installed?: boolean } = {};
  try {
    saved = raw ? JSON.parse(raw) : {};
  } catch {
    saved = {};
  }
  const to = localPath(saved.to);
  const back = (notice: string) => sendRedirect(event, `${to.split('?')[0]}?notice=${notice}`, 302);
  const installing = typeof q.setup_action === 'string' || typeof q.installation_id === 'string';

  if (typeof q.error === 'string') return back('signin-cancelled');
  const ours = typeof q.state === 'string' && typeof saved.s === 'string' && sameSecret(q.state, saved.s);
  if (!ours) {
    if (installing) return sendRedirect(event, `/api/auth/github?to=${encodeURIComponent(to)}&installed=1`, 302);
    return back('signin-failed');
  }
  if (typeof q.code !== 'string' || !q.code) return back('signin-failed');

  try {
    const token = await exchangeCode(app, q.code, callbackUrl());
    const user = await getUser(app, token.token);
    const reached = await userInstallations(app, token.token);
    const { session } = await signIn(db, { user, token, installations: reached }, secret);
    setSessionCookie(event, session);
    return sendRedirect(event, installing || saved.installed ? `${to.split('?')[0]}?notice=installed` : to, 302);
  } catch (e) {
    console.error('[store] sign-in failed:', e instanceof Error ? e.message : e);
    return back('signin-failed');
  }
});
