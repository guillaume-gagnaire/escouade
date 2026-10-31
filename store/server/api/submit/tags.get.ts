import { getQuery, setResponseHeader } from 'h3';
import { repoTags } from '../../utils/repo-read';
import { requireAccount } from '../../utils/session';
import { submissionsReady } from '../../utils/submit-route';

// The second step: the tags and releases of a repository the author picked (server/utils/repo-read.ts, shared with
// the MCP server's tools).
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  return repoTags(db, submissionsReady(), account, getQuery(event));
});
