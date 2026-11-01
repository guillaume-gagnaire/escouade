import { WebStandardStreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js';
import { getHeader, setResponseHeader, setResponseStatus, toWebRequest } from 'h3';
import { clientAddress } from '../utils/limits';
import { mcpServer } from '../utils/mcp';
import { bearerOf, challenge } from '../utils/oauth';
import { oauthDb, openTo } from '../utils/oauth-route';
import { service } from '../utils/service';
import { siteUrl } from '../utils/session';

// The store's MCP server (ESC-317, docs/STORE.md §2.5): Streamable HTTP, stateless (each POST is a whole exchange, its
// answer in JSON), behind an OAuth access token made for this resource. Without one, a 401 and where to find the
// authorization (RFC 9728). No session, no stream: GET and DELETE answer 405.
export default defineEventHandler(async (event) => {
  if (openTo(event, 'POST, GET, DELETE')) return '';
  const db = oauthDb();
  const site = siteUrl();
  const bearer = await bearerOf(db, process.env, site, getHeader(event, 'authorization'));
  if ('error' in bearer) {
    setResponseStatus(event, 401);
    setResponseHeader(event, 'www-authenticate', challenge(site, bearer.error === 'invalid' ? 'invalid_token' : undefined));
    return { error: bearer.error === 'invalid' ? 'invalid_token' : 'unauthorized' };
  }
  if (event.method !== 'POST') {
    setResponseStatus(event, 405);
    setResponseHeader(event, 'allow', 'POST');
    return { error: 'method_not_allowed' };
  }
  const server = mcpServer({
    db,
    env: process.env,
    site,
    bearer,
    address: clientAddress(getHeader(event, 'x-forwarded-for'), event.node.req.socket?.remoteAddress),
    deps: service.deps && service.boss ? { ...service.deps, boss: service.boss } : null,
  });
  const transport = new WebStandardStreamableHTTPServerTransport({ sessionIdGenerator: undefined, enableJsonResponse: true });
  await server.connect(transport);
  try {
    return await transport.handleRequest(toWebRequest(event));
  } finally {
    void server.close();
  }
});
