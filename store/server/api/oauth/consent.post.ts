import { readBody, sendRedirect } from 'h3';
import { answerRequest } from '../../utils/oauth';
import { requireAccount, siteUrl } from '../../utils/session';

// The consent page's form (ESC-317): « Autoriser » or « Refuser », from the store's own page (the Origin is checked by
// server/middleware/origin.ts); the browser goes back to the client with its code, or with the refusal.
export default defineEventHandler(async (event) => {
  const { db, account } = await requireAccount(event);
  const body = ((await readBody(event).catch(() => ({}))) ?? {}) as Record<string, unknown>;
  const done = await answerRequest(db, process.env, siteUrl(), account, body.request, body.decision === 'allow');
  if (!done) return sendRedirect(event, `${body.lang === 'en' ? '/en' : ''}/oauth/consent?notice=expired`, 303);
  return sendRedirect(event, done.redirect, 303);
});
