import { createHash, randomBytes } from 'node:crypto';
import pg from 'pg';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { deleteAccount, signIn, type Account } from '../../server/utils/accounts';
import { migrateDb, openDb, type Db } from '../../server/utils/db';
import {
  ACCESS_SECONDS,
  answerRequest,
  bearerOf,
  connectedApps,
  mcpResource,
  purgeOAuth,
  REFRESH_DAYS,
  registerClient,
  requestView,
  revokeApp,
  revokeByToken,
  startAuthorization,
  tokenRequest,
  type TokenAnswer,
} from '../../server/utils/oauth';

// The MCP server's OAuth against a real PostgreSQL (TEST_DATABASE_URL alone, emptied first; skipped without it):
// registration, the authorization and its consent, PKCE S256, the resource (RFC 8707), the access token's hour, the
// refresh token swapped (and a second use that revokes the grant), the scopes (`review` to administrators alone),
// « Applications connectées » and the account deleted. The whole flow with a real MCP client is in tests/e2e/mcp.spec.ts.
const url = process.env.TEST_DATABASE_URL;
const SECRET = 's'.repeat(64);
const SITE = 'https://store.example';
const T0 = new Date('2026-10-31T10:00:00Z');
const later = (ms: number) => new Date(T0.getTime() + ms);
const ADMIN_ID = 7001;
const env = { STORE_ADMINS: String(ADMIN_ID) };
const REDIRECT = 'http://127.0.0.1:33418/callback';

const verifier = () => randomBytes(32).toString('base64url');
const challengeOf = (v: string) => createHash('sha256').update(v).digest('base64url');

