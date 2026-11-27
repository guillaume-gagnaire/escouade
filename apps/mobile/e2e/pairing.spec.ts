// The pairing of a space and its end (MO-4, ESC-570, docs/MOBILE.md § 3.1, § 3.9, § 3.10), at 360 × 780 and
// 430 × 932, against an `escouade-server` of the test's: the code typed with the space's address, the device born of it
// a phone's (its scope), its token never in the page's list; the space revoked from a computer and the phone's screen
// that says so, when and from which device; « Retirer »; a QR code read, what it holds shown; no pairing without a
// screen lock. The browser stands for the native side (`apps/mobile-plugin/src/web.ts`): the lock is the test's
// (`__ESCOUADE_LOCK__`), the camera too (`__ESCOUADE_SCAN__`), and a browser has no WireGuard.

import fs from 'node:fs';
import path from 'node:path';
import { expect, test as base, type Page, type TestInfo } from '@playwright/test';
import { ROOT, startSpace, type TestSpace } from './harness';

const sizeOf = (info: TestInfo) => (info.project.name.endsWith('360') ? '360' : '430');

const test = base.extend<{ space: TestSpace }>({
  space: async ({}, use) => {
    const space = await startSpace();
    await use(space);
    await space.stop();
  },
});

/** The page opened with no space, in `theme`, the lock and the camera the test's. */
async function open(page: Page, theme: 'dark' | 'light', lock: 'ok' | 'none' = 'ok', scan?: string) {
  await page.emulateMedia({ colorScheme: theme });
  await page.addInitScript(
    ([l, s]) => {
      const w = window as unknown as Record<string, unknown>;
      w.__ESCOUADE_LOCK__ = l;
      if (s) w.__ESCOUADE_SCAN__ = s;
    },
    [lock, scan ?? ''] as const,
  );
  await page.goto('/');
}

/** With ESCOUADE_MOBILE_SHOTS=1, the screen in design/mobile-2-3/app/ (`<name>[-clair]-<size>.png`). */
async function shot(page: Page, info: TestInfo, name: string) {
  if (!process.env.ESCOUADE_MOBILE_SHOTS) return;
  const theme = await page.evaluate(() => document.documentElement.dataset.theme);
  const dir = path.join(ROOT, 'design', 'mobile-2-3', 'app');
  fs.mkdirSync(dir, { recursive: true });
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({
    path: path.join(dir, `${name}${theme === 'light' ? '-clair' : ''}-${sizeOf(info)}.png`),
    animations: 'disabled',
  });
}

interface Device {
  id: string;
  name: string;
  scope?: string;
}

async function pairByCode(page: Page, space: TestSpace, name: string) {
  await page.getByRole('textbox', { name: 'Code à 8 caractères' }).fill(space.pairCode());
  await page.getByRole('textbox', { name: /Adresse de l’espace/ }).fill(space.address);
  await page.getByRole('button', { name: 'Appairer', exact: true }).click();
  await expect(page.getByTestId('pair-found')).toHaveText(space.address);
  await page.getByRole('textbox', { name: 'Le nom de ce téléphone sur cet espace' }).fill(name);
  await page.getByRole('button', { name: 'Appairer ce téléphone' }).click();
}

