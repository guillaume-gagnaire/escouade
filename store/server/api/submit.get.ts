import { setResponseHeader } from 'h3';
import { service } from '../utils/service';
import { requireAccount } from '../utils/session';
import { recentSubmissions, repoChoices } from '../utils/submissions';

// The first step of the submission (docs/STORE.md §3.1): the repositories the App reads among the author's
// installations (a private one is shown, and refused), their last submissions, and whether submitting is on.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const { db, account } = await requireAccount(event);
  return {
    on: !!(service.deps && service.boss),
    repos: await repoChoices(db, account.id),
    recent: await recentSubmissions(db, account.id),
  };
});
