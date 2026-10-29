import { getRouterParam, setResponseHeader } from 'h3';
import { FOREVER, notModified, publishedVersion, requireDb } from '../../../../../utils/app-api';

// GET /api/v1/plugins/<id>/<version>/version.json: the version's canonical file, byte for byte (docs/STORE.md §3.4);
// its SHA-256 is the fingerprint the catalogue gives and the review signs. It never changes: cached for a year.
export default defineEventHandler(async (event) => {
  const v = await publishedVersion(requireDb(), getRouterParam(event, 'id'), getRouterParam(event, 'version'));
  setResponseHeader(event, 'cache-control', FOREVER);
  setResponseHeader(event, 'content-type', 'application/json; charset=utf-8');
  if (notModified(event, `"${v.versionSha256}"`)) return null;
  return v.versionJson;
});
