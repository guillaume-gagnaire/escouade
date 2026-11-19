// The spaces (ESC-539, direction B « les piles », docs/SERVER-UX.md § 5 and 6): two `escouade-server` (their debug
// build) behind the fake `ssh` of tests/fixtures/fake-ssh.mjs, each its own local forward — never a real SSH. Two spaces
// paired from the window; a request on each; the piles of the rail (and their packing at 1024, the rail never
// scrolling); « Partout », the machine on each request and on its gesture, a request answered from there on the right
// server; one server cut without touching the other; the policy that leaves the numbers alone; the shortcuts; a space
// in its own window, one at most; Réglages de l'espace › Appareils; the memory of three connections, measured. At both
// sizes (ESCOUADE_E2E_VIEWPORT=1024x728).
//
// With ESCOUADE_E2E_SHOTS=<folder>, the screens are captured there, dark and light, at the window's size.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, send, test } from './fixture';
import { serverCommand } from './server';

const policyDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-spaces-policies-'));
const POLICIES = path.join(policyDir, 'policies.json');
fs.writeFileSync(POLICIES, '{}');

test.use({ sshServer: 2, appEnv: { ESCOUADE_TEST_POLICIES: POLICIES } });

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const NARROW = process.env.ESCOUADE_E2E_VIEWPORT?.startsWith('1024');
const SIZE = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT.split('x')[0]}` : '';

/** The screen captured, dark then light (when asked). */
async function shot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.mouse.move(1, 1);
  for (const theme of ['dark', 'light']) {
    await page.evaluate((th) => (document.documentElement.dataset.theme = th), theme);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}${theme === 'light' ? '-clair' : ''}${SIZE}.png`), animations: 'disabled' });
  }
  await page.evaluate(() => delete document.documentElement.dataset.theme);
}

/** « Se connecter à un serveur » to `host` of ~/.ssh/config, paired with `code` (else by « le lancer d'ici par SSH »). */
async function connect(page: Page, host: string, name: string, opts: { port?: number; code?: () => string } = {}) {
  const settings = await openSettingsPage(page, 'Serveurs');
  await settings.getByTestId('servers-connect').click();
  const dialog = page.getByRole('dialog', { name: 'Se connecter à un serveur' });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('radio').filter({ hasText: host }).first().click();
  if (opts.port) await dialog.getByPlaceholder('46480').fill(String(opts.port));
  await dialog.getByRole('button', { name: /^Se connecter/ }).click();
  await expect(dialog.getByText(`Appairer ce poste à ${host}`)).toBeVisible({ timeout: 30_000 });
  const code = dialog.getByRole('group', { name: 'Code d’appairage, 8 caractères' });
  if (opts.code) {
    await code.getByRole('textbox').first().click();
    await page.keyboard.type(opts.code().replace('-', ''));
  } else {
    await dialog.getByTestId('pair-over-ssh').click();
    await expect(code.getByRole('textbox').last()).not.toHaveValue('', { timeout: 30_000 });
  }
  await dialog.getByRole('textbox', { name: 'Nom de l’espace' }).fill(name);
  await dialog.getByRole('button', { name: /^Appairer/ }).click();
  await expect(page.getByTestId('title-machine')).toContainText(name, { timeout: 60_000 });
}

/** A project and an agent of the space the window is on, waiting on a permission (`rm -rf build`). */
async function waitingAgent(page: Page, repo: string) {
  await addProject(page, repo);
  await send(page, 'permission');
  await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build', { timeout: 30_000 });
}

const section = (page: Page, name: string): Locator => page.getByTestId('everywhere-space').and(page.locator(`[aria-label="${name}"]`));

