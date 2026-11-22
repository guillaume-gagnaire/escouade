// The phone's shell, direction B (MO-1, docs/MOBILE.md), at 360 × 780 and 430 × 932, against an `escouade-server` of
// the test's: one connection per space, the three tabs, the chips, the « + » and its sheet, the system's back, the
// safe areas, the system's theme followed (light and dark), French and English, the sizes of what a finger touches.

import fs from 'node:fs';
import path from 'node:path';
import { expect, test as base, type Page, type TestInfo } from '@playwright/test';
import { ROOT, startSpace, stoppedAgent, type TestSpace } from './harness';

const sizeOf = (info: TestInfo) => (info.project.name.endsWith('360') ? '360' : '430');
const targetOf = (info: TestInfo) => (info.project.name.startsWith('android') ? 48 : 44);

const test = base.extend<{ space: TestSpace }>({
  space: async ({}, use) => {
    const space = await startSpace();
    await use(space);
    await space.stop();
  },
});

/** The page opened on `spaces` (as MO-4 will have paired them), in `theme`. */
async function open(
  page: Page,
  spaces: { id: string; name: string; here?: boolean; url: string; token: string }[],
  theme: 'dark' | 'light' = 'dark',
) {
  await page.emulateMedia({ colorScheme: theme });
  await page.addInitScript((given) => ((window as unknown as { __ESCOUADE_SPACES__: unknown }).__ESCOUADE_SPACES__ = given), spaces);
  await page.goto('/');
}

/** With ESCOUADE_MOBILE_SHOTS=1, the screen in design/mobile-2-3/app/ (`<name>[-clair]-<size>.png`). */
async function shot(page: Page, info: TestInfo, name: string) {
  if (!process.env.ESCOUADE_MOBILE_SHOTS) return;
  const theme = await page.evaluate(() => document.documentElement.dataset.theme);
  const dir = path.join(ROOT, 'design', 'mobile-2-3', 'app');
  fs.mkdirSync(dir, { recursive: true });
  // The fonts loaded, nothing moving.
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({
    path: path.join(dir, `${name}${theme === 'light' ? '-clair' : ''}-${sizeOf(info)}.png`),
    animations: 'disabled',
  });
}

const tab = (page: Page, name: string | RegExp) => page.getByRole('tablist').getByRole('tab', { name });

test('with no space paired, the phone says what it is for and where the pairing starts', async ({ page }, info) => {
  for (const theme of ['dark', 'light'] as const) {
    await open(page, [], theme);
    await expect(page.getByRole('heading', { name: 'Escouade sur ce téléphone' })).toBeVisible();
    await expect(page.getByRole('tablist')).toHaveCount(0);
    await shot(page, info, 'bienvenue');
  }
});

