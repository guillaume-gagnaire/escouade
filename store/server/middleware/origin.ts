import { createError, getHeader, getRequestURL } from 'h3';
import { siteUrl } from '../utils/session';

// What changes something on the store (a form of the account, the sign-out) must come from the store's own pages:
// the browser's Origin is checked, on top of the cookie's SameSite=Lax. GitHub's webhooks are signed instead.
const SAFE = new Set(['GET', 'HEAD', 'OPTIONS']);

export default defineEventHandler((event) => {
  if (SAFE.has(event.method)) return;
  const path = getRequestURL(event).pathname;
  if (path === '/api/github/webhook') return;
  const origin = getHeader(event, 'origin');
  if (!origin || origin !== new URL(siteUrl()).origin) throw createError({ statusCode: 403, statusMessage: 'Cross-site request refused' });
});
