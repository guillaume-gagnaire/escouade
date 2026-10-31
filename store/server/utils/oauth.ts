import { createHash, randomBytes } from 'node:crypto';
import { and, desc, eq, gt, inArray, isNotNull, isNull, lt, or, sql } from 'drizzle-orm';
import { accounts, audit, oauthClients, oauthGrants, oauthRequests, oauthTokens } from '../db/schema';
import { note, type Account } from './accounts';
import type { Db } from './db';
import type { Env } from './config';
import { isAdmin } from './review';
import { randomToken, sameSecret, tokenHash } from './tokens';

// The authorization of the store's MCP server (ESC-317, docs/STORE.md §2.5), after the MCP specification (2025-06-18,
// OAuth 2.1): the store is both the authorization server and the resource. A client registers itself (RFC 7591), is
// public (no secret: PKCE S256, always), finds the server by the metadata of the resource (RFC 9728) and of the server
// (RFC 8414), and names the resource it wants (RFC 8707): every token is good for `<site>/mcp` alone. The author signs
// in with GitHub as on the site, then consents. Access tokens last an hour, refresh tokens 30 days and are swapped at
// each use (one used twice revokes its grant); the base keeps the hash of each, never the token.

export const SCOPES = ['author', 'review'] as const;
export type Scope = (typeof SCOPES)[number];

export const ACCESS_SECONDS = 3600;
export const REFRESH_DAYS = 30;
export const CODE_SECONDS = 600;
export const REQUEST_SECONDS = 600;
/** What an access token starts with, and a refresh token: a token pasted in the wrong place is recognized. */
export const ACCESS_PREFIX = 'esc_mcp_';
export const REFRESH_PREFIX = 'esc_mcpr_';
/** Grants an account may hold at once, and clients registered from one address in an hour (limits.ts). */
export const GRANTS_MAX = 20;

/** The resource every token names: the store's MCP endpoint. */
export const mcpResource = (site: string) => `${site}/mcp`;
/** Where a client reads the resource's metadata (RFC 9728, §3.1: the resource's path after the well-known one). */
export const resourceMetadataUrl = (site: string) => `${site}/.well-known/oauth-protected-resource/mcp`;

/** The protected resource's metadata (RFC 9728). */
export function resourceMetadata(site: string) {
  return {
    resource: mcpResource(site),
    authorization_servers: [site],
    scopes_supported: [...SCOPES],
    bearer_methods_supported: ['header'],
    resource_name: 'Escouade Store',
    resource_documentation: `${site}/agents`,
  };
}

/** The authorization server's metadata (RFC 8414). */
export function serverMetadata(site: string) {
  return {
    issuer: site,
    authorization_endpoint: `${site}/oauth/authorize`,
    token_endpoint: `${site}/oauth/token`,
    registration_endpoint: `${site}/oauth/register`,
    revocation_endpoint: `${site}/oauth/revoke`,
    scopes_supported: [...SCOPES],
    response_types_supported: ['code'],
    response_modes_supported: ['query'],
    grant_types_supported: ['authorization_code', 'refresh_token'],
    code_challenge_methods_supported: ['S256'],
    token_endpoint_auth_methods_supported: ['none'],
    revocation_endpoint_auth_methods_supported: ['none'],
    authorization_response_iss_parameter_supported: true,
    service_documentation: `${site}/agents`,
    ui_locales_supported: ['fr', 'en'],
  };
}

// ---- Registration (RFC 7591) ----

const LOOPBACK = new Set(['localhost', '127.0.0.1', '[::1]']);

/** A redirect address a client may register: HTTPS, or HTTP on this machine's loopback (a CLI); no fragment, no user. */
export function cleanRedirectUri(raw: unknown): string | null {
  if (typeof raw !== 'string' || raw.length > 2000) return null;
  let u: URL;
  try {
    u = new URL(raw);
  } catch {
    return null;
  }
  if (u.hash || u.username || u.password) return null;
  if (u.protocol === 'https:' || (u.protocol === 'http:' && LOOPBACK.has(u.hostname))) return u.toString() === raw ? raw : u.toString();
  return null;
}

