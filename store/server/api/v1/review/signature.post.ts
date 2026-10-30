import { createError, readBody } from 'h3';
import { acceptSignature } from '../../../utils/review';
import { requireToken, tell } from '../../../utils/review-route';
import { service } from '../../../utils/service';

// POST /api/v1/review/signature { id, version, minisig, summary } (an administration token): the review's signature of
// a version, made on the user's machine; checked with the store's public keys before it is taken in (docs/STORE.md
// §4.5). A stolen token cannot make a seal: without the private key, nothing passes.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireToken(event);
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  const done = await acceptSignature(db, account, service.keys, body, tell());
  if ('error' in done)
    throw createError({
      statusCode: done.error === 'not-found' ? 404 : done.error === 'no-keys' ? 503 : 422,
      statusMessage: done.detail ? `${done.error}: ${done.detail}` : done.error,
    });
  return done;
});
