// What changes machines in server mode (ESC-538, docs/REMOTE.md « Ce qui change de machine », docs/SERVER-UX.md § 4,
// S12, S13), on an `escouade-server` reached through the fake `ssh` (tests/fixtures/fake-ssh.mjs), as ESC-537's e2e:
// - « Nouveau projet » on the server: « Cloner un dépôt » first, then « Un dossier du serveur », its folders (a
//   repository and its branch), its disk; a project made in a folder of the server, nothing on this computer;
// - a file joined to a message: always a copy, sent through the remote API (it lands in the server's data);
// - the files an agent sent: downloaded to this computer when opened (`<root>/downloads/Escouade/Travail`), a script only
//   downloaded and shown in its folder, never opened; « Voir sur le serveur »;
// - a test server of the server relayed to a port of this computer, a link to it opened at this computer's address;
// - a notification of the server shown here, named by its space;
// - the sign-in of a Codex account on the server (the fake `codex login`, which waits for its return on localhost): its
//   page opened on this computer at a click, its return relayed to the server on the same port, the account signed in
//   and kept on the server; the secrets said kept in a file on the server.
// At both sizes (ESCOUADE_E2E_VIEWPORT=1024x728). With ESCOUADE_E2E_SHOTS=<folder>, the screens, dark and light.

import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, FAKE_CODEX, openNewProject, openSettingsPage, send, test } from './fixture';

test.use({ sshServer: true, appEnv: { ESCOUADE_CODEX: FAKE_CODEX, FAKE_CODEX_LOGIN_RELAY: '1' } });

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

/** « Se connecter à un serveur »: the host `travail`, the code got over SSH, the space « Travail »; the window on it. */
async function connect(page: Page) {
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
}

/** What this computer was asked to open (`ESCOUADE_OPEN_LOG`): never opened for real in the e2e. */
function opened(root: string): string[] {
  const file = path.join(root, 'opened.log');
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8').split(/\r?\n/).filter(Boolean) : [];
}

/** The notifications this computer was asked to show (`ESCOUADE_ALERT_LOG`). */
function alerts(root: string): { kind: string; title?: string; body?: string }[] {
  const file = path.join(root, 'alerts.log');
  return fs.existsSync(file)
    ? fs
        .readFileSync(file, 'utf8')
        .split(/\r?\n/)
        .filter(Boolean)
        .map((l) => JSON.parse(l))
    : [];
}