/** A client's own words, shown as such: one line, printable, bounded. */
export const cleanName = (raw: unknown, max = 100) =>
  typeof raw === 'string'
    ? raw
        .replace(/[\u0000-\u001f\u007f-\u009f​-‏‪-‮⁦-⁩]/g, '')
        .replace(/\s+/g, ' ')
        .trim()
        .slice(0, max)
    : '';

export type OAuthError = { error: string; error_description: string };
const fail = (error: string, error_description: string): OAuthError => ({ error, error_description });

/** Registers a public client; what it asks that the store does not do is refused, its way of signing in is « none ». */
export async function registerClient(db: Db, body: Record<string, unknown>, now = new Date()) {
  const uris = Array.isArray(body.redirect_uris) ? body.redirect_uris : null;
  if (!uris || uris.length < 1 || uris.length > 10) return fail('invalid_redirect_uri', 'redirect_uris: 1 to 10 addresses');
  const clean = uris.map(cleanRedirectUri);
  if (clean.some((u) => u === null)) return fail('invalid_redirect_uri', 'redirect_uris: https, or http on localhost, without a fragment');
  const grants = body.grant_types ?? ['authorization_code', 'refresh_token'];
  if (!Array.isArray(grants) || grants.some((g) => g !== 'authorization_code' && g !== 'refresh_token'))
    return fail('invalid_client_metadata', 'grant_types: authorization_code and refresh_token only');
  const responses = body.response_types ?? ['code'];
  if (!Array.isArray(responses) || responses.some((r) => r !== 'code')) return fail('invalid_client_metadata', 'response_types: code only');
  const uri = body.client_uri === undefined ? null : cleanRedirectUri(body.client_uri);
  if (body.client_uri !== undefined && (!uri || !uri.startsWith('https:')))
    return fail('invalid_client_metadata', 'client_uri: an https address');
  const name = cleanName(body.client_name) || 'MCP client';
  const id = `mcp_${randomBytes(16).toString('base64url')}`;
  const redirectUris = [...new Set(clean as string[])];
  await db.insert(oauthClients).values({ id, name, uri, redirectUris, createdAt: now });
  return {
    client_id: id,
    client_id_issued_at: Math.floor(now.getTime() / 1000),
    client_name: name,
    ...(uri ? { client_uri: uri } : {}),
    redirect_uris: redirectUris,
    grant_types: ['authorization_code', 'refresh_token'],
    response_types: ['code'],
    token_endpoint_auth_method: 'none',
    scope: SCOPES.join(' '),
  };
}

// ---- Authorization ----

/** The scopes asked, by their names; none asked is `author`. Null when one is unknown. */
export function parseScopes(raw: unknown): Scope[] | null {
  if (raw === undefined || raw === null || raw === '') return ['author'];
  if (typeof raw !== 'string' || raw.length > 200) return null;
  const asked = [...new Set(raw.split(' ').filter(Boolean))];
  return asked.every((s) => (SCOPES as readonly string[]).includes(s)) ? (asked as Scope[]) : null;
}

/** The language of the consent page: the client's `ui_locales`, else the browser's first, else French. */
export function consentLang(uiLocales: unknown, acceptLanguage: string | undefined): 'fr' | 'en' {
  const first = (typeof uiLocales === 'string' && uiLocales.split(' ')[0]) || (acceptLanguage ?? '').split(',')[0] || '';
  return /^en\b/i.test(first.trim()) ? 'en' : 'fr';
}

/** Where the browser goes back to the client, with what it is told (and the issuer, RFC 9207). */
export function backTo(redirectUri: string, site: string, params: Record<string, string | null | undefined>): string {
  const u = new URL(redirectUri);
  for (const [k, v] of Object.entries(params)) if (v !== null && v !== undefined) u.searchParams.set(k, v);
  u.searchParams.set('iss', site);
  return u.toString();
}

export type AuthorizationStart =
  /** Before the client and its address are known: shown to the user, never sent anywhere. */
  | { fatal: string }
  /** Sent back to the client's address. */
  | { redirect: string }
  | { request: string };

