import { createError, setResponseHeader } from 'h3';
import { REVALIDATE, notModified, requireDb } from '../../utils/app-api';
import { lastRevocations } from '../../utils/revocations';
import { sha256 } from '../../utils/versions';

// GET /api/v1/revocations.json: the last revocation list, its bytes as the user signed them (docs/STORE.md §4.6);
// its signature at revocations.json.minisig. A 404 before the first list.
export default defineEventHandler(async (event) => {
  const list = await lastRevocations(requireDb());
  setResponseHeader(event, 'cache-control', REVALIDATE);
  if (!list) throw createError({ statusCode: 404, statusMessage: 'No revocation list yet' });
  setResponseHeader(event, 'content-type', 'application/json; charset=utf-8');
  if (notModified(event, `"${sha256(list.json)}"`)) return null;
  return list.json;
});
