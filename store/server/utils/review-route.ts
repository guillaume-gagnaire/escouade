import { createError, getHeader, type H3Event } from 'h3';
import type { Account } from './accounts';
import type { Db } from './db';
import type { Tell } from './author';
import { isAdmin, tokenAdmin } from './review';
import { service } from './service';
import { requireAccount, siteUrl } from './session';

// Who may act on the review queue (docs/STORE.md, §4.1): an administrator (STORE_ADMINS) signed in, for the page
// /review and their tokens; the bearer of an administration token, for the review's script (/api/v1/review/*), which
// takes no cookie, so that another site cannot make a browser act there.

/** The administrator of the session, or a 401 (not signed in) or a 404 (the queue does not exist for them). */
export async function requireAdmin(event: H3Event): Promise<{ db: Db; account: Account }> {
  const { db, account } = await requireAccount(event);
  if (!isAdmin(process.env, account.githubId)) throw createError({ statusCode: 404, statusMessage: 'Page not found' });
  return { db, account };
}

/** The administrator of the request's token, or a 401. */
export async function requireToken(event: H3Event): Promise<{ db: Db; account: Account }> {
  if (!service.db || service.starting) throw createError({ statusCode: 503, statusMessage: 'The store is starting' });
  const account = await tokenAdmin(service.db, process.env, getHeader(event, 'authorization'));
  if (!account) throw createError({ statusCode: 401, statusMessage: 'An administration token is needed' });
  return { db: service.db, account };
}

/** Where the review's e-mails go from. */
export const tell = (): Tell => ({ env: process.env, site: siteUrl() });