/** Reads `/oauth/authorize`'s query: the request kept ten minutes for the sign-in and the consent. */
export async function startAuthorization(db: Db, q: Record<string, unknown>, site: string, now = new Date()): Promise<AuthorizationStart> {
  if (typeof q.client_id !== 'string' || q.client_id.length > 100) return { fatal: 'invalid_client' };
  const [client] = await db.select().from(oauthClients).where(eq(oauthClients.id, q.client_id));
  if (!client) return { fatal: 'invalid_client' };
  const redirectUri =
    q.redirect_uri === undefined && client.redirectUris.length === 1 ? client.redirectUris[0]! : String(q.redirect_uri ?? '');
  if (!client.redirectUris.includes(redirectUri)) return { fatal: 'invalid_redirect_uri' };
  const state = typeof q.state === 'string' ? q.state.slice(0, 500) : null;
  const back = (error: string, error_description: string) => ({ redirect: backTo(redirectUri, site, { error, error_description, state }) });
  if (q.response_type !== 'code') return back('unsupported_response_type', 'response_type must be code');
  if (q.code_challenge_method !== 'S256') return back('invalid_request', 'PKCE with S256 is required');
  if (typeof q.code_challenge !== 'string' || !/^[A-Za-z0-9_-]{43}$/.test(q.code_challenge))
    return back('invalid_request', 'code_challenge: the S256 of a verifier, in base64url');
  const scopes = parseScopes(q.scope);
  if (!scopes || !scopes.length) return back('invalid_scope', `scope: ${SCOPES.join(', ')}`);
  if (q.resource !== undefined && q.resource !== mcpResource(site)) return back('invalid_target', `resource: ${mcpResource(site)} only`);
  const id = randomToken();
  await db.insert(oauthRequests).values({
    id: tokenHash(id),
    clientId: client.id,
    redirectUri,
    state,
    codeChallenge: q.code_challenge,
    scopes,
    resource: mcpResource(site),
    createdAt: now,
    expiresAt: new Date(now.getTime() + REQUEST_SECONDS * 1000),
  });
  return { request: id };
}

/** What the consent page shows of a request, for `account`: the client in its own words, what it may get. */
export async function requestView(db: Db, env: Env, account: Account, id: unknown, now = new Date()) {
  if (typeof id !== 'string' || id.length > 100) return null;
  const [row] = await db
    .select({ r: oauthRequests, c: oauthClients })
    .from(oauthRequests)
    .innerJoin(oauthClients, eq(oauthClients.id, oauthRequests.clientId))
    .where(and(eq(oauthRequests.id, tokenHash(id)), gt(oauthRequests.expiresAt, now)));
  if (!row) return null;
  const admin = isAdmin(env, account.githubId);
  return {
    client: { name: row.c.name, uri: row.c.uri, redirectHost: new URL(row.r.redirectUri).host, registeredAt: row.c.createdAt },
    account: { login: account.login, name: account.name },
    asked: row.r.scopes,
    granted: grantable(row.r.scopes, admin),
    refused: row.r.scopes.filter((s) => !(grantable(row.r.scopes, admin) as string[]).includes(s)),
    redirectOrigin: new URL(row.r.redirectUri).origin,
  };
}

/** The scopes an account gets of those asked: `review` to the administrators alone. */
export const grantable = (asked: string[], admin: boolean) => asked.filter((s) => s === 'author' || (s === 'review' && admin)) as Scope[];

/**
 * The author's answer on the consent page: the request used once, a grant and its code (ten minutes) when they
 * allow, the client told either way. Null when the request is gone (expired, or answered).
 */
