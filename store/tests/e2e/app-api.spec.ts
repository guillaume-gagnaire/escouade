import { createHash } from 'node:crypto';
import { expect, test, type APIRequestContext, type Page } from '@playwright/test';

// The app's API on the built server (docs/STORE.md §1.2, §5.4, §8.1): a plugin with binaries published through the
// fake GitHub, then read as the app reads it — the catalogue and its ETag, version.json whose SHA-256 is the
// catalogue's fingerprint, the archive and the binary whose SHA-256 are version.json's, all cached for a year; the
// counter of installations without any id, and the rate limits. They need a database (DATABASE_URL).
test.skip(!process.env.DATABASE_URL, 'the app’s API needs DATABASE_URL');

const RUN = Date.now().toString(36).slice(-5);
const sha = (b: Buffer | string) => createHash('sha256').update(b).digest('hex');
/** An address of the documentation range, its own for each test: the limits count by address (X-Forwarded-For). */
const from = () => ({
  'x-forwarded-for': `198.51.100.${Math.floor(Math.random() * 250) + 1}, 203.0.113.${Math.floor(Math.random() * 250) + 1}`,
});

/** Signs `login` in, installs the App on `repo`, submits its newest tag and publishes it; gives the plugin's id. */
async function publish(page: Page, login: string, repo: string): Promise<string> {
  await page.goto('/submit');
  await page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
  await page.getByRole('link', { name: 'Installer l’App sur un dépôt' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByLabel('Repositories').fill(repo);
  await page.getByRole('button', { name: 'Install & Authorize' }).click();
  await page.locator('.choices li', { hasText: repo }).getByRole('link', { name: 'Choisir' }).click();
  await page.getByRole('button', { name: 'Lancer les contrôles' }).click();
  await expect(page.locator('.check.run')).toHaveCount(0, { timeout: 20_000 });
  await expect(page.locator('.check.pass')).toHaveCount(10);
  await page.getByRole('link', { name: 'Continuer' }).click();
  await page.getByRole('button', { name: 'Publier, non vérifié' }).click();
  await expect(page).toHaveURL(/notice=published$/);
  const id = /io\.github\.[\w.-]+/.exec((await page.locator('main').textContent()) ?? '')?.[0];
  return id ?? `io.github.${login.toLowerCase()}.${repo}`;
}

async function catalog(request: APIRequestContext) {
  const res = await request.get('/api/v1/catalog.json', { headers: from() });
  expect(res.status()).toBe(200);
  return { res, body: await res.json() };
}

test('the app reads the catalogue, version.json, the archive and the binaries, each checked by its SHA-256', async ({ page, request }) => {
  const login = `ada-api-${RUN}`;
  const repo = `pager-${RUN}-bin`;
  await publish(page, login, repo);

  const { res, body } = await catalog(request);
  expect(res.headers()['cache-control']).toBe('public, no-cache');
  expect(res.headers()['content-type']).toContain('application/json');
  expect(body.format).toBe(1);
  const plugin = body.plugins.find((p: { repository: string }) => p.repository === `https://github.com/${login}/${repo}`);
  expect(plugin).toBeTruthy();
  expect(plugin).toMatchObject({ author: login, installs: 0, categories: expect.any(Array) });
  const v = plugin.versions.find((x: { version: string }) => x.version === plugin.latest);
  expect(v).toMatchObject({ verified: false, run: true, escouade: { api: '1' } });
  expect(v.platforms).toEqual(expect.arrayContaining(['linux-x64', 'windows-x64', 'macos-arm64']));

  // The ETag: the same bytes are not sent twice; compressed for a client that takes gzip.
  const etag = res.headers()['etag']!;
  expect(etag).toMatch(/^"[0-9a-f]{64}"$/);
  const again = await request.get('/api/v1/catalog.json', { headers: { ...from(), 'if-none-match': etag } });
  expect(again.status()).toBe(304);
  const gz = await request.get('/api/v1/catalog.json', { headers: { ...from(), 'accept-encoding': 'gzip' } });
  expect(gz.headers()['content-encoding']).toBe('gzip');
  expect(gz.headers()['vary']).toContain('accept-encoding');

  // version.json, byte for byte: its SHA-256 is the catalogue's fingerprint.
  const vj = await request.get(v.files.version_json, { headers: from() });
  expect(vj.status()).toBe(200);
  expect(vj.headers()['cache-control']).toBe('public, max-age=31536000, immutable');
  const bytes = await vj.body();
  expect(sha(bytes)).toBe(v.sha256);
  const file = JSON.parse(bytes.toString());
  expect(file).toMatchObject({ id: plugin.id, version: plugin.latest, archive: v.archive, escouade: v.escouade });

  // The archive and each binary: their SHA-256 are version.json's, cached for a year, never sent twice.
  const archive = await request.get(v.files.archive, { headers: from() });
  expect(archive.status()).toBe(200);
  expect(archive.headers()['content-type']).toBe('application/gzip');
  expect(archive.headers()['cache-control']).toBe('public, max-age=31536000, immutable');
  expect(sha(await archive.body())).toBe(file.archive.sha256);
  expect(Number(archive.headers()['content-length'])).toBe(file.archive.size);
  expect((await request.get(v.files.archive, { headers: { ...from(), 'if-none-match': `"${file.archive.sha256}"` } })).status()).toBe(304);
  for (const [platform, path] of Object.entries(v.files.binaries as Record<string, string>)) {
    const bin = await request.get(path, { headers: from() });
    expect(bin.status(), platform).toBe(200);
    expect(sha(await bin.body()), platform).toBe(file.binaries[platform].sha256);
  }
  const head = await request.head(v.files.archive, { headers: from() });
  expect(head.status()).toBe(200);
  expect(Number(head.headers()['content-length'])).toBe(file.archive.size);

  // What the store does not publish is not served.
  const base = `/api/v1/plugins/${encodeURIComponent(plugin.id)}`;
  expect((await request.get(`${base}/9.9.9/version.json`, { headers: from() })).status()).toBe(404);
  expect((await request.get(`${base}/9.9.9/archive.tar.gz`, { headers: from() })).status()).toBe(404);
  expect((await request.get(`${base}/${plugin.latest}/binaries/linux-riscv`, { headers: from() })).status()).toBe(404);
  expect((await request.get(`${base}/${plugin.latest}/binaries/__proto__`, { headers: from() })).status()).toBe(404);

  // The counter: JSON with the three keys, no cookie, no Origin; a version the store does not publish is refused.
  const count = (data: unknown, headers: Record<string, string> = {}) =>
    request.post('/api/v1/installs', { data: data as object, headers: { ...from(), ...headers } });
  expect((await count({ id: plugin.id, version: plugin.latest, event: 'install' })).status()).toBe(204);
  expect((await count({ id: plugin.id, version: plugin.latest, event: 'install', installId: 'x' })).status()).toBe(400);
  expect((await count({ id: plugin.id, version: '9.9.9', event: 'install' })).status()).toBe(404);
  const form = await request.post('/api/v1/installs', {
    headers: { ...from(), 'content-type': 'text/plain' },
    data: JSON.stringify({ id: plugin.id, version: plugin.latest, event: 'install' }),
  });
  expect(form.status()).toBe(415);
});

test('the counter and the API are rate limited by address, and say how long to wait', async ({ request }) => {
  const headers = from();
  let last = 0;
  for (let i = 0; i < 61; i++)
    last = (
      await request.post('/api/v1/installs', { headers, data: { id: 'dev.nobody.here', version: `1.0.${i}`, event: 'install' } })
    ).status();
  // 60 an hour from one address (refused as unknown versions, but counted against the address), then 429.
  expect(last).toBe(429);
  const refused = await request.post('/api/v1/installs', { headers, data: { id: 'dev.nobody.here', version: '1.0.0', event: 'install' } });
  expect(refused.status()).toBe(429);
  expect(Number(refused.headers()['retry-after'])).toBeGreaterThan(0);
  // Another address is not held back.
  expect(
    (
      await request.post('/api/v1/installs', { headers: from(), data: { id: 'dev.nobody.here', version: '1.0.0', event: 'install' } })
    ).status(),
  ).toBe(404);
});

test('the revocation list is not there before the first one is signed and taken in', async ({ request }) => {
  const res = await request.get('/api/v1/revocations.json', { headers: from() });
  expect(res.status()).toBe(404);
  expect((await request.get('/api/v1/revocations.json.minisig', { headers: from() })).status()).toBe(404);
});
