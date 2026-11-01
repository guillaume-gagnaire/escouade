import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { UnauthorizedError, type OAuthClientProvider } from '@modelcontextprotocol/sdk/client/auth.js';
import { StreamableHTTPClientTransport } from '@modelcontextprotocol/sdk/client/streamableHttp.js';
import type { OAuthClientInformationMixed, OAuthClientMetadata, OAuthTokens } from '@modelcontextprotocol/sdk/shared/auth.js';
import { expect, test, type Page } from '@playwright/test';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import pg from 'pg';
import { E2E_ADMIN_LOGIN, E2E_CHEF_LOGIN, E2E_FIRST_PARTY_REPO, FAKE_GITHUB, SITE } from '../fake/env';

// The store's MCP server (ESC-317, docs/STORE.md §2.5) with a real MCP client (@modelcontextprotocol/sdk) against the
// built server and the fake GitHub: the metadata (RFC 9728, RFC 8414), the registration (RFC 7591), PKCE S256, the
// sign-in with GitHub and the consent in the browser, the tools of an author on a folder of the app's repository
// (plugins/ntfy, tag plugins/ntfy/v1.1.0: the eleven checks, publishing, yanking, the list, a new tag waiting for a click),
// the token expired then refreshed and swapped, « Applications connectées » and its revocation, the scopes (`review` to
// an administrator alone). They need a database (DATABASE_URL).
test.skip(!process.env.DATABASE_URL, 'the MCP server needs DATABASE_URL');
test.describe.configure({ mode: 'serial' });
// The consent page follows the browser's language (or the client's ui_locales): French here.
test.use({ locale: 'fr-FR' });

// The client's own address, where the browser brings the code back: a server of the test, on a port the system gives.
let CALLBACK = '';
let lastCode = '';
const back = createServer((req, res) => {
  lastCode = new URL(req.url ?? '/', 'http://127.0.0.1').searchParams.get('code') ?? '';
  res.writeHead(200, { 'content-type': 'text/plain' });
  res.end('back to the client');
});
const MCP = new URL(`${SITE}/mcp`);

/** A client's memory of its registration, tokens and verifier, as Claude Code keeps them; the browser is the page's. */
class Provider implements OAuthClientProvider {
  info?: OAuthClientInformationMixed;
  saved?: OAuthTokens;
  verifier = '';
  authUrl?: URL;
  constructor(private readonly scope = 'author') {}
  get redirectUrl() {
    return CALLBACK;
  }
  get clientMetadata(): OAuthClientMetadata {
    return {
      client_name: 'E2E agent',
      redirect_uris: [CALLBACK],
      grant_types: ['authorization_code', 'refresh_token'],
      response_types: ['code'],
      token_endpoint_auth_method: 'none',
      scope: this.scope,
    };
  }
  clientInformation() {
    return this.info;
  }
  saveClientInformation(i: OAuthClientInformationMixed) {
    this.info = i;
  }
  tokens() {
    return this.saved;
  }
  saveTokens(t: OAuthTokens) {
    this.saved = t;
  }
  redirectToAuthorization(url: URL) {
    this.authUrl = url;
  }
  saveCodeVerifier(v: string) {
    this.verifier = v;
  }
  codeVerifier() {
    return this.verifier;
  }
}

