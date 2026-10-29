import { getRouterParam } from 'h3';
import { getOrHead, publishedVersion, refuseDisabled, requireDb, sendKept } from '../../../../../utils/app-api';

// GET (and HEAD) /api/v1/plugins/<id>/<version>/archive.tar.gz: the commit's archive as GitHub sent it at the submission, kept
// under its SHA-256 (the one of version.json); a 410 once the revocation list disables the version.
export default defineEventHandler(async (event) => {
  getOrHead(event);
  const db = requireDb();
  const v = await publishedVersion(db, getRouterParam(event, 'id'), getRouterParam(event, 'version'));
  await refuseDisabled(db, v.pluginId!, v.version!);
  return sendKept(event, v.archiveSha256!, '.tar.gz', 'application/gzip', `${v.pluginId}-${v.version}.tar.gz`);
});
