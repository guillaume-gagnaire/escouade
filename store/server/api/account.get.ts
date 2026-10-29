import { getQuery, setResponseHeader } from 'h3';
import { accountView } from '../utils/accounts';
import { authorPlugins, pendingVersions, pluginDetail, repoClaims, transferOffers } from '../utils/author';
import { mailOn } from '../utils/mail';
import { requireAccount } from '../utils/session';

// What the account's page shows: the author, their address, the App's installations they manage, whether the store
// can send an e-mail at all; their plugins and their states, the versions waiting for a click, the plugins offered to
// them or whose repository moved to them; and the plugin chosen by `?plugin=` (else the first): its versions and stats.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  const now = new Date();
  const plugins = await authorPlugins(db, account, now);
  const asked = getQuery(event).plugin;
  const chosen = plugins.find((p) => p.id === asked && p.submission === null) ?? plugins.find((p) => p.submission === null);
  return {
    ...(await accountView(db, account, now)),
    mail: mailOn(process.env),
    plugins,
    pending: await pendingVersions(db, account, now),
    offers: await transferOffers(db, account),
    claims: await repoClaims(db, account),
    selected: chosen ? await pluginDetail(db, account, chosen.id, now) : null,
    // The page's clock, for « il y a 12 min » written the same on the server and in the browser.
    now: now.toISOString(),
  };
});
