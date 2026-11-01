import { getRequestURL } from 'h3';
import { resourceMetadata, serverMetadata } from '../utils/oauth';
import { openTo } from '../utils/oauth-route';
import { siteUrl } from '../utils/session';

// The metadata of the MCP server's authorization (ESC-317): the protected resource's (RFC 9728, at the well-known
// address and at its path for /mcp) and the authorization server's (RFC 8414). Served here rather than by a file of
// server/routes, whose dotted folder the scan may skip.
export default defineEventHandler((event) => {
  const path = getRequestURL(event).pathname;
  if (!path.startsWith('/.well-known/oauth-')) return;
  if (event.method !== 'GET' && event.method !== 'HEAD' && event.method !== 'OPTIONS') return;
  if (path === '/.well-known/oauth-protected-resource' || path === '/.well-known/oauth-protected-resource/mcp') {
    if (openTo(event, 'GET')) return '';
    return resourceMetadata(siteUrl());
  }
  if (path === '/.well-known/oauth-authorization-server') {
    if (openTo(event, 'GET')) return '';
    return serverMetadata(siteUrl());
  }
});
