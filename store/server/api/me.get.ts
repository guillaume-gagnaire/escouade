import { setResponseHeader } from 'h3';
import { admins } from '../utils/config';
import { currentAccount } from '../utils/session';

// Who is signed in, for the header: the login and whether they review; nothing when nobody is, or when the store
// cannot sign anyone in.
export default defineEventHandler(async (event) => {
  setResponseHeader(event, 'cache-control', 'no-store');
  const account = await currentAccount(event);
  if (!account) return { account: null };
  return { account: { login: account.login, githubId: account.githubId, admin: admins(process.env).includes(account.githubId) } };
});
