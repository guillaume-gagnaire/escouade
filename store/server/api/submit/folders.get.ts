import { getQuery, setResponseHeader } from 'h3';
import { repoFolders } from '../../utils/repo-read';
import { requireAccount } from '../../utils/session';
import { submissionsReady } from '../../utils/submit-route';

// The plugin's folder, picked before its version (ESC-313, docs/STORE.md §3.6): server/utils/repo-read.ts, shared
// with the MCP server's tools.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  return repoFolders(db, submissionsReady(), account, getQuery(event));
});
