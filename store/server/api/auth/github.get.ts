import { getQuery, sendRedirect, setCookie } from 'h3';
import { authorizeUrl, installUrl } from '../../utils/github';
import { randomToken } from '../../utils/tokens';
import { accountsReady, callbackUrl, localPath, STATE_COOKIE } from '../../utils/session';

// Starts signing in with GitHub (or installing the App, `?install=1`, which signs in too: the App asks for the
// user's authorization at installation). The state goes to GitHub and into a cookie of ten minutes; the callback
// compares them, so that nobody can sign a visitor in as someone else.
export default defineEventHandler((event) => {
  const { app } = accountsReady();
  const q = getQuery(event);
  const state = randomToken();
  const to = localPath(q.to);
  setCookie(event, STATE_COOKIE, JSON.stringify({ s: state, to, installed: q.installed === '1' }), {
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
    path: '/',
    maxAge: 600,
  });
  return sendRedirect(event, q.install === '1' ? installUrl(app, state) : authorizeUrl(app, callbackUrl(), state), 302);
});