export async function answerRequest(
  db: Db,
  env: Env,
  site: string,
  account: Account,
  id: unknown,
  allow: boolean,
  now = new Date(),
): Promise<{ redirect: string } | null> {
  if (typeof id !== 'string' || id.length > 100) return null;
  const [r] = await db
    .delete(oauthRequests)
    .where(and(eq(oauthRequests.id, tokenHash(id)), gt(oauthRequests.expiresAt, now)))
    .returning();
  if (!r) return null;
  const [client] = await db.select().from(oauthClients).where(eq(oauthClients.id, r.clientId));
  if (!client) return null;
  if (!allow) {
    await note(db, 'oauth.denied', { accountId: account.id, githubId: account.githubId }, { client: client.name }, now);
    return { redirect: backTo(r.redirectUri, site, { error: 'access_denied', error_description: 'The author refused', state: r.state }) };
  }
  const scopes = grantable(r.scopes, isAdmin(env, account.githubId));
  if (!scopes.length)
    return {
      redirect: backTo(r.redirectUri, site, { error: 'invalid_scope', error_description: 'review: administrators only', state: r.state }),
    };
  const live = await liveGrants(db, account.id, now);
  if (live.length >= GRANTS_MAX) {
    // The oldest gives way: an author who connects a 21st client does not stay locked out.
    const oldest = live.sort((a, b) => (a.usedAt ?? a.createdAt).getTime() - (b.usedAt ?? b.createdAt).getTime())[0]!;
    await db.update(oauthGrants).set({ revokedAt: now }).where(eq(oauthGrants.id, oldest.id));
  }
  const [grant] = await db
    .insert(oauthGrants)
    .values({ accountId: account.id, clientId: client.id, scopes, resource: r.resource, createdAt: now })
    .returning();
  const code = randomToken();
  await db.insert(oauthTokens).values({
    id: tokenHash(code),
    grantId: grant!.id,
    kind: 'code',
    codeChallenge: r.codeChallenge,
    redirectUri: r.redirectUri,
    createdAt: now,
    expiresAt: new Date(now.getTime() + CODE_SECONDS * 1000),
  });
  await note(
    db,
    'oauth.granted',
    { accountId: account.id, githubId: account.githubId },
    { client: client.name, grant: grant!.id, scopes, refused: r.scopes.filter((s) => !(scopes as string[]).includes(s)) },
    now,
  );
  return { redirect: backTo(r.redirectUri, site, { code, state: r.state }) };
}

async function liveGrants(db: Db, accountId: number, now: Date) {
  const rows = await db
    .select({ g: oauthGrants })
    .from(oauthGrants)
    .where(and(eq(oauthGrants.accountId, accountId), isNull(oauthGrants.revokedAt)));
  if (!rows.length) return [];
  const alive = await db
    .selectDistinct({ grantId: oauthTokens.grantId })
    .from(oauthTokens)
    .where(
      and(
        inArray(
          oauthTokens.grantId,
          rows.map((r) => r.g.id),
        ),
        gt(oauthTokens.expiresAt, now),
        isNull(oauthTokens.usedAt),
      ),
    );
  const ids = new Set(alive.map((a) => a.grantId));
  return rows.map((r) => r.g).filter((g) => ids.has(g.id));
}

// ---- The token endpoint ----

export interface TokenAnswer {
  access_token: string;
  token_type: 'Bearer';
  expires_in: number;
  refresh_token: string;
  scope: string;
}

/** The S256 of a PKCE verifier (RFC 7636, §4.2). */
export const s256 = (verifier: string) => createHash('sha256').update(verifier).digest('base64url');

async function issue(db: Db, grantId: number, scopes: string[], now: Date): Promise<TokenAnswer> {
  const access = `${ACCESS_PREFIX}${randomToken()}`;
  const refresh = `${REFRESH_PREFIX}${randomToken()}`;
  await db.insert(oauthTokens).values([
    { id: tokenHash(access), grantId, kind: 'access', createdAt: now, expiresAt: new Date(now.getTime() + ACCESS_SECONDS * 1000) },
    { id: tokenHash(refresh), grantId, kind: 'refresh', createdAt: now, expiresAt: new Date(now.getTime() + REFRESH_DAYS * 86_400_000) },
  ]);
  return { access_token: access, token_type: 'Bearer', expires_in: ACCESS_SECONDS, refresh_token: refresh, scope: scopes.join(' ') };
}