test('paired by its code, the phone is a phone of the server; revoked from a computer, it says so and forgets it', async ({
  page,
  space,
}, info) => {
  for (const theme of ['dark', 'light'] as const) {
    await open(page, theme);
    await expect(page.getByRole('heading', { name: 'Escouade sur ce téléphone' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Scanner le QR code' })).toBeVisible();
    await shot(page, info, 'appairage');
    await pairByCode(page, space, `Pixel ${theme}`);
    // The shell, on the space paired.
    await expect(page.getByRole('tablist')).toBeVisible({ timeout: 30_000 });
    await expect(page.getByRole('radio', { name: /^127\.0\.0\.1, 0 en attente/ })).toBeVisible({ timeout: 30_000 });
    // On the server, a device of the scope « phone », of the name given.
    const devices = await space.call<Device[]>('remote_devices');
    const mine = devices.find((d) => d.name === `Pixel ${theme}`);
    expect(mine?.scope).toBe('phone');
    // The page's list holds no token, no key.
    const list = await page.evaluate(() => localStorage.getItem('escouade.spaces') ?? '');
    expect(list).toContain('127.0.0.1');
    expect(list).not.toMatch(/token|key/i);
    // Revoked from a computer (Réglages de l'espace › Appareils): its screen, when and from which device.
    await space.call('remote_device_revoke', { id: mine!.id });
    const dialog = page.getByRole('alertdialog', { name: 'Ce téléphone n’est plus appairé à 127.0.0.1' });
    await expect(dialog).toBeVisible({ timeout: 30_000 });
    await expect(dialog).toContainText(/Révoqué le .*, depuis Téléphone de test\./);
    await shot(page, info, 'revoque');
    // « Retirer 127.0.0.1 de ce téléphone »: forgotten, back to the start.
    await dialog.getByRole('button', { name: 'Retirer 127.0.0.1 de ce téléphone' }).click();
    await expect(page.getByRole('heading', { name: 'Escouade sur ce téléphone' })).toBeVisible();
    expect(await page.evaluate(() => localStorage.getItem('escouade.spaces'))).toBe('[]');
    await page.evaluate(() => localStorage.clear());
  }
});

test('« Retirer » a space asks its server to revoke this phone, from the « + » sheet', async ({ page, space }) => {
  await open(page, 'dark');
  await pairByCode(page, space, 'Pixel retiré');
  await expect(page.getByRole('tablist')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Créer' }).click();
  const sheet = page.getByRole('dialog', { name: 'Créer' });
  await sheet.getByRole('button', { name: 'Retirer 127.0.0.1 de ce téléphone' }).click();
  await sheet.getByRole('button', { name: 'Retirer', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Escouade sur ce téléphone' })).toBeVisible({ timeout: 30_000 });
  // The server revoked it, signed by the phone itself.
  await expect
    .poll(async () => (await space.call<Device[]>('remote_devices')).some((d) => d.name === 'Pixel retiré'), { timeout: 20_000 })
    .toBe(false);
});

test('a QR code read says what it holds, and a browser has no tunnel to reach it', async ({ page }, info) => {
  const qr = JSON.stringify({
    escouade: 'pair/1',
    name: 'DESKTOP-7QK2',
    here: true,
    version: '2.3.0',
    code: 'K7Q2-9MXA',
    expiresAt: Date.now() + 600_000,
    tunnel: { privateKey: 'x', peerPublicKey: 'y', endpoint: '203.0.113.7:51820', address: '10.77.0.2/32', api: '10.77.0.1:80' },
  });
  for (const theme of ['dark', 'light'] as const) {
    await open(page, theme, 'ok', qr);
    await page.getByRole('button', { name: 'Scanner le QR code' }).click();
    await expect(page.getByTestId('pair-found')).toHaveText('Ce poste · DESKTOP-7QK2 · Escouade 2.3.0');
    await shot(page, info, 'appairage-qr');
    await page.getByRole('button', { name: 'Appairer ce téléphone' }).click();
    await expect(page.getByRole('alert')).toContainText('Le poste ne répond pas', { timeout: 30_000 });
    await expect(page.getByRole('alert')).toContainText('203.0.113.7:51820');
  }
});

test('a phone without a screen lock is not paired', async ({ page, space }, info) => {
  for (const theme of ['dark', 'light'] as const) {
    await open(page, theme, 'none');
    await page.getByRole('textbox', { name: 'Code à 8 caractères' }).fill('ABCD-EFGH');
    await page.getByRole('textbox', { name: /Adresse de l’espace/ }).fill(space.address);
    await page.getByRole('button', { name: 'Appairer', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Ce téléphone n’a pas de verrou d’écran' })).toBeVisible();
    await shot(page, info, 'sans-verrou');
  }
  // Nothing reached the server: no device born.
  expect((await space.call<Device[]>('remote_devices')).map((d) => d.name)).toEqual(['Téléphone de test']);
});
