import { getHeader, setResponseHeader } from 'h3';
import { REVALIDATE, notModified, requireDb } from '../../utils/app-api';
import { catalogBytes } from '../../utils/catalog';

// GET /api/v1/catalog.json: the whole catalogue the app searches on its machine (server/utils/catalog.ts),
// compressed when the client takes gzip, revalidated by its ETag.
export default defineEventHandler(async (event) => {
  const built = await catalogBytes(requireDb());
  setResponseHeader(event, 'cache-control', REVALIDATE);
  setResponseHeader(event, 'vary', 'accept-encoding');
  setResponseHeader(event, 'content-type', 'application/json; charset=utf-8');
  if (notModified(event, built.etag)) return null;
  if (/\bgzip\b/.test(getHeader(event, 'accept-encoding') ?? '')) {
    setResponseHeader(event, 'content-encoding', 'gzip');
    return built.gzip;
  }
  return built.body;
});
