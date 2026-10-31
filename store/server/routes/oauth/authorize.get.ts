import { getHeader, getQuery, sendRedirect, setResponseHeader, setResponseStatus } from 'h3';
import { consentLang, startAuthorization } from '../../utils/oauth';
import { oauthDb } from '../../utils/oauth-route';
import { accountsReady, currentAccount, siteUrl } from '../../utils/session';

// GET /oauth/authorize: a client asks for an author's consent. Its client and address checked first (an unknown one is
// told here, never sent anywhere); then the request kept ten minutes, the sign-in with GitHub if there is no session,
// and the consent page in the browser's language.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const db = oauthDb();
  accountsReady();
  const q = getQuery(event);
  const started = await startAuthorization(db, q, siteUrl());
  if ('fatal' in started) {
    setResponseStatus(event, 400);
    setResponseHeader(event, 'content-type', 'text/plain; charset=utf-8');
    return started.fatal === 'invalid_client'
      ? 'Unknown MCP client: it must register again (RFC 7591). Client MCP inconnu : il doit s’enregistrer à nouveau.'
      : 'This return address is not one the client registered. Cette adresse de retour n’est pas une de celles que le client a enregistrées.';
  }
  if ('redirect' in started) return sendRedirect(event, started.redirect, 302);
  const lang = consentLang(q.ui_locales, getHeader(event, 'accept-language'));
  const page = `${lang === 'en' ? '/en' : ''}/oauth/consent?request=${started.request}`;
  if (await currentAccount(event)) return sendRedirect(event, page, 302);
  return sendRedirect(event, `/api/auth/github?to=${encodeURIComponent(page)}`, 302);
});
