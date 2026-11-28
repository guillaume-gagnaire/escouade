// What the phone's e2e do with the page (MO-1, MO-5): open it on spaces, take a capture of a screen of B.

import fs from 'node:fs';
import path from 'node:path';
import type { Page, TestInfo } from '@playwright/test';
import { ROOT, type TestSpace } from './harness';

export const sizeOf = (info: TestInfo) => (info.project.name.endsWith('360') ? '360' : '430');
export const targetOf = (info: TestInfo) => (info.project.name.startsWith('android') ? 48 : 44);

export interface GivenSpace {
  id: string;
  name: string;
  here?: boolean;
  url: string;
  token: string;
}

/** The page opened on `spaces` (as MO-4 paired them), in `theme`. */
export async function open(page: Page, spaces: GivenSpace[], theme: 'dark' | 'light' = 'dark') {
  await page.emulateMedia({ colorScheme: theme });
  await page.addInitScript((given) => ((window as unknown as { __ESCOUADE_SPACES__: unknown }).__ESCOUADE_SPACES__ = given), spaces);
  await page.goto('/');
}

/** « Travail », the space of a test. */
export const travail = (space: TestSpace): GivenSpace[] => [{ id: 'travail', name: 'Travail', url: space.url, token: space.token }];

/** With ESCOUADE_MOBILE_SHOTS=1, the screen in design/mobile-2-3/app/ (`<name>[-clair]-<size>.png`). */
export async function shot(page: Page, info: TestInfo, name: string) {
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

export const tab = (page: Page, name: string | RegExp) => page.getByRole('tablist').getByRole('tab', { name });
