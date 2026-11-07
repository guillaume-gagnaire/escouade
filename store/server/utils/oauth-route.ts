import { createError, getHeader, setResponseHeader, setResponseStatus, type H3Event } from 'h3';
import type { Db } from './db';
import { service } from './service';

// What the OAuth and MCP endpoints share (ESC-317): they read no cookie (a token or a code stands for the client), so
// any origin may call them (CORS), and they are never cached.

/** The base, or a 503 while the store starts. */
export function oauthDb(): Db {
  if (!service.db || service.starting) throw createError({ statusCode: 503, statusMessage: 'The store is starting' });
  return service.db;
}

/** CORS for a browser-based client, and no cache; true when the request was a preflight, answered here. */
export function openTo(event: H3Event, methods: string): boolean {
  setResponseHeader(event, 'cache-control', 'no-store');
  setResponseHeader(event, 'access-control-allow-origin', '*');
  setResponseHeader(event, 'access-control-allow-methods', `${methods}, OPTIONS`);
  setResponseHeader(
    event,
    'access-control-allow-headers',
    'authorization, content-type, mcp-protocol-version, mcp-session-id, last-event-id',
  );
  setResponseHeader(event, 'access-control-expose-headers', 'www-authenticate, mcp-session-id');
  setResponseHeader(event, 'access-control-max-age', 86_400);
  // An answer read by any origin (no cookie behind it): CORP must not stop a browser client.
  setResponseHeader(event, 'cross-origin-resource-policy', 'cross-origin');
  if (event.method !== 'OPTIONS') return false;
  setResponseStatus(event, 204);
  return true;
}

/** The answer of an OAuth error (RFC 6749 §5.2): 400, or 401 for a client the store does not know. */
export function oauthError(event: H3Event, e: { error: string; error_description: string }) {
  setResponseStatus(event, e.error === 'invalid_client' ? 401 : 400);
  return e;
}

/** Whether the request's body is the form or the JSON a client sends. */
export const isForm = (event: H3Event) => /^application\/(x-www-form-urlencoded|json)\b/i.test(getHeader(event, 'content-type') ?? '');
