import { setResponseHeader } from 'h3';
import { admins } from '../utils/config';
import { queueCount } from '../utils/review';
import { service } from '../utils/service';
import { currentAccount } from '../utils/session';

// Who is signed in, for the header: the login and whether they review (and how many wait for the review); nothing
// when nobody is, or when the store cannot sign anyone in.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const account = await currentAccount(event);
  if (!account) return { account: null };
  const admin = admins(process.env).includes(account.githubId);
  const queue = admin && service.db && !service.starting ? await queueCount(service.db) : 0;
  return { account: { login: account.login, githubId: account.githubId, admin, queue } };
});
