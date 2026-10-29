import { createError, getHeader, getRequestURL, setResponseHeader } from 'h3';
import { clientAddress, limiter, ruleFor } from '../utils/limits';

// The rate limits of the app's API, the downloads, the counter of installations and the submissions
// (server/utils/limits.ts): past its rule, a request gets a 429 and the seconds to wait.
export default defineEventHandler((event) => {
  const rule = ruleFor(event.method, getRequestURL(event).pathname);
  if (!rule) return;
  const address = clientAddress(getHeader(event, 'x-forwarded-for'), event.node.req.socket?.remoteAddress);
  const taken = limiter.take(rule, address);
  if (taken.ok) return;
  setResponseHeader(event, 'retry-after', taken.retryAfter);
  setResponseHeader(event, 'cache-control', 'no-store');
  throw createError({ statusCode: 429, statusMessage: 'Too many requests' });
});
