import { and, eq } from 'drizzle-orm';
import { createError, setResponseHeader } from 'h3';
import { signatures } from '../../../../../db/schema';
import { REVALIDATE, notModified, publishedVersion, requireDb } from '../../../../../utils/app-api';
import { sha256 } from '../../../../../utils/versions';

// GET /api/v1/plugins/<id>/<version>/version.json.minisig: the review's signature of the version's `version.json`, as
// minisign wrote it on the user's machine (docs/STORE.md §4.5); the app checks it with the keys it embeds. A 404 for
// a version not verified. Revalidated: a signature may be made again with the backup key (§7).
export default defineEventHandler(async (event) => {
  const db = requireDb();
  const v = await publishedVersion(db, getRouterParam(event, 'id'), getRouterParam(event, 'version'));
  const [s] = await db
    .select()
    .from(signatures)
    .where(and(eq(signatures.pluginId, v.pluginId!), eq(signatures.version, v.version!)));
  setResponseHeader(event, 'cache-control', REVALIDATE);
  if (!s || s.versionSha256 !== v.versionSha256) throw createError({ statusCode: 404, statusMessage: 'Not verified' });
  setResponseHeader(event, 'content-type', 'text/plain; charset=utf-8');
  if (notModified(event, `"${sha256(s.minisig)}"`)) return null;
  return s.minisig;
});
