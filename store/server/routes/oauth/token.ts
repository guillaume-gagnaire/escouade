import { readBody, setResponseHeader } from 'h3';
import { tokenRequest } from '../../utils/oauth';
import { isForm, oauthDb, oauthError, openTo } from '../../utils/oauth-route';
import { siteUrl } from '../../utils/session';

// POST /oauth/token: a code for the first tokens (PKCE S256 checked), or a refresh token for new ones, swapped.
export default defineEventHandler(async (event) => {
  if (openTo(event, 'POST')) return '';
  setResponseHeader(event, 'pragma', 'no-cache');
  if (event.method !== 'POST' || !isForm(event)) return oauthError(event, { error: 'invalid_request', error_description: 'POST a form' });
  const body = ((await readBody(event).catch(() => null)) ?? {}) as Record<string, unknown>;
  const done = await tokenRequest(oauthDb(), process.env, siteUrl(), body);
  return 'error' in done ? oauthError(event, done) : done;
});