/** Revokes a grant: its tokens stop at once. */
async function revoke(db: Db, grantId: number, now: Date, why: string) {
  const [g] = await db
    .update(oauthGrants)
    .set({ revokedAt: now })
    .where(and(eq(oauthGrants.id, grantId), isNull(oauthGrants.revokedAt)))
    .returning();
  if (g) await note(db, `oauth.revoked`, { accountId: g.accountId }, { grant: g.id, why }, now);
}

/** A code or a refresh token, with its grant and the grant's account; marked used at once (the first to take it wins). */
async function take(db: Db, token: string, kind: 'code' | 'refresh', now: Date) {
  const id = tokenHash(token);
  const [row] = await db
    .select({ t: oauthTokens, g: oauthGrants, a: accounts })
    .from(oauthTokens)
    .innerJoin(oauthGrants, eq(oauthGrants.id, oauthTokens.grantId))
    .innerJoin(accounts, eq(accounts.id, oauthGrants.accountId))
    .where(and(eq(oauthTokens.id, id), eq(oauthTokens.kind, kind)));
  if (!row) return { error: 'unknown' as const };
  const [mine] = await db
    .update(oauthTokens)
    .set({ usedAt: now })
    .where(and(eq(oauthTokens.id, id), isNull(oauthTokens.usedAt)))
    .returning({ id: oauthTokens.id });
  if (!mine) {
    // Used twice: someone else holds it. The whole grant goes (OAuth 2.1, §4.3.1 and §6.1).
    await revoke(db, row.g.id, now, `${kind}-reused`);
    return { error: 'reused' as const };
  }
  if (row.t.expiresAt <= now) return { error: 'expired' as const };
  if (row.g.revokedAt) return { error: 'revoked' as const };
  return { ...row };
}

/** `POST /oauth/token`: a code for its first tokens (PKCE checked), or a refresh token for new ones (swapped). */
export async function tokenRequest(
  db: Db,
  env: Env,
  site: string,
  body: Record<string, unknown>,
  now = new Date(),
): Promise<TokenAnswer | OAuthError> {
  const clientId = typeof body.client_id === 'string' ? body.client_id : '';
  if (body.resource !== undefined && body.resource !== mcpResource(site))
    return fail('invalid_target', `resource: ${mcpResource(site)} only`);
  if (body.grant_type === 'authorization_code') {
    if (typeof body.code !== 'string' || body.code.length > 100) return fail('invalid_request', 'code is missing');
    if (typeof body.code_verifier !== 'string' || !/^[A-Za-z0-9._~-]{43,128}$/.test(body.code_verifier))
      return fail('invalid_request', 'code_verifier: 43 to 128 characters');
    const got = await take(db, body.code, 'code', now);
    if ('error' in got) return fail('invalid_grant', `the code is ${got.error}`);
    if (got.g.clientId !== clientId) return fail('invalid_grant', 'the code was given to another client');
    if (body.redirect_uri !== undefined && body.redirect_uri !== got.t.redirectUri) return fail('invalid_grant', 'redirect_uri differs');
    if (!sameSecret(s256(body.code_verifier), got.t.codeChallenge ?? '')) {
      await revoke(db, got.g.id, now, 'pkce');
      return fail('invalid_grant', 'code_verifier does not match the challenge');
    }
    await note(db, 'oauth.token', { accountId: got.a.id, githubId: got.a.githubId }, { grant: got.g.id }, now);
    return issue(db, got.g.id, scopesNow(env, got.a, got.g.scopes), now);
  }
  if (body.grant_type === 'refresh_token') {
    if (typeof body.refresh_token !== 'string' || !body.refresh_token.startsWith(REFRESH_PREFIX) || body.refresh_token.length > 200)
      return fail('invalid_request', 'refresh_token is missing');
    if (body.scope !== undefined) {
      const asked = parseScopes(body.scope);
      if (!asked) return fail('invalid_scope', `scope: ${SCOPES.join(', ')}`);
    }
    const got = await take(db, body.refresh_token, 'refresh', now);
    if ('error' in got) return fail('invalid_grant', `the refresh token is ${got.error}`);
    if (got.g.clientId !== clientId) return fail('invalid_grant', 'the token was given to another client');
    const asked = body.scope === undefined ? null : parseScopes(body.scope)!;
    if (asked && asked.some((s) => !got.g.scopes.includes(s))) return fail('invalid_scope', 'more than the grant');
    return issue(db, got.g.id, scopesNow(env, got.a, got.g.scopes), now);
  }
  return fail('unsupported_grant_type', 'authorization_code or refresh_token');
}

