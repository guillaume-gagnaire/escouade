import { readBody } from 'h3';
import { revokeByToken } from '../../utils/oauth';
import { isForm, oauthDb, oauthError, openTo } from '../../utils/oauth-route';

// POST /oauth/revoke (RFC 7009): the client lets go of its grant. An unknown token is not an error.
export default defineEventHandler(async (event) => {
  if (openTo(event, 'POST')) return '';
  if (event.method !== 'POST' || !isForm(event)) return oauthError(event, { error: 'invalid_request', error_description: 'POST a form' });
  const body = ((await readBody(event).catch(() => null)) ?? {}) as Record<string, unknown>;
  await revokeByToken(oauthDb(), body.token);
  return {};
});