test('the shell on an escouade-server: one connection, the tabs, the « + », the system’s back, the theme followed', async ({
  page,
  space,
}, info) => {
  await open(page, [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }]);
  // The handshake passed through the forward (the phone's tunnel), and the glance read: the chip says what waits.
  await expect(page.getByRole('radio', { name: 'Travail, 0 en attente' })).toBeVisible();
  await expect.poll(() => space.server.log()).toContain('« Téléphone de test » connected');
  await expect(page.getByRole('radio', { name: 'Partout, 0 en attente' })).toHaveAttribute('aria-checked', 'true');
  await expect(page.getByRole('heading', { level: 1, name: 'À toi' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Rien ne t’attend' })).toBeVisible();
  await page.getByRole('radio', { name: /^Travail/ }).click();
  await expect(page.getByRole('heading', { name: 'Rien ne t’attend sur Travail' })).toBeVisible();

  // The tabs, then the system's back: one step at a time, back to « À toi ».
  await tab(page, 'Agents').click();
  await expect(page.getByRole('heading', { level: 1, name: 'Agents' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Aucun agent sur Travail' })).toBeVisible();
  await tab(page, 'Kanban').click();
  await expect(page.getByRole('heading', { name: 'Choisis un projet' })).toBeVisible();
  await expect(page.getByRole('radiogroup')).toHaveCount(0);
  await page.getByRole('button', { name: 'Créer' }).click();
  const sheet = page.getByRole('dialog', { name: 'Créer' });
  await expect(sheet.getByText('Nouveau projet')).toBeVisible();
  await expect(sheet.getByText('Nouvel agent')).toBeVisible();
  await expect(sheet.getByText('Nouveau ticket')).toBeVisible();
  await page.goBack();
  await expect(sheet).toBeHidden();
  await expect(tab(page, 'Kanban')).toHaveAttribute('aria-selected', 'true');
  await page.goBack();
  await expect(tab(page, /^À toi/)).toHaveAttribute('aria-selected', 'true');
  // The scrim closes the sheet too.
  await page.getByRole('button', { name: 'Créer' }).click();
  await page.mouse.click(10, 60);
  await expect(sheet).toBeHidden();

  // The system's theme, followed as it changes: the app's tokens in both.
  const bg = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(await page.evaluate(() => document.documentElement.dataset.theme)).toBe('dark');
  const dark = await bg();
  await page.emulateMedia({ colorScheme: 'light' });
  await expect.poll(() => page.evaluate(() => document.documentElement.dataset.theme)).toBe('light');
  expect(await bg()).not.toBe(dark);
});

test('an agent stopped on the server shows in « À toi » and « Agents », in both themes', async ({ page, space }, info) => {
  await stoppedAgent(space);
  for (const theme of ['dark', 'light'] as const) {
    await open(page, [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }], theme);
    await expect(page.getByRole('heading', { name: /Tes agents se sont arrêtés/ })).toBeVisible({ timeout: 45_000 });
    const row = page.getByRole('listitem').filter({ hasText: 'Autorisation' });
    await expect(row).toContainText('Travail');
    await expect(row).toContainText('facturation');
    await expect(tab(page, 'À toi, 1 en attente')).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Travail, 1 en attente' })).toBeVisible();
    await shot(page, info, 'a-toi');
    await tab(page, 'Agents').click();
    await expect(page.getByRole('listitem').filter({ hasText: 'Attend ta réponse' })).toContainText('facturation');
    await shot(page, info, 'agents');
    await tab(page, 'Kanban').click();
    await expect(page.getByRole('heading', { name: 'Choisis un projet' })).toBeVisible();
    await shot(page, info, 'kanban');
    await page.getByRole('button', { name: 'Créer' }).click();
    await expect(page.getByRole('dialog', { name: 'Créer' })).toBeVisible();
    await shot(page, info, 'creer');
  }
});

test.describe('two spaces', () => {
  let second: TestSpace | null = null;
  test.afterEach(async () => {
    await second?.stop();
    second = null;
  });

  test('a space cut keeps its chip, says so in a band and tries again; the other one goes on', async ({ page, space }, info) => {
    second = await startSpace();
    await open(page, [
      { id: 'here', name: 'Ce poste', here: true, url: space.url, token: space.token },
      { id: 'travail', name: 'Travail', url: second.url, token: second.token },
    ]);
    await expect(page.getByRole('radio', { name: 'Ce poste, 0 en attente' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Travail, 0 en attente' })).toBeVisible();
    // Each space its own connection: both servers saw the phone.
    await expect.poll(() => second!.server.log()).toContain('« Téléphone de test » connected');
    await expect.poll(() => space.server.log()).toContain('« Téléphone de test » connected');
    second.cut();
    await expect(page.getByRole('radio', { name: 'Travail, injoignable' })).toBeVisible();
    await expect(page.getByRole('status')).toContainText(/Travail injoignable · nouvel essai/);
    await expect(page.getByRole('radio', { name: 'Ce poste, 0 en attente' })).toBeVisible();
    await shot(page, info, 'coupe');
    await page.getByRole('status').getByRole('button', { name: 'Réessayer' }).click();
    await expect(page.getByRole('radio', { name: 'Travail, injoignable' })).toBeVisible();
  });
});

test('the phone’s sizes: what a finger touches is 44 px under iOS, 48 under Android; the safe areas are kept', async ({
  page,
  space,
}, info) => {
  // The safe areas as Capacitor gives them under Android (`SystemBars`, insetsHandling css).
  await page.addInitScript(() =>
    document.addEventListener('DOMContentLoaded', () => {
      const s = document.documentElement.style;
      s.setProperty('--safe-area-inset-top', '32px');
      s.setProperty('--safe-area-inset-bottom', '24px');
    }),
  );
  await open(page, [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }]);
  await expect(page.getByRole('radio', { name: 'Travail, 0 en attente' })).toBeVisible();
  const target = targetOf(info);
  for (const control of [
    page.getByRole('button', { name: 'Créer' }),
    page.getByRole('radio', { name: /^Partout/ }),
    page.getByRole('radio', { name: /^Travail/ }),
    tab(page, /^À toi/),
    tab(page, 'Agents'),
    tab(page, 'Kanban'),
  ]) {
    const box = (await control.boundingBox())!;
    expect(box.height, (await control.getAttribute('aria-label')) ?? '').toBeGreaterThanOrEqual(target);
    expect(box.width).toBeGreaterThanOrEqual(target);
  }
  // The body's text at 15 px.
  expect(await page.evaluate(() => getComputedStyle(document.body).fontSize)).toBe('15px');
  // Under the status bar: the title starts below it; above the gesture bar: the tabs end above it.
  const title = (await page.getByRole('heading', { level: 1 }).boundingBox())!;
  expect(title.y).toBeGreaterThanOrEqual(32);
  const tabs = (await page.getByRole('tablist').boundingBox())!;
  const height = page.viewportSize()!.height;
  expect(tabs.y + tabs.height).toBeLessThanOrEqual(height - 24);
  // Nothing goes past the screen's width.
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize()!.width);
});

test.describe('in English', () => {
  test.use({ locale: 'en-US' });

  test('the shell speaks the system’s language', async ({ page, space }) => {
    await open(page, [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }]);
    await expect(page.getByRole('radio', { name: 'Travail, 0 waiting' })).toBeVisible();
    await expect(page.getByRole('heading', { level: 1, name: 'Your turn' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Nothing is waiting for you' })).toBeVisible();
    await tab(page, 'Agents').click();
    await expect(page.getByRole('heading', { name: 'No agent' })).toBeVisible();
    await page.getByRole('button', { name: 'Create' }).click();
    await expect(page.getByRole('dialog', { name: 'Create' }).getByText('New project')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.lang)).toBe('en');
  });
});
