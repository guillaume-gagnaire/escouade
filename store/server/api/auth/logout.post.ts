import { readBody, sendRedirect } from 'h3';
import { signOut } from '../../utils/accounts';
import { revokeToken } from '../../utils/github';
import { accountsReady, clearSessionCookie, localPath, sessionToken } from '../../utils/session';

// Signs out: the session goes, and the GitHub token it held is revoked at GitHub.
export default defineEventHandler(async (event) => {
  const { db, app, secret } = accountsReady();
  const body = (await readBody(event).catch(() => ({}))) ?? {};
  const token = sessionToken(event);
  clearSessionCookie(event);
  if (token) {
    const github = await signOut(db, token, secret);
    if (github) await revokeToken(app, github);
  }
  return sendRedirect(event, localPath(body.to, '/'), 303);
});
