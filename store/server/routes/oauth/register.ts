import { readBody, setResponseStatus } from 'h3';
import { registerClient } from '../../utils/oauth';
import { isForm, oauthDb, oauthError, openTo } from '../../utils/oauth-route';

// POST /oauth/register: a client registers itself (RFC 7591), public, with the addresses it may be sent back to.
export default defineEventHandler(async (event) => {
  if (openTo(event, 'POST')) return '';
  if (event.method !== 'POST' || !isForm(event))
    return oauthError(event, { error: 'invalid_request', error_description: 'POST a JSON body' });
  const body = (await readBody(event).catch(() => null)) as unknown;
  if (!body || typeof body !== 'object' || Array.isArray(body))
    return oauthError(event, { error: 'invalid_client_metadata', error_description: 'a JSON object' });
  const made = await registerClient(oauthDb(), body as Record<string, unknown>);
  if ('error' in made) return oauthError(event, made);
  setResponseStatus(event, 201);
  return made;
});
