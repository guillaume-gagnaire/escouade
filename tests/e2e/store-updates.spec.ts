// « Rechercher des mises à jour » reads the store again first (ESC-405), on the real app against the fake store of
// tests/fixtures/store/fake-store.mjs: a version published after the start is offered at the click, the page says when the
// store was read, and a store that does not answer is said with the date of the offers looked at.

import { addProject, expect, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

let fake: { url: string; key: string; close: () => Promise<void> };

test.beforeAll(async () => {
  fake = await startFakeStore(0);
  process.env.ESCOUADE_STORE_URL = fake.url;
  process.env.ESCOUADE_STORE_MIRROR_URL = `${fake.url}/mirror`;
  process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_URL;
  delete process.env.ESCOUADE_STORE_MIRROR_URL;
  delete process.env.ESCOUADE_STORE_TEST_KEY;
});

const post = (p: string, body: unknown) =>
  fetch(`${fake.url}${p}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
const catalogueReads = async () =>
  ((await (await fetch(`${fake.url}/_fake/state`)).json()).requests as string[]).filter((r) => r === 'GET /api/v1/catalog.json').length;

test('a version published after the start is offered at the click, and an unreachable store is said', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  const side = page.locator('.plugins-side');
  const work = page.locator('.plugins-work');
  await side.getByRole('button', { name: /^Store/ }).click();
  await expect(work.locator('.sub')).toContainText('catalogue lu à');
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('regles equipe');
  await work.locator('.rowp').first().getByRole('button', { name: 'Installer' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Installer', exact: true }).click();
  await expect(dialog).toBeHidden();

  await side.getByRole('button', { name: /^Mises à jour/ }).click();
  const look = work.getByRole('button', { name: 'Chercher maintenant' });
  await look.click();
  await expect(look).toBeEnabled();
  await expect(work.getByText('Tes plugins sont à jour.')).toBeVisible();

  // Published after the start; the reading of the store is older than 30 s by the next click.
  await post('/_fake/bump', { id: 'dev.exemple.team-rules', version: '1.1.0' });
  await page.waitForTimeout(31_000);
  const before = await catalogueReads();
  await look.click();
  await expect(work.locator('.ups')).toContainText('v1.0.0 → v1.1.0');
  await expect(work.getByRole('status')).toHaveText(/^Store lu le \d\d\/\d\d \d\d:\d\d$/);
  // Clicks close together read it once.
  await look.click();
  await expect(look).toBeEnabled();
  await look.click();
  await expect(look).toBeEnabled();
  expect(await catalogueReads()).toBe(before + 1);

  // The store does not answer: the search goes on, on the offers kept, and says of when they are.
  await post('/_fake/down', { down: true });
  await page.waitForTimeout(31_000);
  await look.click();
  await expect(work.getByRole('status')).toHaveText(/^Store injoignable : offres du \d\d\/\d\d \d\d:\d\d$/);
  await expect(work.locator('.ups')).toContainText('v1.0.0 → v1.1.0');
  await expect(work.getByRole('status')).toBeInViewport();
});
