// « Se connecter à un serveur » (ESC-537, docs/SERVER-UX.md § 2 and 3): the desktop app reaches an `escouade-server`
// (its debug build) through a tunnel the app starts itself — the fake `ssh` of tests/fixtures/fake-ssh.mjs, which makes the
// local forward as `ssh -L` would: never a real SSH. From this computer: the hosts of ~/.ssh/config, an unknown host key
// refused, a wrong code, the code got « d'ici par SSH », the pairing; then on the server: a project, an agent of the fake
// `claude` and its reply, the machine said; the tunnel cut and back (sending paused meanwhile); the token revoked on the
// server; the way back to this computer. At both sizes (ESCOUADE_E2E_VIEWPORT=1024x728).
//
// With ESCOUADE_E2E_SHOTS=<folder>, the screens are captured there, dark and light, at the window's size.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, send, test } from './fixture';
import { serverCommand, startBareServer, type BareServer } from './server';

test.use({ sshServer: true });

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZE = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT.split('x')[0]}` : '';

/** The screen captured, dark then light (when asked). */
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

test('connects to a server through an SSH tunnel: pairing, an agent, a cut and back, a revoked token, back on this computer', async ({
  app,
}) => {
  test.setTimeout(300_000);
  const ssh = app.ssh!;
  const { page } = app;
  // On this computer, nothing of a server yet: the title bar says no machine.
  await expect(page.getByTestId('title-machine')).toHaveCount(0);

  // Réglages › Serveurs › « Se connecter à un serveur ».
  const settings = await openSettingsPage(page, 'Serveurs');
  await expect(settings.getByText('Aucun serveur.', { exact: false })).toBeVisible();
  await settings.getByTestId('servers-connect').click();
  const dialog = page.getByRole('dialog', { name: 'Se connecter à un serveur' });
  await expect(dialog).toBeVisible();
  // The hosts of ~/.ssh/config, with what they point to; the pattern `Host *` is not one.
  const hosts = dialog.getByRole('radio');
  await expect(hosts).toHaveCount(2);
  await expect(hosts.first()).toContainText('travail');
  await expect(hosts.first()).toContainText('escouade@escouade.test.internal');
  await expect(hosts.nth(1)).toContainText('par travail (ProxyJump)');
  await shot(page, 'hote');

  // An unknown host key: refused, never accepted in the user's place.
  await hosts.nth(1).click();
  await dialog.getByRole('button', { name: /^Se connecter/ }).click();
  const problem = dialog.getByRole('alertdialog');
  await expect(problem).toContainText('Clé d’hôte inconnue', { timeout: 30_000 });
  await expect(problem).toContainText('connecte-toi une fois par ssh labo-inconnu');
  await expect(problem).toContainText('Rien n’a été changé sur ce poste ni sur le serveur.');
  await shot(page, 'cle-inconnue');
  const runs = fs.readFileSync(path.join(ssh.dir, 'runs.log'), 'utf8');
  expect(runs).toContain('StrictHostKeyChecking=yes');
  expect(runs).not.toMatch(/StrictHostKeyChecking=(no|accept-new)/);

  // The server's host: Escouade answers, the pairing step.
  await dialog.getByRole('button', { name: 'Modifier l’hôte' }).click();
  await dialog.getByRole('radio').first().click();
  await dialog.getByRole('button', { name: /^Se connecter/ }).click();
  await expect(dialog.getByText('Appairer ce poste à travail')).toBeVisible({ timeout: 30_000 });
  const code = dialog.getByRole('group', { name: 'Code d’appairage, 8 caractères' });
  // A wrong code (one waits on the server): said, emptied.
  serverCommand(ssh.program, ['pair', '--data-dir', ssh.server.data], ssh.env);
  await code.getByRole('textbox').first().click();
  await page.keyboard.type('ZZZZZZZZ');
  await dialog.getByRole('button', { name: /^Appairer/ }).click();
  await expect(dialog.getByRole('alert')).toHaveText('Ce code n’est pas le bon.');
  // « Le lancer d'ici par SSH »: `escouade-server pair` run through the same host, its code filled in.
  await dialog.getByTestId('pair-over-ssh').click();
  await expect(code.getByRole('textbox').last()).not.toHaveValue('', { timeout: 30_000 });
  const name = dialog.getByRole('textbox', { name: 'Nom de l’espace' });
  await name.fill('Travail');
  await shot(page, 'connexion');
  await dialog.getByRole('button', { name: /^Appairer/ }).click();

  // The window on the server: the machine said in the title bar, the rail, the status bar.
  await expect(page.getByTestId('title-machine')).toContainText('Travail', { timeout: 60_000 });
  // This computer as a pile (direction B, ESC-539): « Poste » under its plate, its whole name said.
  await expect(page.getByTestId('rail-space-other')).toHaveAccessibleName(/^Ce poste, /);
  // The token in the keychain of this computer, never in its settings nor its log.
  const servers = fs.readFileSync(path.join(app.data, 'servers.json'), 'utf8');
  expect(servers).toContain('Travail');
  const tokens = path.join(app.data, 'servers-tokens.json');
  const token = fs.existsSync(tokens) ? Object.values(JSON.parse(fs.readFileSync(tokens, 'utf8')) as Record<string, string>)[0] : null;
  for (const file of ['settings.json', 'servers.json']) {
    const text = fs.readFileSync(path.join(app.data, file), 'utf8');
    expect(text).not.toMatch(/"token"/);
    if (token) expect(text).not.toContain(token);
  }

  // A project and an agent, on the server.
  await addProject(page, app.repo);
  // The status bar and the rail, once a project is there: the tunnel and its space.
  await expect(page.getByTestId('status-tunnel')).toContainText('Travail · ssh travail', { timeout: 30_000 });
  await expect(page.getByTestId('rail-space')).toContainText('Travail');
  await expect(page.getByTestId('side-machine')).toHaveText('Travail :');
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => fs.readFileSync(path.join(ssh.server.data, 'state.json'), 'utf8'), { timeout: 20_000 }).toContain('demo-api');
  const own = path.join(app.data, 'state.json');
  expect(fs.existsSync(own) ? fs.readFileSync(own, 'utf8') : '').not.toContain('demo-api');
  await expect(page.getByTestId('status-tunnel')).toContainText(/\d+ ms/, { timeout: 20_000 });
  await shot(page, 'serveur');

  // The tunnel cut, the host unreachable a while: the band names the space, sending is paused, the draft kept.
  fs.writeFileSync(path.join(ssh.dir, 'down'), '');
  fs.writeFileSync(path.join(ssh.dir, 'cut'), '');
  const band = page.getByTestId('connection-band');
  await expect(band).toContainText('Connexion à Travail perdue à', { timeout: 30_000 });
  await expect(band).toContainText('Les agents continuent sur le serveur');
  await expect(page.getByTestId('title-machine')).toContainText('reconnexion');
  await expect(page.getByTestId('status-tunnel')).toContainText('reconnexion');
  const composer = page.getByPlaceholder('Envoi suspendu jusqu’à la reprise ; ton brouillon est gardé.');
  await expect(composer).toBeVisible();
  await composer.fill('Encore');
  await expect(page.getByRole('button', { name: /^Envoyer/ })).toBeDisabled();
  await shot(page, 'coupure');
  // Back: the screen caught up, the draft still there, and it goes.
  fs.rmSync(path.join(ssh.dir, 'down'));
  await expect(band).toBeHidden({ timeout: 60_000 });
  await expect(page.getByText('Reconnecté à Travail')).toBeVisible();
  await expect(
    page
      .getByRole('textbox', { name: /Écris|agent/ })
      .or(page.locator('textarea'))
      .first(),
  ).toHaveValue('Encore');
  await page.getByRole('button', { name: /^Envoyer/ }).click();
  await expect(page.getByText('Bonjour, tu as dit : Encore')).toBeVisible({ timeout: 30_000 });

  // The token revoked on the server (`escouade-server devices revoke`): the whole space says so, from where and when.
  const listed = serverCommand(ssh.program, ['devices', '--data-dir', ssh.server.data], ssh.env);
  const id = listed.trim().split('\n').at(-1)!.split(/\s+/)[0];
  serverCommand(ssh.program, ['devices', 'revoke', id, '--data-dir', ssh.server.data], ssh.env);
  const screen = page.getByTestId('server-screen');
  await expect(screen).toContainText('Ce poste n’est plus appairé à Travail', { timeout: 60_000 });
  await expect(screen).toContainText('depuis le serveur');
  await expect(screen.getByRole('button', { name: 'Appairer de nouveau' })).toBeVisible();
  await shot(page, 'jeton');

  // Back on this computer: its own space, nothing of the server's.
  await screen.getByRole('button', { name: 'Aller sur Ce poste' }).click();
  await expect(page.getByTestId('title-machine')).toContainText('Ce poste', { timeout: 30_000 });
  await expect(page.getByTestId('status-tunnel')).toHaveCount(0);
  await expect(page.getByTestId('rail-space-other')).toContainText('Travail');
  // Réglages › Serveurs: the server known, its state said.
  const again = await openSettingsPage(page, 'Serveurs');
  const row = again.getByTestId('server-row');
  await expect(row).toContainText('Travail');
  await expect(row).toContainText('Jeton révoqué', { timeout: 15_000 });
  await shot(page, 'etats');
});

// ESC-706 (BC-5): the server on its Unix socket. Under Linux, a real `escouade-server --socket`; under Windows, where the
// server has none (`--socket` is refused there), the fake `ssh` plays the socket by a port (its file `socket`:
// `<path>=<port>`): what the client does is the same, its status read by SSH, `ssh -L` to the socket's path.
test('reaches a server on its Unix socket, read by SSH; a forward refused there offers the port', async ({ app }) => {
  test.setTimeout(240_000);
  const ssh = app.ssh!;
  const { page } = app;
  const root = path.dirname(ssh.server.data);
  const data = path.join(root, 'server-socket-data');
  fs.mkdirSync(data);
  fs.copyFileSync(path.join(app.data, 'settings.json'), path.join(data, 'settings.json'));
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  const linux = process.platform === 'linux';
  const socket = linux ? path.join(root, 'run', 'server.sock') : '/run/user/1000/escouade/server.sock';
  let server: BareServer | null = null;
  try {
    server = linux
      ? await startBareServer(ssh.program, data, { ...ssh.env, ESCOUADE_DATA_DIR: data }, 0, socket)
      : await startBareServer(ssh.program, data, { ...ssh.env, ESCOUADE_DATA_DIR: data }, base ? base + 3 : 0);
    fs.writeFileSync(path.join(ssh.dir, 'socket'), linux ? socket : `${socket}=${server.port}`);
    // The host's SSH refuses forwards to a socket (AllowStreamLocalForwarding no): its own state, and the fallback.
    fs.writeFileSync(path.join(ssh.dir, 'no-streamlocal'), '');
    const settings = await openSettingsPage(page, 'Serveurs');
    await settings.getByTestId('servers-connect').click();
    const dialog = page.getByRole('dialog', { name: 'Se connecter à un serveur' });
    await dialog.getByRole('radio').first().click();
    await dialog.getByRole('button', { name: /^Se connecter/ }).click();
    const problem = dialog.getByRole('alertdialog');
    await expect(problem).toContainText('travail refuse la redirection vers la socket', { timeout: 30_000 });
    await expect(problem).toContainText('AllowStreamLocalForwarding no');
    await expect(dialog.getByTestId('socket-fallback')).toHaveText('sh install.sh --port 46480');
    await expect(dialog.getByTestId('copy-socket-fallback')).toBeVisible();
    await expect(problem).toContainText(`:${socket} travail`);
    await shot(page, 'socket-refusee');
    // Forwards allowed again: « Réessayer » reads where it listens, the tunnel goes to the socket, Escouade answers.
    fs.rmSync(path.join(ssh.dir, 'no-streamlocal'));
    await dialog.getByRole('button', { name: 'Réessayer' }).click();
    await expect(dialog.getByText('Appairer ce poste à travail')).toBeVisible({ timeout: 30_000 });
    const runs = fs.readFileSync(path.join(ssh.dir, 'runs.log'), 'utf8');
    expect(runs).toContain('status --json');
    // The forward goes to the socket's path: `-L 127.0.0.1:<free port>:<socket>`.
    const forwards = runs.split('\n').filter((l) => l.includes('"-L"'));
    expect(forwards.some((l) => /"127\.0\.0\.1:\d+:/.test(l) && l.includes(`:${socket}"`))).toBe(true);
    // Paired by a code of the server on the socket; servers.json keeps the socket beside the port.
    const printed = serverCommand(ssh.program, ['pair', '--data-dir', data], ssh.env);
    const code = /([A-HJ-NP-Z2-9]{4})-([A-HJ-NP-Z2-9]{4})/.exec(printed)!;
    await dialog.getByRole('group', { name: 'Code d’appairage, 8 caractères' }).getByRole('textbox').first().click();
    await page.keyboard.type(code[1] + code[2]);
    await dialog.getByRole('textbox', { name: 'Nom de l’espace' }).fill('Socket');
    await dialog.getByRole('button', { name: /^Appairer/ }).click();
    await expect(page.getByTestId('title-machine')).toContainText('Socket', { timeout: 60_000 });
    const saved = JSON.parse(fs.readFileSync(path.join(app.data, 'servers.json'), 'utf8'));
    expect(saved.servers[0]).toMatchObject({ name: 'Socket', socket, remotePort: 46480 });
    await shot(page, 'socket');
  } finally {
    await server?.stop();
  }
});
