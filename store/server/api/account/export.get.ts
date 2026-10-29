import { setResponseHeader } from 'h3';
import { exportAccount } from '../../utils/accounts';
import { requireAccount } from '../../utils/session';

// Everything the store keeps of the account, as a JSON file (RGPD, articles 15 and 20). A GET for a download: it
// changes nothing but the journal, and SameSite=Lax keeps it to the author's own visits.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const data = await exportAccount(db, account);
  setResponseHeader(event, 'cache-control', 'no-store');
  setResponseHeader(event, 'content-type', 'application/json; charset=utf-8');
  setResponseHeader(event, 'content-disposition', `attachment; filename="escouade-store-${account.login}.json"`);
  return JSON.stringify(data, null, 2);
});