/** A grant's scopes as they stand: `review` only while the account is in STORE_ADMINS. */
export const scopesNow = (env: Env, a: Pick<Account, 'githubId'>, scopes: string[]) =>
  scopes.filter((s) => s !== 'review' || isAdmin(env, a.githubId));

/** `POST /oauth/revoke` (RFC 7009): an access or refresh token revokes its grant. Unknown tokens are not an error. */
export async function revokeByToken(db: Db, token: unknown, now = new Date()): Promise<void> {
  if (typeof token !== 'string' || token.length > 200) return;
  const [t] = await db
    .select()
    .from(oauthTokens)
    .where(eq(oauthTokens.id, tokenHash(token)));
  if (t && t.kind !== 'code') await revoke(db, t.grantId, now, 'client');
}

// ---- The resource ----

export interface Bearer {
  account: Account;
  grantId: number;
  client: { id: string; name: string };
  scopes: Scope[];
  expiresAt: Date;
}

/** Who a request to `/mcp` acts for: an access token not expired, of a grant not revoked, made for this resource. */
export async function bearerOf(
  db: Db,
  env: Env,
  site: string,
  header: string | undefined | null,
  now = new Date(),
): Promise<Bearer | { error: 'missing' | 'invalid' }> {
  const m = /^Bearer\s+(\S+)$/i.exec(header ?? '');
  if (!m) return { error: 'missing' };
  const token = m[1]!;
  if (!token.startsWith(ACCESS_PREFIX) || token.startsWith(REFRESH_PREFIX) || token.length > 200) return { error: 'invalid' };
  const [row] = await db
    .select({ t: oauthTokens, g: oauthGrants, a: accounts, c: oauthClients })
    .from(oauthTokens)
    .innerJoin(oauthGrants, eq(oauthGrants.id, oauthTokens.grantId))
    .innerJoin(accounts, eq(accounts.id, oauthGrants.accountId))
    .innerJoin(oauthClients, eq(oauthClients.id, oauthGrants.clientId))
    .where(and(eq(oauthTokens.id, tokenHash(token)), eq(oauthTokens.kind, 'access')));
  if (!row || row.t.expiresAt <= now || row.g.revokedAt || row.g.resource !== mcpResource(site)) return { error: 'invalid' };
  if (!row.g.usedAt || now.getTime() - row.g.usedAt.getTime() > 60_000)
    await db.update(oauthGrants).set({ usedAt: now }).where(eq(oauthGrants.id, row.g.id));
  return {
    account: row.a,
    grantId: row.g.id,
    client: { id: row.c.id, name: row.c.name },
    scopes: scopesNow(env, row.a, row.g.scopes) as Scope[],
    expiresAt: row.t.expiresAt,
  };
}

/** The `WWW-Authenticate` of a 401 from `/mcp` (RFC 9728 §5.1, RFC 6750 §3). */
export function challenge(site: string, error?: 'invalid_token' | 'insufficient_scope', scope?: string): string {
  const parts = [`resource_metadata="${resourceMetadataUrl(site)}"`];
  if (error) parts.push(`error="${error}"`);
  if (scope) parts.push(`scope="${scope}"`);
  return `Bearer ${parts.join(', ')}`;
}

// ---- « Applications connectées » ----

export interface ConnectedApp {
  ref: number;
  client: string;
  uri: string | null;
  redirectHost: string;
  scopes: string[];
  createdAt: Date;
  usedAt: Date | null;
  /** The last actions of its tools, newest first. */
  actions: { at: Date; tool: string; ok: boolean }[];
}