/** Signs `login` in on the fake GitHub from /submit; with `repos`, installs the App on them. */
async function signIn(page: Page, login: string, repos?: string) {
  await page.goto('/submit');
  await page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
  await expect(page).toHaveURL(/\/submit$/);
  if (!repos) return;
  await page.getByRole('link', { name: 'Installer l’App sur un dépôt' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByLabel('Repositories').fill(repos);
  await page.getByRole('button', { name: 'Install & Authorize' }).click();
  await expect(page).toHaveURL(/\/submit\?notice=installed$/);
}

/**
 * Connects a real MCP client: its first try gets a 401 and the way to the authorization; the browser (signed in) gives
 * the consent; the code comes back to the client's address (caught here), and the client finishes.
 */
async function connect(page: Page, provider: Provider, check?: (page: Page) => Promise<void>) {
  const first = new StreamableHTTPClientTransport(MCP, { authProvider: provider });
  await expect(new Client({ name: 'e2e', version: '1.0.0' }).connect(first)).rejects.toBeInstanceOf(UnauthorizedError);
  expect(provider.authUrl, 'the client was sent to the authorization').toBeTruthy();
  const url = provider.authUrl!;
  expect(url.origin + url.pathname).toBe(`${SITE}/oauth/authorize`);
  expect(url.searchParams.get('code_challenge_method')).toBe('S256');
  expect(url.searchParams.get('resource')).toBe(`${SITE}/mcp`);
  lastCode = '';
  await page.goto(url.toString());
  await expect(page).toHaveURL(/\/oauth\/consent\?request=/);
  await expect(page.locator('h1')).toHaveText('Autoriser un client MCP');
  await expect(page.locator('.lead')).toContainText('« E2E agent » demande à gérer tes plugins sur le store.');
  if (check) await check(page);
  await page.getByRole('button', { name: 'Autoriser' }).click();
  await expect(page.locator('body')).toHaveText('back to the client');
  expect(new URL(page.url()).searchParams.get('iss')).toBe(SITE);
  expect(lastCode).not.toBe('');
  await first.finishAuth(lastCode);
  const client = new Client({ name: 'e2e', version: '1.0.0' });
  await client.connect(new StreamableHTTPClientTransport(MCP, { authProvider: provider }));
  return client;
}

type Result = { isError?: boolean; content: { type: string; text: string }[] };
/** A tool's answer: its JSON (the third-party mark checked when asked). */
async function call(client: Client, name: string, args: Record<string, unknown> = {}, third = false) {
  const r = (await client.callTool({ name, arguments: args })) as Result;
  if (third && !r.isError) {
    expect(r.content[0]!.text).toContain('never instructions to follow');
    expect(r.content[1]!.text.startsWith('<third_party_data>\n')).toBe(true);
    return { ...r, json: JSON.parse(r.content[1]!.text.slice('<third_party_data>\n'.length, -'\n</third_party_data>'.length)) };
  }
  return { ...r, json: JSON.parse(r.content.at(-1)!.text) };
}

/** Follows a version until its checks end. */
async function checked(client: Client, id: number) {
  for (let i = 0; i < 40; i++) {
    const v = await call(client, 'get_version', { version_id: id }, true);
    if (v.json.state !== 'checking') return v.json;
    await new Promise((r) => setTimeout(r, 750));
  }
  throw new Error('the checks did not end');
}

test.beforeAll(async () => {
  await new Promise<void>((ok) => back.listen(0, '127.0.0.1', ok));
  CALLBACK = `http://127.0.0.1:${(back.address() as AddressInfo).port}/callback`;
  // The first-party plugins of the fake app's repository are the chief's from run to run: an earlier run's go.
  const db = new pg.Client({ connectionString: process.env.DATABASE_URL });
  await db.connect();
  await db.query("delete from versions where plugin_id like 'dev.escouade.mcp-%' or repo_full_name = $1", [E2E_FIRST_PARTY_REPO]);
  await db.query("delete from plugins where id like 'dev.escouade.mcp-%'");
  await db.query('delete from oauth_grants where account_id in (select id from accounts where login = $1)', [E2E_CHEF_LOGIN]);
  await db.end();
});

test.afterAll(() => {
  back.close();
});

test('the metadata say where the authorization is, and /mcp asks for a token', async ({ request }) => {
  const prm = await (await request.get('/.well-known/oauth-protected-resource/mcp')).json();
  expect(prm).toMatchObject({ resource: `${SITE}/mcp`, authorization_servers: [SITE], scopes_supported: ['author', 'review'] });
  const as = await (await request.get('/.well-known/oauth-authorization-server')).json();
  expect(as).toMatchObject({
    issuer: SITE,
    authorization_endpoint: `${SITE}/oauth/authorize`,
    token_endpoint: `${SITE}/oauth/token`,
    registration_endpoint: `${SITE}/oauth/register`,
    code_challenge_methods_supported: ['S256'],
    token_endpoint_auth_methods_supported: ['none'],
  });
  const none = await request.post('/mcp', { data: { jsonrpc: '2.0', id: 1, method: 'tools/list' } });
  expect(none.status()).toBe(401);
  expect(none.headers()['www-authenticate']).toBe(`Bearer resource_metadata="${SITE}/.well-known/oauth-protected-resource/mcp"`);
  const bad = await request.post('/mcp', { data: {}, headers: { authorization: 'Bearer esc_mcp_forged' } });
  expect(bad.status()).toBe(401);
  expect(bad.headers()['www-authenticate']).toContain('error="invalid_token"');
  // An authorization for an unknown client is told on the store, never sent anywhere.
  const unknown = await request.get('/oauth/authorize?client_id=mcp_nobody&redirect_uri=https://evil.example/cb', { maxRedirects: 0 });
  expect(unknown.status()).toBe(400);
});

test('the chief submits, publishes and manages a folder of the app’s repository from an agent', async ({ page, request }) => {
  test.setTimeout(180_000);
  await signIn(page, E2E_CHEF_LOGIN, E2E_FIRST_PARTY_REPO.split('/')[1]);
  const provider = new Provider();
  const client = await connect(page, provider, async (p) => {
    await expect(p.locator('.who')).toContainText(`Pour le compte ${E2E_CHEF_LOGIN}`);
    await expect(p.locator('.scope h2')).toHaveText(['Gérer tes plugins']);
    await expect(p.locator('.never')).toContainText('ni signer, ni révoquer, ni toucher à une clé');
  });

  const tools = (await client.listTools()).tools.map((t) => t.name).sort();
  expect(tools).toEqual(
    [
      'change_source',
      'dismiss_version',
      'get_plugin',
      'get_version',
      'list_folders',
      'list_plugins',
      'list_repositories',
      'list_tags',
      'publish_version',
      'retry_checks',
      'set_auto_publish',
      'set_listed',
      'submit_version',
      'yank_version',
    ].sort(),
  );

  // The repositories, the folders and the tags of the folder (the app's own v2.1.0 is no plugin's).
  const repos = await call(client, 'list_repositories', {}, true);
  expect(repos.json.repositories).toContainEqual({ repository: E2E_FIRST_PARTY_REPO, private: false, suspended: false });
  const folders = await call(client, 'list_folders', { repository: E2E_FIRST_PARTY_REPO }, true);
  expect(folders.json.folders).toEqual(expect.arrayContaining(['plugins/ntfy', 'plugins/slack']));
  const tagged = await request.post(`${FAKE_GITHUB}/_fake/tag`, { data: { repo: E2E_FIRST_PARTY_REPO, tag: 'plugins/ntfy/v1.1.0' } });
  expect(tagged.ok()).toBe(true);
  const tags = await call(client, 'list_tags', { repository: E2E_FIRST_PARTY_REPO, path: 'plugins/ntfy' }, true);
  expect(tags.json.tags.map((t: { name: string }) => t.name)).toEqual(['plugins/ntfy/v1.1.0', 'plugins/ntfy/v1.0.0']);
  expect(tags.json.others).toBeGreaterThanOrEqual(2);

  // Inputs refused as the routes refuse them.
  const climbing = await call(client, 'submit_version', { repository: E2E_FIRST_PARTY_REPO, path: '../etc', tag: 'plugins/ntfy/v1.1.0' });
  expect(climbing.isError).toBe(true);
  expect(climbing.json.error).toBe('path-parent');
  const notMine = await call(client, 'submit_version', { repository: 'someone/else', tag: 'v1.0.0' });
  expect(notMine.json.error).toBe('not-yours');
  const malformed = (await client.callTool({ name: 'submit_version', arguments: { repository: 'no slash', tag: 'v1' } })) as Result;
  expect(malformed.isError).toBe(true);

  // The submission and its eleven checks, then publishing.
  const submitted = await call(client, 'submit_version', {
    repository: E2E_FIRST_PARTY_REPO,
    path: 'plugins/ntfy',
    tag: 'plugins/ntfy/v1.1.0',
  });
  expect(submitted.isError).toBeFalsy();
  const id = submitted.json.version_id as number;
  const v = await checked(client, id);
  expect(v.state).toBe('ready');
  expect(v.pluginId).toBe('dev.escouade.mcp-ntfy');
  expect(v.version).toBe('1.1.0');
  expect(v.path).toBe('plugins/ntfy');
  expect(v.checks).toHaveLength(11);
  expect(v.checks.every((c: { status: string }) => c.status === 'pass')).toBe(true);
  const noCategory = await call(client, 'publish_version', { version_id: id });
  expect(noCategory.json.error).toBe('categories');
  const published = await call(client, 'publish_version', { version_id: id, categories: ['notifications'], request_review: true });
  expect(published.json).toEqual({ ok: true, version_id: id });

  const listed = await call(client, 'list_plugins', {}, true);
  expect(listed.json.plugins).toContainEqual(expect.objectContaining({ id: 'dev.escouade.mcp-ntfy', version: '1.1.0' }));
  const detail = await call(client, 'get_plugin', { plugin_id: 'dev.escouade.mcp-ntfy' }, true);
  expect(detail.json.source).toMatchObject({ repository: E2E_FIRST_PARTY_REPO, path: 'plugins/ntfy' });
  expect(detail.json.versions[0]).toMatchObject({ version: '1.1.0', state: 'published', review: true, yankedAt: null });

  // Nothing secret in what the tools gave: no GitHub token, no secret, no internal address.
  const everything = JSON.stringify([repos, folders, tags, v, listed, detail]);
  expect(everything).not.toMatch(
    /ghs_|ghu_|gho_|postgres:|DATABASE_URL|NUXT_SESSION_SECRET|installationId|\.data\/archives|BEGIN [A-Z ]*PRIVATE KEY/,
  );

  // A new tag waits for a click (no auto publish), is set aside; the list, then the auto publish.
  await request.post(`${FAKE_GITHUB}/_fake/tag`, { data: { repo: E2E_FIRST_PARTY_REPO, tag: 'plugins/ntfy/v1.2.0' } });
  let waiting: { id: number; version: string } | undefined;
  for (let i = 0; i < 40 && !waiting; i++) {
    waiting = (await call(client, 'list_plugins', {}, true)).json.pending.find((p: { version: string }) => p.version === '1.2.0');
    if (!waiting) await new Promise((r) => setTimeout(r, 750));
  }
  expect(waiting, 'the new tag waits for a click').toBeTruthy();
  expect((await call(client, 'dismiss_version', { version_id: waiting!.id })).json.ok).toBe(true);
  expect((await call(client, 'set_auto_publish', { plugin_id: 'dev.escouade.mcp-ntfy', on: true })).json.auto_publish).toBe(true);
  expect((await call(client, 'set_listed', { plugin_id: 'dev.escouade.mcp-ntfy', listed: false })).json.listed).toBe(false);
  expect((await call(client, 'get_plugin', { plugin_id: 'dev.escouade.mcp-ntfy' }, true)).json.unlistedAt).not.toBeNull();
  expect((await call(client, 'set_listed', { plugin_id: 'dev.escouade.mcp-ntfy', listed: true })).json.listed).toBe(true);
  expect(
    (
      await call(client, 'change_source', {
        plugin_id: 'dev.escouade.mcp-ntfy',
        repository: E2E_FIRST_PARTY_REPO,
        path: 'plugins/ntfy',
        tag: 'x',
      })
    ).json.error,
  ).toBe('same-source');
  expect((await call(client, 'retry_checks', { version_id: id })).json.error).toBe('not-retriable');

  // Moving the plugin: a folder whose manifest names another plugin is refused; its new folder, then published (to
  // the review whatever was asked, docs/STORE.md §3.6).
  const other = await call(client, 'change_source', {
    plugin_id: 'dev.escouade.mcp-ntfy',
    repository: E2E_FIRST_PARTY_REPO,
    path: 'plugins/slack',
    tag: 'plugins/slack/v1.0.0',
  });
  expect(other.json).toEqual({ error: 'id-differs', found: 'dev.escouade.mcp-slack' });
  await request.post(`${FAKE_GITHUB}/_fake/tag`, { data: { repo: E2E_FIRST_PARTY_REPO, tag: 'plugins/ntfy-moved/v1.3.0' } });
  const moving = await call(client, 'change_source', {
    plugin_id: 'dev.escouade.mcp-ntfy',
    repository: E2E_FIRST_PARTY_REPO,
    path: 'plugins/ntfy-moved',
    tag: 'plugins/ntfy-moved/v1.3.0',
  });
  expect(moving.json).toMatchObject({ from: `${E2E_FIRST_PARTY_REPO}/plugins/ntfy`, to: `${E2E_FIRST_PARTY_REPO}/plugins/ntfy-moved` });
  const moved = await checked(client, moving.json.version_id);
  expect(moved).toMatchObject({ state: 'ready', pluginId: 'dev.escouade.mcp-ntfy', version: '1.3.0', path: 'plugins/ntfy-moved' });
  expect((await call(client, 'publish_version', { version_id: moving.json.version_id, request_review: false })).json.ok).toBe(true);
  const movedPlugin = (await call(client, 'get_plugin', { plugin_id: 'dev.escouade.mcp-ntfy' }, true)).json;
  expect(movedPlugin.source).toMatchObject({ path: 'plugins/ntfy-moved', from: `${E2E_FIRST_PARTY_REPO}/plugins/ntfy` });
  expect(movedPlugin.versions[0]).toMatchObject({ version: '1.3.0', review: true });

  // A second folder of the app's repository, published for the review (read by the administrator below).
  await request.post(`${FAKE_GITHUB}/_fake/tag`, { data: { repo: E2E_FIRST_PARTY_REPO, tag: 'plugins/slack/v1.1.0' } });
  const slack = await call(client, 'submit_version', {
    repository: E2E_FIRST_PARTY_REPO,
    path: 'plugins/slack',
    tag: 'plugins/slack/v1.1.0',
  });
  expect((await checked(client, slack.json.version_id)).state).toBe('ready');
  expect((await call(client, 'publish_version', { version_id: slack.json.version_id, categories: ['notifications'] })).json.ok).toBe(true);

  // The access token expired: the client refreshes it by itself, the refresh token is swapped.
  const before = provider.saved!;
  const db = new pg.Client({ connectionString: process.env.DATABASE_URL });
  await db.connect();
  await db.query("update oauth_tokens set expires_at = now() - interval '1 second' where kind = 'access'");
  await db.end();
  const yanked = await call(client, 'yank_version', { version_id: id });
  expect(yanked.json).toEqual({ ok: true, version_id: id });
  expect(provider.saved!.refresh_token).not.toBe(before.refresh_token);
  expect(provider.saved!.access_token).not.toBe(before.access_token);
  const yankedNow = (await call(client, 'get_plugin', { plugin_id: 'dev.escouade.mcp-ntfy' }, true)).json.versions;
  expect(yankedNow.find((x: { version: string }) => x.version === '1.1.0').yankedAt).not.toBeNull();
  // The old refresh token again, by someone who copied it: refused, and the whole grant revoked.
  const reuse = await request.post('/oauth/token', {
    form: { grant_type: 'refresh_token', refresh_token: before.refresh_token!, client_id: provider.info!.client_id },
  });
  expect(reuse.status()).toBe(400);
  expect((await reuse.json()).error).toBe('invalid_grant');
  const after = await request.post('/mcp', {
    data: { jsonrpc: '2.0', id: 1, method: 'tools/list' },
    headers: { authorization: `Bearer ${provider.saved!.access_token}`, accept: 'application/json, text/event-stream' },
  });
  expect(after.status()).toBe(401);

  // « Applications connectées »: a new connection, its actions, then « Déconnecter ».
  const second = new Provider();
  const again = await connect(page, second);
  await call(again, 'list_plugins', {}, true);
  await page.goto('/account');
  const apps = page.locator('#apps');
  await expect(apps.locator('h3')).toHaveText('Applications connectées');
  const line = apps.locator('.lines > li').first();
  await expect(line).toContainText('E2E agent');
  await expect(line).toContainText(new URL(CALLBACK).host);
  await line.locator('summary').click();
  await expect(line.locator('.acts')).toContainText('list_plugins');
  await line.getByRole('button', { name: 'Déconnecter' }).click();
  await expect(page).toHaveURL(/\/account\?notice=app-revoked#apps$/);
  await expect(page.locator('.notice.ok').first()).toContainText('Application déconnectée');
  const revoked = await request.post('/mcp', {
    data: { jsonrpc: '2.0', id: 1, method: 'tools/list' },
    headers: { authorization: `Bearer ${second.saved!.access_token}`, accept: 'application/json, text/event-stream' },
  });
  expect(revoked.status()).toBe(401);
  await expect(page.locator('#apps')).toContainText('Aucun client MCP connecté.');
});

test('the review scope: an author does not get it, an administrator reads the queue and posts a report', async ({ browser }) => {
  test.setTimeout(120_000);
  // An author who asks for `review` is told they will not get it, and gets `author` alone.
  const authorPage = await browser.newPage();
  await signIn(authorPage, `mcp-author-${Date.now().toString(36).slice(-5)}`);
  const asking = new Provider('author review');
  const theirs = await connect(authorPage, asking, async (p) => {
    await expect(p.locator('.notice.warn')).toContainText('ce compte n’administre pas le store');
  });
  expect(asking.saved!.scope).toBe('author');
  const names = (await theirs.listTools()).tools.map((t) => t.name);
  expect(names).not.toContain('review_queue');
  const refused = (await theirs.callTool({ name: 'review_queue', arguments: {} })) as Result;
  expect(refused.isError).toBe(true);
  await authorPage.close();

  // The administrator asks for it apart, and gets it.
  const adminPage = await browser.newPage();
  await signIn(adminPage, E2E_ADMIN_LOGIN);
  const provider = new Provider('author review');
  const admin = await connect(adminPage, provider, async (p) => {
    await expect(p.locator('.scope h2')).toHaveText(['Gérer tes plugins', 'La file de revue (administrateur)']);
  });
  expect(provider.saved!.scope).toBe('author review');
  const tools = (await admin.listTools()).tools.map((t) => t.name);
  expect(tools).toEqual(expect.arrayContaining(['review_queue', 'review_version', 'post_report']));
  expect(tools).not.toEqual(expect.arrayContaining(['sign', 'revoke']));
  const queue = await call(admin, 'review_queue', {}, true);
  expect(queue.json.counts).toBeTruthy();
  // The chief's version, published with « Demander la revue » then yanked, is no longer to review; a report on it is
  // refused as `report` refuses an unknown version, and a report on nothing published is refused.
  const none = await call(admin, 'post_report', { plugin_id: 'dev.escouade.nothing', version: '1.0.0', report: '# Rien' });
  expect(none.json.error).toBe('not-found');
  const posted = await call(admin, 'post_report', {
    plugin_id: 'dev.escouade.mcp-ntfy',
    version: '1.1.0',
    verdict: 'Vérifier',
    report: '# Relu\n\nRien à redire.',
  });
  expect(posted.json).toEqual({ ok: true });
  // The slack plugin, published with « Demander la revue »: in the queue, its report posted then read back.
  expect(queue.json.toReview.map((x: { id: string }) => x.id)).toContain('dev.escouade.mcp-slack');
  const report = '# Relu\n\nLe banc passe.';
  expect(
    (await call(admin, 'post_report', { plugin_id: 'dev.escouade.mcp-slack', version: '1.1.0', verdict: 'Vérifier', report })).json,
  ).toEqual({ ok: true });
  const read = await call(admin, 'review_version', { plugin_id: 'dev.escouade.mcp-slack', version: '1.1.0' }, true);
  expect(read.json.report).toMatchObject({ verdict: 'Vérifier', text: report });
  expect(read.json.source).toMatchObject({ repository: E2E_FIRST_PARTY_REPO, path: 'plugins/slack' });
  await adminPage.close();
});
