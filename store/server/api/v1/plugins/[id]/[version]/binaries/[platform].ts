import { createError, getRouterParam } from 'h3';
import { getOrHead, publishedVersion, refuseDisabled, requireDb, sendKept } from '../../../../../../utils/app-api';
import { ext } from '../../../../../../utils/submissions';
import type { Binary } from '../../../../../../utils/versions';

// GET (and HEAD) /api/v1/plugins/<id>/<version>/binaries/<platform>: the release's program for a system (`windows-x64`…), kept
// under its SHA-256 (the one of version.json); a 410 once the revocation list disables the version.
export default defineEventHandler(async (event) => {
  getOrHead(event);
  const db = requireDb();
  const v = await publishedVersion(db, getRouterParam(event, 'id'), getRouterParam(event, 'version'));
  const platform = getRouterParam(event, 'platform') ?? '';
  const bin = Object.hasOwn(v.binaries ?? {}, platform) ? ((v.binaries as Record<string, Binary>)[platform] ?? null) : null;
  if (!bin) throw createError({ statusCode: 404, statusMessage: 'No program for this system' });
  await refuseDisabled(db, v.pluginId!, v.version!);
  return sendKept(event, bin.sha256, ext(bin.name), 'application/octet-stream', bin.name);
});