describe.skipIf(!url)('the MCP server’s OAuth', () => {
  let pool: pg.Pool;
  let db: Db;
  let author: Account;
  let admin: Account;

  beforeAll(async () => {
    const c = new pg.Client({ connectionString: url });
    await c.connect();
    await c.query(
      'drop schema if exists drizzle cascade; drop schema if exists pgboss cascade; drop schema public cascade; create schema public;',
    );
    await c.end();
    ({ pool, db } = openDb(url!));
    await migrateDb(db, './server/db/migrations');
  });
  beforeEach(async () => {
    await pool.query('truncate accounts, sessions, audit, oauth_clients, oauth_requests, oauth_grants, oauth_tokens cascade');
    const sign = async (id: number, login: string) =>
      (
        await signIn(
          db,
          { user: { id, login, name: null, avatar_url: '' }, token: { token: `ghu_${login}`, expiresAt: null }, installations: [] },
          SECRET,
          T0,
        )
      ).account;
    author = await sign(7000, 'ana-author');
    admin = await sign(ADMIN_ID, 'chef');
  });
  afterAll(async () => {
    await pool?.end();
  });

  const register = async (extra: Record<string, unknown> = {}) => {
    const c = await registerClient(db, { client_name: 'Claude Code (escouade-store)', redirect_uris: [REDIRECT], ...extra }, T0);
    if ('error' in c) throw new Error(c.error);
    return c.client_id;
  };

  /** The way to a first pair of tokens: authorize, consent, exchange the code. */
  async function connect(who: Account, scope = 'author', opts: { verifier?: string; at?: Date } = {}) {
    const clientId = await register();
    const v = verifier();
    const at = opts.at ?? T0;
    const started = await startAuthorization(
      db,
      {
        client_id: clientId,
        redirect_uri: REDIRECT,
        response_type: 'code',
        code_challenge: challengeOf(v),
        code_challenge_method: 'S256',
        scope,
        state: 'st4te',
        resource: mcpResource(SITE),
      },
      SITE,
      at,
    );
    if (!('request' in started)) throw new Error(JSON.stringify(started));
    const answered = await answerRequest(db, env, SITE, who, started.request, true, at);
    const back = new URL(answered!.redirect);
    const code = back.searchParams.get('code')!;
    expect(back.searchParams.get('state')).toBe('st4te');
    expect(back.searchParams.get('iss')).toBe(SITE);
    const tokens = await tokenRequest(
      db,
      env,
      SITE,
      {
        grant_type: 'authorization_code',
        code,
        code_verifier: opts.verifier ?? v,
        client_id: clientId,
        redirect_uri: REDIRECT,
        resource: mcpResource(SITE),
      },
      at,
    );
    return { clientId, code, v, tokens };
  }

  it('registers a public client: https or the loopback, nothing it cannot do', async () => {
    const ok = await registerClient(db, {
      client_name: 'Agent‮\u0007 X',
      redirect_uris: [REDIRECT],
      token_endpoint_auth_method: 'client_secret_basic',
    });
    expect(ok).toMatchObject({ client_name: 'Agent X', token_endpoint_auth_method: 'none', redirect_uris: [REDIRECT] });
    for (const bad of [[], ['http://evil.example/cb'], ['https://x.example/cb#frag'], ['javascript:alert(1)'], 'https://x.example'])
      expect(await registerClient(db, { redirect_uris: bad })).toMatchObject({ error: 'invalid_redirect_uri' });
    expect(await registerClient(db, { redirect_uris: [REDIRECT], grant_types: ['client_credentials'] })).toMatchObject({
      error: 'invalid_client_metadata',
    });
  });

  it('refuses before the client is known, then sends errors back: S256 required, the store’s resource alone', async () => {
    const clientId = await register();
    const base = {
      client_id: clientId,
      redirect_uri: REDIRECT,
      response_type: 'code',
      code_challenge: challengeOf(verifier()),
      code_challenge_method: 'S256',
      state: 's',
    };
    expect(await startAuthorization(db, { ...base, client_id: 'mcp_nobody' }, SITE)).toEqual({ fatal: 'invalid_client' });
    expect(await startAuthorization(db, { ...base, redirect_uri: 'http://127.0.0.1:1/other' }, SITE)).toEqual({
      fatal: 'invalid_redirect_uri',
    });
    const err = async (q: Record<string, unknown>) =>
      new URL(((await startAuthorization(db, q, SITE)) as { redirect: string }).redirect).searchParams;
    expect((await err({ ...base, code_challenge_method: 'plain' })).get('error')).toBe('invalid_request');
    expect((await err({ ...base, code_challenge: undefined })).get('error')).toBe('invalid_request');
    expect((await err({ ...base, resource: 'https://other.example/mcp' })).get('error')).toBe('invalid_target');
    expect((await err({ ...base, scope: 'author admin' })).get('error')).toBe('invalid_scope');
    expect((await err({ ...base, response_type: 'token' })).get('error')).toBe('unsupported_response_type');
  });

  it('gives a pair of tokens for the code and the right verifier, once', async () => {
    const { tokens, code, v, clientId } = await connect(author);
    expect(tokens).toMatchObject({ token_type: 'Bearer', expires_in: ACCESS_SECONDS, scope: 'author' });
    const t = tokens as TokenAnswer;
    expect(t.access_token).toMatch(/^esc_mcp_/);
    expect(t.refresh_token).toMatch(/^esc_mcpr_/);
    // The base keeps hashes only.
    const rows = (await pool.query('select id from oauth_tokens')).rows.map((r) => r.id);
    expect(rows).not.toContain(t.access_token);
    expect(rows).not.toContain(code);
    // The bearer acts for the author, with their scopes.
    const b = await bearerOf(db, env, SITE, `Bearer ${t.access_token}`, later(1000));
    expect(b).toMatchObject({ account: { id: author.id }, scopes: ['author'], client: { name: 'Claude Code (escouade-store)' } });
    // The code a second time: refused, and the grant it made revoked (someone else holds it).
    const again = await tokenRequest(db, env, SITE, { grant_type: 'authorization_code', code, code_verifier: v, client_id: clientId }, T0);
    expect(again).toMatchObject({ error: 'invalid_grant' });
    expect(await bearerOf(db, env, SITE, `Bearer ${t.access_token}`, later(2000))).toEqual({ error: 'invalid' });
  });

  it('refuses a wrong verifier, another client, an expired code', async () => {
    const bad = await connect(author, 'author', { verifier: verifier() });
    expect(bad.tokens).toMatchObject({ error: 'invalid_grant', error_description: 'code_verifier does not match the challenge' });

    const clientId = await register();
    const v = verifier();
    const started = (await startAuthorization(
      db,
      { client_id: clientId, redirect_uri: REDIRECT, response_type: 'code', code_challenge: challengeOf(v), code_challenge_method: 'S256' },
      SITE,
      T0,
    )) as { request: string };
    const code = new URL((await answerRequest(db, env, SITE, author, started.request, true, T0))!.redirect).searchParams.get('code')!;
    const other = await register();
    expect(
      await tokenRequest(db, env, SITE, { grant_type: 'authorization_code', code, code_verifier: v, client_id: other }, T0),
    ).toMatchObject({
      error: 'invalid_grant',
    });

    const started2 = (await startAuthorization(
      db,
      { client_id: clientId, redirect_uri: REDIRECT, response_type: 'code', code_challenge: challengeOf(v), code_challenge_method: 'S256' },
      SITE,
      T0,
    )) as { request: string };
    const code2 = new URL((await answerRequest(db, env, SITE, author, started2.request, true, T0))!.redirect).searchParams.get('code')!;
    expect(
      await tokenRequest(
        db,
        env,
        SITE,
        { grant_type: 'authorization_code', code: code2, code_verifier: v, client_id: clientId },
        later(11 * 60_000),
      ),
    ).toMatchObject({ error: 'invalid_grant', error_description: 'the code is expired' });
  });

  it('expires the access token after an hour; the refresh token is swapped, and one used twice revokes the grant', async () => {
    const { tokens, clientId } = await connect(author);
    const t = tokens as TokenAnswer;
    expect(await bearerOf(db, env, SITE, `Bearer ${t.access_token}`, later(ACCESS_SECONDS * 1000 + 1))).toEqual({ error: 'invalid' });
    const fresh = (await tokenRequest(
      db,
      env,
      SITE,
      { grant_type: 'refresh_token', refresh_token: t.refresh_token, client_id: clientId },
      later(3700_000),
    )) as TokenAnswer;
    expect(fresh.refresh_token).not.toBe(t.refresh_token);
    expect(await bearerOf(db, env, SITE, `Bearer ${fresh.access_token}`, later(3800_000))).toMatchObject({ account: { id: author.id } });
    // The old refresh token again: someone copied it. Everything of the grant stops.
    expect(
      await tokenRequest(
        db,
        env,
        SITE,
        { grant_type: 'refresh_token', refresh_token: t.refresh_token, client_id: clientId },
        later(3900_000),
      ),
    ).toMatchObject({
      error: 'invalid_grant',
    });
    expect(await bearerOf(db, env, SITE, `Bearer ${fresh.access_token}`, later(3950_000))).toEqual({ error: 'invalid' });
    expect(
      await tokenRequest(
        db,
        env,
        SITE,
        { grant_type: 'refresh_token', refresh_token: fresh.refresh_token, client_id: clientId },
        later(4000_000),
      ),
    ).toMatchObject({
      error: 'invalid_grant',
    });
  });

  it('refuses a refresh token past its 30 days, and a token for another resource', async () => {
    const { tokens, clientId } = await connect(author);
    const t = tokens as TokenAnswer;
    expect(
      await tokenRequest(
        db,
        env,
        SITE,
        { grant_type: 'refresh_token', refresh_token: t.refresh_token, client_id: clientId },
        later(REFRESH_DAYS * 86_400_000 + 1),
      ),
    ).toMatchObject({ error: 'invalid_grant' });
    // A grant made for another resource (another store's address): its tokens do not open this one.
    const other = await connect(author);
    await pool.query("update oauth_grants set resource = 'https://other.example/mcp' where id = (select max(id) from oauth_grants)");
    expect(await bearerOf(db, env, SITE, `Bearer ${(other.tokens as TokenAnswer).access_token}`, T0)).toEqual({ error: 'invalid' });
    expect(
      await tokenRequest(db, env, SITE, { grant_type: 'refresh_token', refresh_token: 'x', resource: 'https://other.example/mcp' }),
    ).toMatchObject({
      error: 'invalid_target',
    });
  });

  it('gives `review` to an administrator alone, and takes it back when they leave STORE_ADMINS', async () => {
    const clientId = await register();
    const started = (await startAuthorization(
      db,
      {
        client_id: clientId,
        redirect_uri: REDIRECT,
        response_type: 'code',
        code_challenge: challengeOf(verifier()),
        code_challenge_method: 'S256',
        scope: 'author review',
      },
      SITE,
      T0,
    )) as { request: string };
    expect(await requestView(db, env, author, started.request, T0)).toMatchObject({
      asked: ['author', 'review'],
      granted: ['author'],
      refused: ['review'],
    });
    const mine = await connect(author, 'author review');
    expect(mine.tokens).toMatchObject({ scope: 'author' });
    const onlyReview = await register();
    const s2 = (await startAuthorization(
      db,
      {
        client_id: onlyReview,
        redirect_uri: REDIRECT,
        response_type: 'code',
        code_challenge: challengeOf(verifier()),
        code_challenge_method: 'S256',
        scope: 'review',
      },
      SITE,
      T0,
    )) as { request: string };
    expect(new URL((await answerRequest(db, env, SITE, author, s2.request, true, T0))!.redirect).searchParams.get('error')).toBe(
      'invalid_scope',
    );

    const theirs = await connect(admin, 'author review');
    expect(theirs.tokens).toMatchObject({ scope: 'author review' });
    const token = `Bearer ${(theirs.tokens as TokenAnswer).access_token}`;
    expect(await bearerOf(db, env, SITE, token, T0)).toMatchObject({ scopes: ['author', 'review'] });
    expect(await bearerOf(db, {}, SITE, token, T0)).toMatchObject({ scopes: ['author'] });
  });

  it('says a refusal to the client, uses a request once', async () => {
    const clientId = await register();
    const started = (await startAuthorization(
      db,
      {
        client_id: clientId,
        redirect_uri: REDIRECT,
        response_type: 'code',
        code_challenge: challengeOf(verifier()),
        code_challenge_method: 'S256',
        state: 'k',
      },
      SITE,
      T0,
    )) as { request: string };
    const no = new URL((await answerRequest(db, env, SITE, author, started.request, false, T0))!.redirect);
    expect(no.searchParams.get('error')).toBe('access_denied');
    expect(no.searchParams.get('state')).toBe('k');
    expect(await answerRequest(db, env, SITE, author, started.request, true, T0)).toBeNull();
  });

  it('lists the connected apps with their actions, revokes one from the account and one by the client (RFC 7009)', async () => {
    const a = await connect(author);
    const b = await connect(author);
    const apps = await connectedApps(db, env, author, T0);
    expect(apps).toHaveLength(2);
    expect(apps[0]).toMatchObject({ client: 'Claude Code (escouade-store)', redirectHost: '127.0.0.1:33418', scopes: ['author'] });
    // Another account cannot revoke it.
    expect(await revokeApp(db, admin, apps[0]!.ref, T0)).toBe(false);
    expect(await revokeApp(db, author, apps[0]!.ref, T0)).toBe(true);
    expect(await bearerOf(db, env, SITE, `Bearer ${(b.tokens as TokenAnswer).access_token}`, T0)).toEqual({ error: 'invalid' });
    await revokeByToken(db, (a.tokens as TokenAnswer).refresh_token, T0);
    expect(await bearerOf(db, env, SITE, `Bearer ${(a.tokens as TokenAnswer).access_token}`, T0)).toEqual({ error: 'invalid' });
    expect(await connectedApps(db, env, author, T0)).toEqual([]);
    const journal = (
      await pool.query("select action from audit where account_id = $1 and action like 'oauth.%' order by id", [author.id])
    ).rows.map((r) => r.action);
    expect(journal).toEqual(['oauth.granted', 'oauth.token', 'oauth.granted', 'oauth.token', 'oauth.revoked', 'oauth.revoked']);
  });

  it('loses every token with the account, and purges what has outlived its time', async () => {
    const { tokens } = await connect(author);
    await deleteAccount(db, author, SECRET, T0);
    expect(await bearerOf(db, env, SITE, `Bearer ${(tokens as TokenAnswer).access_token}`, T0)).toEqual({ error: 'invalid' });
    expect((await pool.query('select count(*)::int as n from oauth_grants')).rows[0].n).toBe(0);
    await connect(admin);
    const purged = await purgeOAuth(db, later(REFRESH_DAYS * 86_400_000 + 2 * 86_400_000));
    expect(purged.grants).toBeGreaterThanOrEqual(1);
    expect((await pool.query('select count(*)::int as n from oauth_clients')).rows[0].n).toBe(0);
  });
});
