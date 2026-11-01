import { getRouterParam, readBody, sendRedirect } from 'h3';
import { revokeApp } from '../../../../utils/oauth';
import { requireAccount } from '../../../../utils/session';

// « Déconnecter » an MCP client from the author's space (ESC-317), by its form: its tokens stop at once, and the
// account's page says so.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  const done = await revokeApp(db, account, getRouterParam(event, 'ref'));
  const page = body.lang === 'en' ? '/en/account' : '/account';
  return sendRedirect(event, `${page}?notice=${done ? 'app-revoked' : 'app-missing'}#apps`, 303);
});
