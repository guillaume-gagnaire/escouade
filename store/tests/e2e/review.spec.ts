import { readFileSync } from 'node:fs';
import pg from 'pg';
import { expect, test, type Page } from '@playwright/test';
import { seedCatalog } from '../fake/catalog.mjs';
import { E2E_ADMIN_LOGIN, e2eKey, SITE } from '../fake/env';
import { signWithSecret } from '../fake/minisign.mjs';

// The review queue (STORE-8, docs/STORE.md §4.1, §4.3, §4.5, §4.6) on the built server: /review for the
// administrator alone (STORE_ADMINS, set by playwright.config.ts), its tabs, a version's detail with the command to
// copy and what grew; the administration token made in the account and shown once, the report and the signature sent
// with it as the script sends them, the seal on the plugin's page; changes asked from the page; a report closed; the
// page compared to the preview (design/store/index.html, #review) in both themes. The signature is made with a key of
// the run, in the system's temporary folder (tests/fake/env.ts), never the store's.
test.skip(!process.env.DATABASE_URL, 'the review queue needs DATABASE_URL');
test.describe.configure({ mode: 'serial' });

const PREVIEW = new URL('../../../design/store/index.html', import.meta.url).href;
let seeded: Awaited<ReturnType<typeof seedCatalog>>;
test.beforeAll(async () => {
  seeded = await seedCatalog();
});

async function signIn(page: Page, login: string, to = '/review') {
  await page.goto(to);
  await page.locator('main').getByRole('link', { name: 'Se connecter avec GitHub' }).click();
  await page.getByLabel('Login').fill(login);
  await page.getByRole('button', { name: 'Authorize Escouade Store' }).click();
}

/** The queue's row of `id@version`, by its link. */
const row = (page: Page, name: string, version: string) =>
  page
    .locator('table.t tr')
    .filter({ has: page.locator('.pl b', { hasText: name }) })
    .filter({ has: page.locator('.pl .mono', { hasText: new RegExp(`^v${version.replaceAll('.', '\\.')}$`) }) });

test('the queue is the administrator’s alone', async ({ page, request }) => {
  await page.goto('/review');
  await expect(page.locator('main')).toContainText('La file de revue demande d’être connecté.');
  await signIn(page, `just-an-author-${seeded.run}`);
  await expect(page).toHaveURL(/\/review$/);
  await expect(page.locator('main')).toContainText('Cette page n’existe pas.');
  await expect(page.locator('header')).not.toContainText('File de revue');
  expect((await request.get('/api/v1/review/queue')).status()).toBe(401);
  expect((await request.get('/api/v1/review/queue', { headers: { authorization: 'Bearer esc_admin_nope' } })).status()).toBe(401);
  expect((await request.post('/api/review/tokens', { data: { label: 'x' }, headers: { origin: SITE } })).status()).toBe(401);
});

