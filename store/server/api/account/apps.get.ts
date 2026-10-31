import { setResponseHeader } from 'h3';
import { connectedApps } from '../../utils/oauth';
import { requireAccount } from '../../utils/session';

// « Applications connectées » (ESC-317): the MCP clients the author let in, their scopes, their last use and actions.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  return { apps: await connectedApps(db, process.env, account) };
});