/** The clients an account let in and has not revoked, whose tokens still live; with their last actions. */
export async function connectedApps(db: Db, env: Env, account: Account, now = new Date()): Promise<ConnectedApp[]> {
  const live = await liveGrants(db, account.id, now);
  if (!live.length) return [];
  const clients = await db
    .select()
    .from(oauthClients)
    .where(
      inArray(
        oauthClients.id,
        live.map((g) => g.clientId),
      ),
    );
  const actions = await db
    .select({ at: audit.at, action: audit.action, detail: audit.detail })
    .from(audit)
    .where(
      and(
        eq(audit.accountId, account.id),
        sql`${audit.action} like 'mcp.%'`,
        inArray(
          sql`(${audit.detail}->>'grant')::int`,
          live.map((g) => g.id),
        ),
      ),
    )
    .orderBy(desc(audit.at), desc(audit.id))
    .limit(200);
  return live
    .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime() || b.id - a.id)
    .map((g) => {
      const c = clients.find((x) => x.id === g.clientId)!;
      return {
        ref: g.id,
        client: c.name,
        uri: c.uri,
        redirectHost: new URL(c.redirectUris[0]!).host,
        scopes: scopesNow(env, account, g.scopes),
        createdAt: g.createdAt,
        usedAt: g.usedAt,
        actions: actions
          .filter((a) => Number((a.detail as Record<string, unknown> | null)?.grant) === g.id)
          .slice(0, 10)
          .map((a) => ({ at: a.at, tool: a.action.slice(4), ok: (a.detail as Record<string, unknown>).ok !== false })),
      };
    });
}

/** The author revokes a client from their space. */
export async function revokeApp(db: Db, account: Account, ref: unknown, now = new Date()): Promise<boolean> {
  const id = Number(ref);
  if (!Number.isSafeInteger(id) || id <= 0 || id > 2 ** 31 - 1) return false;
  const done = await db
    .update(oauthGrants)
    .set({ revokedAt: now })
    .where(and(eq(oauthGrants.id, id), eq(oauthGrants.accountId, account.id), isNull(oauthGrants.revokedAt)))
    .returning({ id: oauthGrants.id });
  if (done.length)
    await note(db, 'oauth.revoked', { accountId: account.id, githubId: account.githubId }, { grant: id, why: 'author' }, now);
  return done.length > 0;
}

/** Forgets what has outlived its time: requests, codes and tokens past their end, grants dead a day, clients unused. */
export async function purgeOAuth(db: Db, now = new Date()) {
  const dayAgo = new Date(now.getTime() - 86_400_000);
  const r = await db.delete(oauthRequests).where(lt(oauthRequests.expiresAt, now)).returning({ id: oauthRequests.id });
  const t = await db
    .delete(oauthTokens)
    .where(or(lt(oauthTokens.expiresAt, now), and(isNotNull(oauthTokens.usedAt), lt(oauthTokens.usedAt, dayAgo))))
    .returning({ id: oauthTokens.id });
  const g = await db
    .delete(oauthGrants)
    .where(
      and(
        lt(oauthGrants.createdAt, dayAgo),
        or(
          isNotNull(oauthGrants.revokedAt),
          sql`not exists (select 1 from ${oauthTokens} where ${oauthTokens.grantId} = ${oauthGrants.id} and ${oauthTokens.expiresAt} > ${now})`,
        ),
      ),
    )
    .returning({ id: oauthGrants.id });
  const c = await db
    .delete(oauthClients)
    .where(
      and(
        lt(oauthClients.createdAt, dayAgo),
        sql`not exists (select 1 from ${oauthGrants} where ${oauthGrants.clientId} = ${oauthClients.id})`,
        sql`not exists (select 1 from ${oauthRequests} where ${oauthRequests.clientId} = ${oauthClients.id})`,
      ),
    )
    .returning({ id: oauthClients.id });
  return { requests: r.length, tokens: t.length, grants: g.length, clients: c.length };
}