/** The core's and the window's memory now: the app's private memory (MB) and the page's JS heap (MB). */
async function memory(page: Page, exe: string): Promise<{ app: number; heap: number }> {
  const heap = await page.evaluate(() => (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize ?? 0);
  let app = 0;
  try {
    const out = execFileSync(
      'powershell',
      [
        '-NoProfile',
        '-Command',
        // Its private memory, the least of three readings a second apart: a moment's peak is not the connections'.
        `$m = 1..3 | ForEach-Object { (Get-Process | Where-Object { $_.Path -eq '${exe.replace(/'/g, "''")}' } | Measure-Object PrivateMemorySize64 -Sum).Sum; Start-Sleep -Seconds 1 }; ($m | Measure-Object -Minimum).Minimum`,
      ],
      { encoding: 'utf8' },
    );
    app = Number(out.trim()) || 0;
  } catch {
    // not measured
  }
  return { app: Math.round(app / 1048576), heap: Math.round(heap / 1048576) };
}

test('two servers as spaces: piles, « Partout » answered on the right server, a cut of one alone, the policy, the shortcuts, a space in its window', async ({
  app,
}, testInfo) => {
  test.setTimeout(600_000);
  const ssh = app.ssh!;
  const second = ssh.second!;
  const { page } = app;
  const exe = path.join(path.dirname(ssh.program), `escouade${path.extname(ssh.program)}`);
  // One space: the window of today, no machine, no pile.
  await expect(page.getByTestId('title-machine')).toHaveCount(0);
  await expect(page.getByTestId('rail-piles')).toHaveCount(0);
  await page.waitForTimeout(1500);
  const before = await memory(page, exe);

  // Travail, paired over SSH: the window goes on it; a request there.
  await connect(page, 'travail', 'Travail');
  await waitingAgent(page, app.repo);
  // Two spaces: the open one heads the projects, this computer is a pile.
  await expect(page.getByTestId('rail-space')).toContainText('Travail');
  await expect(page.getByTestId('rail-space-other')).toHaveAccessibleName(/^Ce poste, \d+ en attente \(Alt 1\)$/, { timeout: 20_000 });
  await shot(page, 'espaces-deux');

  // Perso, its code typed (its port on the server: 46481): the window goes on it; a request there too.
  await connect(page, 'perso', 'Perso', {
    port: 46481,
    code: () => {
      const printed = serverCommand(ssh.program, ['pair', '--data-dir', second.server.data], second.env);
      return /([A-Z2-9]{4}-[A-Z2-9]{4})/.exec(printed)![1];
    },
  });
  await waitingAgent(page, app.repo);

  // Three spaces: two piles, Travail's counting its request; the rail never scrolls; packed into plates at 1024.
  const piles = page.getByTestId('rail-piles');
  await expect(piles.getByTestId('rail-space-other')).toHaveCount(2);
  const travailPile = piles.locator('[data-testid="rail-space-other"]', { hasText: 'Travail' });
  await expect(travailPile).toHaveAccessibleName('Travail, 1 en attente (Alt 2)', { timeout: 20_000 });
  await expect(piles).toHaveAttribute('data-compact', NARROW ? 'true' : 'false');
  const rail = page.getByRole('navigation', { name: /Projets/ }).first();
  const overflow = await rail.evaluate((el) => el.scrollHeight - el.clientHeight);
  expect(overflow, 'the rail does not scroll').toBeLessThanOrEqual(0);
  for (const p of await piles.getByTestId('rail-space-other').all()) await expect(p).toBeInViewport();
  const three = await memory(page, exe);
  await shot(page, 'espaces-piles');

  // « Partout »: every space under its machine, the open one first; the machine on each request and on its gesture.
  await page.keyboard.press('Control+Shift+A');
  await expect(page.getByRole('tab', { name: /Partout/ })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByTestId('title-machine')).toContainText('Tous les espaces');
  await expect(page.getByTestId('everywhere-space')).toHaveCount(3);
  await expect(page.getByTestId('everywhere-space').first()).toHaveAttribute('aria-label', 'Perso');
  const travail = section(page, 'Travail');
  const perso = section(page, 'Perso');
  const allowTravail = travail.getByRole('button', { name: /^Autoriser sur Travail la demande de / });
  await expect(allowTravail).toHaveText('Autoriser sur Travail', { timeout: 20_000 });
  await expect(perso.getByRole('button', { name: /^Autoriser sur Perso/ })).toBeVisible();
  for (const row of await page.getByTestId('everywhere-row').all()) {
    const space = await row.locator('xpath=ancestor::section[1]').getAttribute('aria-label');
    await expect(row.getByTestId('row-machine')).toHaveText(new RegExp(`${space}$`));
  }
  await shot(page, 'espaces-partout');

  // Answered from there, without changing space: on Travail alone. Perso's still waits; the window stays on Perso.
  await allowTravail.click();
  await expect(travail.getByRole('button', { name: /^Autoriser/ })).toHaveCount(0, { timeout: 20_000 });
  await expect(perso.getByRole('button', { name: /^Autoriser sur Perso/ })).toBeVisible();
  await expect(page.getByTestId('title-machine')).toContainText('Tous les espaces');
  await expect(travailPile).toHaveAccessibleName('Travail, 0 en attente (Alt 2)', { timeout: 20_000 });

  // Travail cut, and unreachable a while: its section greyed, with no gesture; Perso untouched (no band, its gesture).
  fs.writeFileSync(path.join(ssh.dir, 'down-travail'), '');
  fs.writeFileSync(path.join(ssh.dir, 'cut-travail'), '');
  await expect(travail.getByRole('heading')).toContainText('reconnexion', { timeout: 30_000 });
  await expect(travailPile).toHaveAccessibleName('Travail, coupé (Alt 2)');
  await expect(travail.getByRole('button', { name: /Autoriser|Ouvrir/ })).toHaveCount(0);
  await expect(page.getByTestId('connection-band')).toHaveCount(0);
  await expect(perso.getByRole('button', { name: /^Autoriser sur Perso/ })).toBeVisible();
  await shot(page, 'espaces-coupe');
  fs.rmSync(path.join(ssh.dir, 'down-travail'));
  await expect(travail.getByRole('heading')).not.toContainText('reconnexion', { timeout: 60_000 });

  // This computer's policy denies the aggregate view: Travail's numbers alone, no text, no gesture.
  fs.writeFileSync(POLICIES, JSON.stringify({ 'servers.aggregateView': false }));
  await expect(travail.getByTestId('everywhere-counts')).toBeVisible({ timeout: 30_000 });
  await expect(travail.getByTestId('everywhere-row')).toHaveCount(0);
  await expect(travail).not.toContainText('rm -rf build');
  await shot(page, 'espaces-masque');
  // Locked in Réglages › Serveurs, with who imposes it.
  const settings = await openSettingsPage(page, 'Serveurs');
  await expect(settings.getByTestId('server-policy')).toContainText('Imposé par la stratégie de ce poste');
  await page.keyboard.press('Escape');
  fs.writeFileSync(POLICIES, '{}');
  await expect(travail.getByTestId('everywhere-counts')).toHaveCount(0, { timeout: 30_000 });

  // Réglages de l'espace › Appareils: this computer marked; a code created from here.
  const devices = await openSettingsPage(page, 'Appareils');
  await expect(devices.getByTestId('device-row').first()).toContainText('Ce poste');
  await devices.getByRole('button', { name: 'Créer un code' }).click();
  await expect(devices.getByTestId('device-code')).toContainText(/[A-Z2-9]{4}-[A-Z2-9]{4}/);
  await shot(page, 'espaces-appareils');
  await page.keyboard.press('Escape');

  // A space in its own window (S14): this computer's, from its pile's menu; asked again, the same window comes back.
  const poste = piles.locator('[data-testid="rail-space-other"]', { hasText: 'Poste' });
  const opened = page.context().waitForEvent('page');
  await poste.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Ouvrir dans une fenêtre' }).click();
  const win = await opened;
  await win.waitForURL(/space=here/, { timeout: 30_000 });
  await expect(win.getByTestId('title-machine')).toContainText('Ce poste', { timeout: 30_000 });
  await poste.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Ouvrir dans une fenêtre' }).click();
  await page.waitForTimeout(1500);
  expect(
    page
      .context()
      .pages()
      .filter((p) => p.url().includes('space=here')),
  ).toHaveLength(1);
  // Closed by its own button: it closes (the main window alone goes to the tray).
  await win.getByRole('button', { name: 'Fermer', exact: true }).click();
  await expect
    .poll(
      () =>
        page
          .context()
          .pages()
          .filter((p) => p.url().includes('space=here')).length,
      { timeout: 15_000 },
    )
    .toBe(0);

  // The shortcuts: Alt+1 this computer, Ctrl+² the next space, Alt+3 Perso.
  await page.keyboard.press('Escape');
  await page
    .locator('body')
    .click({ position: { x: 600, y: 400 } })
    .catch(() => {});
  // A request card just shown holds the keys that answer a moment (500 ms): past it, Alt+1 is the space's.
  await page.waitForTimeout(800);
  await page.keyboard.press('Alt+Digit1');
  await expect(page.getByTestId('rail-space')).toContainText('Poste', { timeout: 30_000 });
  await expect(page.getByTestId('title-machine')).toContainText('Ce poste');
  await page.keyboard.press('Control+Backquote');
  await expect(page.getByTestId('rail-space')).toContainText('Travail', { timeout: 30_000 });
  await page.keyboard.press('Alt+Digit3');
  await expect(page.getByTestId('rail-space')).toContainText('Perso', { timeout: 30_000 });

  // The memory of three connections (this computer's core, two servers through their tunnels), against one space.
  const note = `memory: one space ${before.app} MB (app) / ${before.heap} MB (heap); three spaces ${three.app} MB / ${three.heap} MB`;
  console.log(note);
  await testInfo.attach('memory', { body: note, contentType: 'text/plain' });
  if (SHOTS) fs.appendFileSync(path.join(SHOTS, 'memoire.txt'), `${process.env.ESCOUADE_E2E_VIEWPORT ?? '1440x900'} · ${note}\n`);
});
