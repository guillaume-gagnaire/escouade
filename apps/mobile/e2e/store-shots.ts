// The captures of the stores (MO-8): see store-shots.config.ts. Each project is a size and a locale; each capture a
// screen of B on fictitious data (the demo of `npm run dev:server -- --demo`), in the dark theme, the first one also in
// the light theme. The texts of the agents are the fake `claude`'s, in French; the app's own texts follow the locale.

import fs from 'node:fs';
import path from 'node:path';
import { expect, test as base, type Locator, type Page, type TestInfo } from '@playwright/test';
import { kanbanProject } from '../scripts/local-space.mjs';
import { demo, ROOT, startSpace, type TestSpace } from './harness';
import { open, travail } from './page';

const test = base.extend<{ space: TestSpace }>({
  space: async ({}, use) => {
    const space = await startSpace();
    await use(space);
    await space.stop();
  },
});

const line = (page: Page, text: string | RegExp) => page.getByTestId('line').filter({ hasText: text });
const unfold = async (l: Locator) => {
  const head = l.getByTestId('line-head');
  if ((await head.getAttribute('aria-expanded')) !== 'true') await head.click();
  await expect(head).toHaveAttribute('aria-expanded', 'true');
};
/** The tabs in their order: « À toi », « Agents », « Kanban ». */
const tabAt = (page: Page, i: number) => page.getByRole('tablist').getByRole('tab').nth(i);

/** apps/mobile/store/screenshots/<store>/<locale>/<n>-<name>[-light].png */
async function capture(page: Page, info: TestInfo, n: number, name: string, theme: 'dark' | 'light' = 'dark') {
  const [size, locale] = info.project.name.split('@');
  const dir = path.join(ROOT, 'apps', 'mobile', 'store', 'screenshots', size, locale);
  fs.mkdirSync(dir, { recursive: true });
  await page.evaluate(() => document.fonts.ready);
  // Nothing goes past the screen's width.
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize()!.width);
  await page.screenshot({
    path: path.join(dir, `${n}-${name}${theme === 'light' ? '-light' : ''}.png`),
    animations: 'disabled',
  });
}

test('the screens of the thread, an authorization, the box, the conversation and the agents', async ({ page, space }, info) => {
  await demo(space);
  for (const theme of ['dark', 'light'] as const) {
    await open(page, travail(space), theme);
    const ask = line(page, /tableau de bord/);
    await expect(ask).toBeVisible({ timeout: 45_000 });
    await expect(ask.locator('img')).toHaveCount(3);
    await expect(page.getByTestId('line')).toHaveCount(5, { timeout: 45_000 });
    await capture(page, info, 1, 'thread', theme);
    if (theme === 'light') break;

    const perm = line(page, 'rm -rf build');
    await unfold(perm);
    await capture(page, info, 2, 'authorization');

    await unfold(ask);
    await ask.locator('img').first().scrollIntoViewIfNeeded();
    await capture(page, info, 3, 'choice');

    await unfold(perm);
    await perm.getByRole('button', { name: /^Conversation/ }).click();
    const talk = page.getByRole('dialog', { name: 'lance-les-fake' });
    await expect(talk.getByTestId('command')).toBeVisible();
    await capture(page, info, 4, 'conversation');
    await page.goBack();
    await expect(talk).toBeHidden();

    await tabAt(page, 1).click();
    await expect(page.getByRole('listitem').first()).toBeVisible();
    await capture(page, info, 5, 'agents');
  }
});

test('the screen of the Kanban', async ({ page, space }, info) => {
  await kanbanProject(space.call, space.root);
  await open(page, travail(space));
  await tabAt(page, 2).click();
  await expect(page.locator('[data-ticket]')).toHaveCount(2, { timeout: 45_000 });
  await capture(page, info, 6, 'kanban');
});