test('what changes machines on a server: its folders, the files sent and received, a port relayed, a notification, a sign-in relayed', async ({
  app,
}) => {
  test.setTimeout(360_000);
  const { page } = app;
  await connect(page);

  // ---------- « Nouveau projet » on the server ----------
  await openNewProject(page);
  const wizard = page.getByRole('dialog', { name: 'Nouveau projet' });
  await expect(wizard.getByTestId('server-lead')).toContainText(
    'Les dossiers sont ceux de Travail (escouade.test.internal), pas de ce poste',
  );
  // « Cloner un dépôt » first, and chosen; the system's picker never there.
  const tabs = wizard.getByRole('tab');
  await expect(tabs.first()).toHaveText('Cloner un dépôt');
  await expect(tabs.first()).toHaveAttribute('aria-selected', 'true');
  await expect(wizard.getByTestId('clone-repo')).toContainText('Le serveur clone avec ses propres clés');
  await expect(wizard.getByRole('button', { name: 'Parcourir' })).toHaveCount(0);
  // « Un dossier du serveur »: the server's folders (its home is the test's root here), the repository and its branch.
  await wizard.getByRole('tab', { name: 'Un dossier du serveur' }).click();
  const folders = wizard.getByTestId('server-folders');
  const repo = folders.getByRole('button', { name: /^demo-api/ });
  await expect(repo).toContainText(/git · (main|master)/, { timeout: 30_000 });
  await expect(folders).toContainText(/Le disque du serveur : .* libres sur /);
  await repo.click();
  await expect(wizard.getByRole('textbox', { name: 'Dossier' })).toHaveValue(app.repo);
  await shot(page, 'projet');
  await wizard.getByRole('button', { name: 'Créer sans l’assistant' }).click();
  await expect(wizard).toBeHidden();
  // The project is the server's: in its state, not in this computer's.
  await expect
    .poll(() => fs.readFileSync(path.join(app.ssh!.server.data, 'state.json'), 'utf8'), { timeout: 20_000 })
    .toContain('demo-api');
  const own = path.join(app.data, 'state.json');
  expect(fs.existsSync(own) ? fs.readFileSync(own, 'utf8') : '').not.toContain('demo-api');
  // Back in the browser, it is a project already.
  await openNewProject(page);
  await wizard.getByRole('tab', { name: 'Un dossier du serveur' }).click();
  await expect(wizard.getByTestId('server-folders').getByRole('button', { name: /^demo-api/ })).toContainText('Déjà un projet', {
    timeout: 20_000,
  });
  await page.keyboard.press('Escape');
  await expect(wizard).toBeHidden();

  // ---------- a file joined: a copy, sent ----------
  const composer = page.locator('textarea').first();
  await expect(composer).toBeVisible({ timeout: 30_000 });
  await page
    .locator('input[type="file"]')
    .first()
    .setInputFiles({
      name: 'export-du-poste.zip',
      mimeType: 'application/zip',
      buffer: Buffer.from([0x50, 0x4b, 0x05, 0x06, ...new Array(18).fill(0)]),
    });
  await expect(page.getByText('export-du-poste.zip').first()).toBeVisible({ timeout: 20_000 });
  await send(page, 'lis ce fichier');
  // The agent of the server reads the copy, in the server's data.
  await expect(page.getByText('Lu : export-du-poste.zip=22')).toBeVisible({ timeout: 30_000 });
  // The copy is in the server's data, never a path of this computer.
  const copies = (dir: string): string[] =>
    fs.existsSync(dir)
      ? fs
          .readdirSync(dir, { recursive: true })
          .map(String)
          .filter((f) => f.endsWith('export-du-poste.zip'))
      : [];
  await expect.poll(() => copies(path.join(app.ssh!.server.data, 'attachments')).length, { timeout: 20_000 }).toBeGreaterThan(0);
  expect(copies(path.join(app.data, 'attachments'))).toEqual([]);

  // ---------- a notification of the server, named by its space ----------
  const notifications = await openSettingsPage(page, 'Notifications');
  await notifications.getByRole('switch', { name: 'Notifications système' }).click();
  await page.keyboard.press('Escape');
  await send(page, 'bonjour');
  await expect(page.getByText('Bonjour, tu as dit : bonjour')).toBeVisible({ timeout: 30_000 });
  await expect
    .poll(
      () =>
        alerts(app.root)
          .filter((a) => a.kind === 'notify')
          .map((a) => a.title),
      { timeout: 30_000 },
    )
    .toContainEqual(expect.stringMatching(/^Travail · /));

  // ---------- the files an agent sent: downloaded, a script never opened ----------
  await send(page, 'envoie des fichiers [script]');
  const card = page.locator('[data-sent]');
  await expect(card).toBeVisible({ timeout: 30_000 });
  const pdf = card.locator('.row').filter({ hasText: 'contrat.pdf' });
  const script = card.locator('.row').filter({ hasText: 'rejouer.sh' });
  await expect(script).toContainText('un script ne s’ouvre pas d’Escouade : il se télécharge seulement');
  await expect(script.getByRole('button', { name: /Ouvrir/ })).toHaveCount(0);
  await pdf.getByRole('button', { name: 'Ouvrir contrat.pdf' }).click();
  const downloads = path.join(app.root, 'downloads', 'Escouade', 'Travail');
  await expect.poll(() => fs.existsSync(path.join(downloads, 'contrat.pdf')), { timeout: 20_000 }).toBe(true);
  await expect(pdf).toContainText(`envoyé depuis Travail · copié sur ce poste dans ${downloads}`);
  expect(opened(app.root)).toContain(`path ${path.join(downloads, 'contrat.pdf')}`);
  // The script: downloaded, shown in its folder, never opened.
  await script.getByRole('button', { name: 'Télécharger sur ce poste' }).click();
  await expect.poll(() => fs.existsSync(path.join(downloads, 'rejouer.sh')), { timeout: 20_000 }).toBe(true);
  await expect.poll(() => opened(app.root)).toContain(`reveal ${path.join(downloads, 'rejouer.sh')}`);
  expect(opened(app.root)).not.toContain(`path ${path.join(downloads, 'rejouer.sh')}`);
  // A file of no type Escouade opens (ESC-629): no « Ouvrir », the card says why; downloaded, shown in its folder only.
  const other = card.locator('.row').filter({ hasText: 'etude.docx' });
  await expect(other).toContainText('Escouade n’ouvre que les images, les PDF et les textes');
  await expect(other.getByRole('button', { name: /Ouvrir/ })).toHaveCount(0);
  await other.getByRole('button', { name: 'Télécharger sur ce poste' }).click();
  await expect.poll(() => fs.existsSync(path.join(downloads, 'etude.docx')), { timeout: 20_000 }).toBe(true);
  await expect.poll(() => opened(app.root)).toContain(`reveal ${path.join(downloads, 'etude.docx')}`);
  expect(opened(app.root)).not.toContain(`path ${path.join(downloads, 'etude.docx')}`);
  await shot(page, 'fichier');

  // ---------- a test server of the server, relayed to this computer ----------
  const dev = http.createServer((_, res) => res.end('serveur de test du serveur'));
  await new Promise<void>((ok) => dev.listen(0, '127.0.0.1', ok));
  const port = (dev.address() as { port: number }).port;
  try {
    await send(page, `regarde http://localhost:${port}/app`);
    const link = page.locator('.assistant .md a', { hasText: `http://localhost:${port}/app` }).last();
    await link.click();
    // Opened at a port of this computer (the server's is taken here: both are this machine), which relays it.
    await expect.poll(() => opened(app.root).find((l) => l.startsWith('url http://localhost:')), { timeout: 30_000 }).toBeTruthy();
    const url = opened(app.root)
      .find((l) => l.startsWith('url http://localhost:'))!
      .slice(4);
    const here = Number(new URL(url).port);
    expect(here).not.toBe(port);
    expect(new URL(url).pathname).toBe('/app');
    const got = await fetch(`http://127.0.0.1:${here}/app`).then((r) => r.text());
    expect(got).toBe('serveur de test du serveur');
  } finally {
    dev.close();
  }

  // ---------- the sign-in of a Codex account on the server, relayed ----------
  const accounts = await openSettingsPage(page, 'Comptes et harness');
  // The secrets of the server: in a file of its data, not this computer's keychain.
  await accounts.getByRole('button', { name: 'Ajouter un compte Codex…' }).click();
  const add = page.getByRole('dialog', { name: 'Ajouter un compte Codex' });
  await add.getByRole('textbox', { name: 'Nom' }).fill('Serveur');
  await add.getByRole('button', { name: 'Créer et se connecter' }).click();
  const signIn = page.getByRole('dialog', { name: 'Se connecter au compte « Serveur »' });
  const band = signIn.getByTestId('server-signin');
  await expect(band).toContainText('La page de connexion est prête', { timeout: 30_000 });
  await shot(page, 'compte');
  await band.getByRole('button', { name: 'Ouvrir la page sur ce poste' }).click();
  await expect(band).toContainText('La page de connexion s’est ouverte sur ce poste.');
  await expect(band).toContainText(/Le retour par localhost:\d+ est relayé à Travail/);
  await expect(band).toContainText('Le compte sera gardé sur Travail, pas sur ce poste.');
  const page_ = opened(app.root)
    .find((l) => l.startsWith('url https://auth.openai.com/'))!
    .slice(4);
  const back = new URL(new URL(page_).searchParams.get('redirect_uri')!);
  // The browser of this computer comes back to localhost: relayed to the server's harness, which signs in.
  const done = await fetch(`http://127.0.0.1:${back.port}/auth/callback?code=abc`).then((r) => r.text());
  expect(done).toContain('Signed in to Codex');
  await expect(signIn.getByRole('status')).toHaveText('Connecté au compte codex@example.com', { timeout: 30_000 });
  // The account is the server's: its sign-in in the server's data, nothing in this computer's.
  const signedIn = (dir: string) =>
    fs.existsSync(dir)
      ? fs
          .readdirSync(dir, { recursive: true })
          .map(String)
          .some((f) => f.endsWith('auth.json'))
      : false;
  expect(signedIn(path.join(app.ssh!.server.data, 'codex'))).toBe(true);
  expect(signedIn(path.join(app.data, 'codex'))).toBe(false);
  await signIn.getByRole('button', { name: 'Terminé' }).click();
});