test('the administrator reads the queue, makes a token, gets the report and the signature, and the seal shows', async ({ page }) => {
  await signIn(page, E2E_ADMIN_LOGIN);
  await expect(page).toHaveURL(/\/review$/);
  await expect(page.locator('header')).toContainText('File de revue');
  await expect(page.locator('h1')).toContainText('File de revue');
  const name = `Slack ${seeded.run}`;
  await expect(row(page, name, '1.2.0')).toHaveCount(1);
  await expect(row(page, name, '1.1.0')).toHaveCount(1);
  await row(page, name, '1.2.0').locator('a.pl').click();
  await expect(page).toHaveURL(/\/review\?v=\d+$/);
  const detail = page.locator('aside.detail');
  await expect(detail.locator('.cmd').first()).toContainText(`npm run plugin-review -- ${seeded.ids.slack}@1.2.0`);
  await expect(detail).toContainText('Ce qui a changé');
  await expect(detail).toContainText('Pas encore reçu');
  await expect(detail.getByRole('button', { name: 'Vérifier' })).toHaveCount(0);

  // The token: made in the account, shown once, never again.
  await page.goto('/account#tokens');
  await page.locator('#token-label').fill(`Poste ${seeded.run}`);
  await page.getByRole('button', { name: 'Créer un jeton' }).click();
  const token = (await page.locator('#tokens .made code').textContent())!.trim();
  expect(token).toMatch(/^esc_admin_/);
  await page.reload();
  await expect(page.locator('#tokens')).toContainText(`Poste ${seeded.run}`);
  await expect(page.locator('#tokens')).not.toContainText(token);
  const auth = { authorization: `Bearer ${token}` };

  // The script's report, then its signature (made with the run's key, as minisign would).
  const report = await page.request.post('/api/v1/review/report', {
    headers: auth,
    data: {
      id: seeded.ids.slack,
      version: '1.2.0',
      verdict: 'Vérifier',
      report: `# Revue de ${seeded.ids.slack}@1.2.0\n\nRien à redire.\n`,
    },
  });
  expect(report.status()).toBe(200);
  const json = await (await page.request.get(`/api/v1/plugins/${seeded.ids.slack}/1.2.0/version.json`)).text();
  const secret = readFileSync(e2eKey().secret, 'utf8');
  const other = await page.request.post('/api/v1/review/signature', {
    headers: auth,
    data: {
      id: seeded.ids.slack,
      version: '1.1.0',
      minisig: signWithSecret(secret, json, `escouade-verified-v1 ${seeded.ids.slack}@1.2.0 2026-10-29`),
    },
  });
  expect(other.status()).toBe(422);
  const signed = await page.request.post('/api/v1/review/signature', {
    headers: auth,
    data: {
      id: seeded.ids.slack,
      version: '1.2.0',
      minisig: signWithSecret(secret, json, `escouade-verified-v1 ${seeded.ids.slack}@1.2.0 2026-10-29`),
      summary: {
        fr: '- Réseau vers hooks.slack.com seulement\n- Banc de conformité passé',
        en: '- Network to hooks.slack.com only\n- Conformance bench passed',
      },
    },
  });
  expect(signed.status(), await signed.text()).toBe(200);
  const sig = await page.request.get(`/api/v1/plugins/${seeded.ids.slack}/1.2.0/version.json.minisig`);
  expect(sig.status()).toBe(200);
  expect(await sig.text()).toContain(`escouade-verified-v1 ${seeded.ids.slack}@1.2.0`);

  await expect
    .poll(async () => {
      const c = (await (await page.request.get('/api/v1/catalog.json')).json()) as {
        plugins: { id: string; versions: { version: string; verified: boolean }[] }[];
      };
      return c.plugins.find((p) => p.id === seeded.ids.slack)?.versions.find((v) => v.version === '1.2.0')?.verified;
    })
    .toBe(true);
  await page.goto(`/plugins/${seeded.ids.slack}`);
  await expect(page.locator('.review-note.signed')).toContainText('Relu par Escouade le');
  await expect(page.locator('.review-note.signed li').first()).toHaveText('Réseau vers hooks.slack.com seulement');
  await expect(page.locator('.chain li.signed')).toContainText('Signée par Escouade');

  // Done: the signed version left « À relire » for « Faites ».
  await page.goto('/review?tab=done');
  await expect(row(page, name, '1.2.0')).toContainText('Vérifiée');
  await page.goto('/review');
  await expect(row(page, name, '1.2.0')).toHaveCount(0);

  // A token revoked does nothing more.
  await page.goto('/account#tokens');
  await page
    .locator('#tokens li', { hasText: `Poste ${seeded.run}` })
    .getByRole('button', { name: 'Révoquer' })
    .click();
  await expect(page.locator('#tokens')).not.toContainText(`Poste ${seeded.run}`);
  expect((await page.request.get('/api/v1/review/queue', { headers: auth })).status()).toBe(401);
});

