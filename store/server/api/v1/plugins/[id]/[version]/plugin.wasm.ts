import { createError, getRouterParam } from 'h3';
import { getOrHead, publishedVersion, refuseDisabled, requireDb, sendKept } from '../../../../../utils/app-api';
import { wasmOf } from '../../../../../utils/versions';

// GET (and HEAD) /api/v1/plugins/<id>/<version>/plugin.wasm: the module the store compiled of a run.js plugin
// (ESC-323), kept under its SHA-256, the one version.json names; the app checks it against it before precompiling it.
// A 404 for a plugin of another kind, a 410 once the revocation list disables the version.
export default defineEventHandler(async (event) => {
  getOrHead(event);
  const db = requireDb();
  const v = await publishedVersion(db, getRouterParam(event, 'id'), getRouterParam(event, 'version'));
  const wasm = wasmOf(v.versionJson);
  if (!wasm) throw createError({ statusCode: 404, statusMessage: 'No module for this version' });
  await refuseDisabled(db, v.pluginId!, v.version!);
  return sendKept(event, wasm.sha256, '.wasm', 'application/wasm', `${v.pluginId}-${v.version}.wasm`);
});
