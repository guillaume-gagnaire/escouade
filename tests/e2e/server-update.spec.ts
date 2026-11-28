// « Mettre à jour le serveur » (ESC-702): the desktop app on an `escouade-server` (its debug build) through the fake
// `ssh`, a release of test on 127.0.0.1 whose latest.json says 9.9.9. On the server's space, Réglages › Général shows the
// server's own updates (its version, 9.9.9 found, its command, its two switches), never this computer's. The server then
// started again at an older version (ESCOUADE_SERVER_TEST_VERSION, debug builds only): the whole space says both versions,
// « Mettre à jour le serveur » asks it after a confirmation, through the tunnel, by this computer's token; the server
// (under Windows, without systemd) refuses and says why, once. In the image (ESCOUADE_SERVER_IMAGE), the `docker pull`
// to copy, no button. At both sizes and at 1028 × 779 (ESCOUADE_E2E_VIEWPORT).
//
// With ESCOUADE_E2E_SHOTS=<folder>, the screens are captured there, dark and light, at the window's size.

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';
import { startBareServer } from './server';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZE = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT.split('x')[0]}` : '';
const BASE = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
/** The release of test: the worktree's base + 6 when given. */
const RELEASE_PORT = BASE ? BASE + 6 : 47986;

async function shot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const theme of ['dark', 'light']) {
    await page.evaluate((th) => (document.documentElement.dataset.theme = th), theme);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}${theme === 'light' ? '-clair' : ''}${SIZE}.png`), animations: 'disabled' });
  }
  await page.evaluate(() => delete document.documentElement.dataset.theme);
}

let release: http.Server;
test.beforeAll(async () => {
  release = http.createServer((req, res) => {
    if (req.url === '/r/latest/download/latest.json') {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ version: '9.9.9', platforms: {} }));
    } else {
      res.writeHead(404);
      res.end();
    }
  });
  await new Promise<void>((done) => release.listen(RELEASE_PORT, '127.0.0.1', done));
});
test.afterAll(() => release?.close());

test.use({
  sshServer: true,
  // The server reads the release of test, and looks for a version every second (a build of development only).
  appEnv: { ESCOUADE_SERVER_UPDATE_URL: `http://127.0.0.1:${RELEASE_PORT}/r`, ESCOUADE_SERVER_UPDATE_EVERY: '1' },
});

test('the server’s update: its state on its space, then « Mettre à jour le serveur » on the refusal of the versions, and in the image', async ({
  app,
}) => {
  test.setTimeout(300_000);
  const ssh = app.ssh!;
  const { page } = app;

  // Paired to « Travail », its code got over SSH.
  const settings = await openSettingsPage(page, 'Serveurs');
  await settings.getByTestId('servers-connect').click();
  const dialog = page.getByRole('dialog', { name: 'Se connecter à un serveur' });
  await dialog.getByRole('radio').first().click();
  await dialog.getByRole('button', { name: /^Se connecter/ }).click();
  await expect(dialog.getByText('Appairer ce poste à travail')).toBeVisible({ timeout: 30_000 });
  await dialog.getByTestId('pair-over-ssh').click();
  const code = dialog.getByRole('group', { name: 'Code d’appairage, 8 caractères' });
  await expect(code.getByRole('textbox').last()).not.toHaveValue('', { timeout: 30_000 });
  await dialog.getByRole('textbox', { name: 'Nom de l’espace' }).fill('Travail');
  await dialog.getByRole('button', { name: /^Appairer/ }).click();
  await expect(page.getByTestId('title-machine')).toContainText('Travail', { timeout: 60_000 });

  // Réglages › Général of the space: the server's updates, 9.9.9 found, the command to run on it (no systemd here).
  const general = await openSettingsPage(page, 'Général');
  const group = general.getByTestId('server-updates');
  await expect(group).toContainText('Mises à jour de Travail');
  await expect(group).toContainText(/escouade-server \d+\.\d+\.\d+/);
  await expect(group).toContainText('9.9.9 est disponible', { timeout: 30_000 });
  await expect(group).toContainText('Ce serveur ne tourne pas sous systemd');
  await expect(group).toContainText('escouade-server update --to 9.9.9');
  await expect(group.getByRole('switch', { name: 'Mettre le serveur à jour automatiquement' })).toBeVisible();
  await expect(group.getByRole('switch', { name: 'Canal bêta' })).toBeVisible();
  // Not this computer's updates.
  await expect(general.getByRole('button', { name: 'Rechercher une mise à jour' })).toHaveCount(0);
  await group.scrollIntoViewIfNeeded();
  await shot(page, 'maj-serveur-reglages');
  // The server's log says the version found, once.
  await expect.poll(() => ssh.server.log(), { timeout: 20_000 }).toContain('9.9.9 is published');
  await page.keyboard.press('Escape');

  // The server started again at an older version: the whole space says both, and offers to update it.
  await ssh.server.stop();
  const older = await startBareServer(ssh.program, ssh.server.data, { ...ssh.env, ESCOUADE_SERVER_TEST_VERSION: '2.2.0' }, ssh.server.port);
  const screen = page.getByTestId('server-screen');
  await expect(screen).toContainText('Travail a une autre version d’Escouade', { timeout: 90_000 });
  await expect(screen).toContainText('le serveur 2.2.0');
  const update = screen.getByTestId('update-server');
  await expect(update).toContainText(/escouade-server update --to \d+\.\d+\.\d+/);
  await shot(page, 'maj-serveur-refus');
  await update.getByRole('button', { name: 'Mettre à jour le serveur' }).click();
  const confirm = update.getByRole('group', { name: /^Mettre à jour Travail en / });
  await expect(confirm).toContainText('vérifie sa signature');
  await shot(page, 'maj-serveur-confirmer');
  await confirm.getByRole('button', { name: 'Mettre à jour' }).click();
  // Without systemd, the server refuses, said once: never asked again by itself.
  await expect(update.getByTestId('update-failed')).toContainText('ne tourne pas sous systemd', { timeout: 30_000 });
  await expect(update.getByRole('button', { name: 'Mettre à jour le serveur' })).toBeEnabled();
  await shot(page, 'maj-serveur-erreur');
  await page.waitForTimeout(3000);
  expect(older.log().match(/asks the server's update/g)?.length).toBe(1);
  expect(older.log()).toContain('refused');

  // In the image: the docker pull to copy, no button that would change it.
  await older.stop();
  const image = await startBareServer(
    ssh.program,
    ssh.server.data,
    { ...ssh.env, ESCOUADE_SERVER_TEST_VERSION: '2.2.0', ESCOUADE_SERVER_IMAGE: '1' },
    ssh.server.port,
  );
  // Tried again by hand: refused again, the image's command this time. A try made before this computer's tunnel reaches
  // the server just started again (the CI's runners are slower) ends on « escouade-server ne répond pas »: tried again,
  // by its own « Réessayer », until the server's refusal of the versions comes back.
  await expect(async () => {
    await screen.getByRole('button', { name: 'Réessayer' }).first().click();
    await expect(page.getByTestId('update-server')).toContainText('Dans l’image Docker, rien n’est remplacé', { timeout: 10_000 });
  }).toPass({ timeout: 90_000 });
  expect(image.log()).toContain('refused a connection (version)');
  await expect(page.getByTestId('update-server')).toContainText('docker pull ghcr.io/guillaume-gagnaire/escouade/escouade-server:');
  await expect(page.getByTestId('update-server').getByRole('button', { name: 'Mettre à jour le serveur' })).toHaveCount(0);
  await shot(page, 'maj-serveur-image');
  await image.stop();
});