test('the administrator asks for changes, and closes a report', async ({ page }) => {
  await signIn(page, E2E_ADMIN_LOGIN);
  const name = `Slack ${seeded.run}`;
  await row(page, name, '1.1.0').locator('a.pl').click();
  const detail = page.locator('aside.detail');
  await detail.locator('summary', { hasText: 'Demander des changements' }).click();
  await detail.locator('textarea[name=message]').first().fill('Déclare api.slack.com dans permissions.network.');
  await detail.getByRole('button', { name: 'Envoyer' }).click();
  await expect(page).toHaveURL(/notice=review-changes/);
  await expect(page.locator('.notice')).toContainText('Changements demandés');
  await expect(row(page, name, '1.1.0')).toHaveCount(0);

  const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL, max: 1 });
  await pool.query(
    `insert into reports (plugin_id, version, reason, details, email) values ($1, '1.2.0', 'malicious', $2, 'qui@exemple.dev')`,
    [seeded.ids.slack, 'Il envoie le webhook à un autre hôte que hooks.slack.com.'],
  );
  await pool.end();
  await page.goto('/review?tab=reports');
  await page.locator('table.t tr', { hasText: name }).first().locator('a.pl').click();
  await expect(page.locator('aside.detail')).toContainText('Il envoie le webhook');
  await expect(page.locator('aside.detail')).not.toContainText('qui@exemple.dev');
  await expect(page.locator('aside.detail .cmd')).toContainText(`revoke ${seeded.ids.slack}@1.2.0 --disable`);
  await page.locator('aside.detail').getByRole('button', { name: 'Classer sans suite' }).click();
  await expect(page).toHaveURL(/notice=review-report-dismissed/);
});

const SELS = {
  title: '.page-title',
  chip: '.qtabs .chip',
  on: '.qtabs .chip.on',
  th: 'table.t th',
  name: 'table.t td .pl b',
  detail: 'aside.detail',
  cmd: 'aside.detail .cmd',
  perm: 'aside.detail .perm.strong',
  flag: 'aside.detail .flag',
} as const;

async function measure(page: Page) {
  return page.evaluate((sels) => {
    const out: Record<string, Record<string, string> | null> = {};
    for (const [k, sel] of Object.entries(sels)) {
      const el = document.querySelector(sel);
      if (!el) {
        out[k] = null;
        continue;
      }
      const s = getComputedStyle(el);
      out[k] = {
        size: s.fontSize,
        weight: s.fontWeight,
        color: s.color,
        bg: s.backgroundColor,
        border: s.borderTopColor,
        radius: s.borderTopLeftRadius,
        family: s.fontFamily.split(',')[0]!.replace(/"/g, ''),
      };
    }
    return out;
  }, SELS);
}

for (const scheme of ['dark', 'light'] as const)
  test(`the queue looks like the preview (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${PREVIEW}${scheme === 'light' ? '?theme=light' : ''}#review`);
    const preview = await measure(page);
    // A fresh seed: the tests above answered this file's versions; an update with what grew is chosen, as in the preview.
    const fresh = await seedCatalog();
    await signIn(page, E2E_ADMIN_LOGIN);
    await row(page, `Slack ${fresh.run}`, '1.2.0').locator('a.pl').click();
    await expect(page.locator('aside.detail .perm').first()).toBeVisible();
    const store = await measure(page);
    for (const k of Object.keys(SELS)) {
      expect(store[k], `${k} is on the page`).not.toBeNull();
      expect(preview[k], `${k} is in the preview`).not.toBeNull();
      for (const p of ['size', 'weight', 'color', 'bg', 'border', 'radius'] as const)
        // The preview's strong line is its second, the page's its first (whose border-top is none): the same rule.
        if (!(k === 'perm' && p === 'border')) expect(store[k]![p], `${k} ${p}`).toBe(preview[k]![p]);
    }
  });

for (const [w, h] of [
  [1440, 900],
  [1028, 779],
  [390, 844],
] as const)
  test(`the queue holds in ${w} px, without scrolling sideways`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await signIn(page, E2E_ADMIN_LOGIN);
    for (const path of ['/review', '/review?tab=reports', '/review?tab=names', '/review?tab=done', '/en/review']) {
      await page.goto(path);
      const wide = await page.evaluate(() => document.documentElement.scrollWidth);
      expect(wide, path).toBeLessThanOrEqual(w);
    }
  });
