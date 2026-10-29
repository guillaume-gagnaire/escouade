import { createError, setResponseHeader } from 'h3';
import { REVALIDATE, notModified, requireDb } from '../../utils/app-api';
import { lastRevocations } from '../../utils/revocations';
import { sha256 } from '../../utils/versions';

// GET /api/v1/revocations.json.minisig: the signature of the last revocation list, as minisign wrote it.
export default defineEventHandler(async (event) => {
  const list = await lastRevocations(requireDb());
  setResponseHeader(event, 'cache-control', REVALIDATE);
  if (!list) throw createError({ statusCode: 404, statusMessage: 'No revocation list yet' });
  setResponseHeader(event, 'content-type', 'text/plain; charset=utf-8');
  if (notModified(event, `"${sha256(list.minisig)}"`)) return null;
  return list.minisig;
});
