// ESC-544: an agent whose remote control was turned on by hand stays reachable (never stopped for
// being idle, and started again with the app); one the setting « pour tous les agents » covers
// keeps the rule of the idle ones, and says it is asleep until its next message, or « Réveiller ».
import type { Locator, Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

// An idle delay of 4 s, looked at twice a second: what takes 30 minutes in the app.
const IDLE_MS = 4_000;
test.use({ appEnv: { ESCOUADE_IDLE_STOP_MS: String(IDLE_MS), ESCOUADE_IDLE_CHECK_EVERY_MS: '500' } });

const linked = (card: Locator) => card.locator('[data-tip*="Remote control : connecté"]');
const asleep = (card: Locator) => card.locator('[data-tip*="Remote control : en veille : reprend au prochain message"]');

async function menuOf(page: Page, card: Locator) {
  await card.click({ button: 'right' });
  return page.getByRole('menu');
}

test('an agent turned on by hand is never stopped for being idle, and starts again with the app', async ({ app }) => {
  let { page } = app;
  await addProject(page, app.repo);
  let card = page.locator('.card').first();
  await card.click({ button: 'right' });
  await page.getByRole('menuitem', { name: /Activer le remote control/ }).click();
  await expect(linked(card)).toBeVisible();
  // The fake CLI plays a message sent from the phone, which Claude answers: then the agent is idle.
  await expect(page.getByText('Bonjour, tu as dit : Message depuis le téléphone')).toBeVisible();
  // Three times the idle delay: still linked, never asleep.
  await page.waitForTimeout(3 * IDLE_MS);
  await expect(linked(card)).toBeVisible();
  await expect(asleep(card)).toHaveCount(0);
  const menu = await menuOf(page, card);
  await expect(menu.getByText('Relié', { exact: true })).toBeVisible();
  await expect(menu.getByRole('menuitem', { name: 'Réveiller' })).toHaveCount(0);
  await page.keyboard.press('Escape');

  // The app starts again: the agent turned on by hand starts with it and takes its session back, no message sent.
  page = await app.restart();
  card = page.locator('.card').first();
  // The app's start, then the agent's process and its link: more than a click.
  await expect(linked(card)).toBeVisible({ timeout: 30_000 });
  await expect(card.locator('[data-tip*="activé à la main"]')).toBeVisible();
});

test('an agent of the setting for all is asleep once idle, until « Réveiller » starts it again', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const card = page.locator('.card').first();
  const dialog = await openSettingsPage(page, 'Agents');
  await dialog.getByRole('switch', { name: 'Remote control pour tous les agents' }).click();
  await closeSettings(dialog);
  await expect(card.locator('[data-tip*="activé par le réglage de tous les agents"]')).toBeVisible();
  // Idle for longer than the delay: its process stops, its remote control stays, and the card says it is asleep.
  await expect(asleep(card)).toBeVisible({ timeout: 6 * IDLE_MS });
  await expect(asleep(card)).toHaveClass(/asleep/);
  const menu = await menuOf(page, card);
  await expect(menu.getByText('En veille : reprend au prochain message')).toBeVisible();
  await expect(menu.getByRole('menuitem', { name: 'Désactiver le remote control' })).toBeVisible();
  await menu.getByRole('menuitem', { name: 'Réveiller' }).click();
  // Its process starts again and takes its session back (before the idle delay stops it once more).
  await expect(asleep(card)).toHaveCount(0, { timeout: IDLE_MS });
  await expect(card.locator('[data-tip*="Remote control : "]')).not.toHaveAttribute('data-tip', /en veille/);
});
