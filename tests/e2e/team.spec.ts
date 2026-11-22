// The team server (TS-3) on the real app, against a real `escouade-team-server` started by the test: Alice creates a
// team (its 24 words noted, three typed back), invites; Bob, a second client on a data folder of his own, joins by the
// link (he checks the team's six words) and waits; Alice's app, online, lets him in; Bob sees it. Then the alarm of a
// fork as the window shows it. With `ESCOUADE_TEAM_SHOTS=<folder>`, the screens are kept there, dark and light.
//
// The server's binary is built when it is not there (`cargo build -p escouade-team-server`). Its port: the worktree's
// last one (`ESCOUADE_PORT_BASE` + 9) when the run has a range, else one the system gives.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const SHOTS = process.env.ESCOUADE_TEAM_SHOTS;

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 9;
  return new Promise((resolve) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => {
      const port = (s.address() as net.AddressInfo).port;
      s.close(() => resolve(port));
    });
  });
}

test.beforeAll(async () => {
  test.setTimeout(600_000);
  if (!fs.existsSync(SERVER)) {
    execFileSync('cargo', ['build', '-p', 'escouade-team-server'], { cwd: path.join(ROOT, 'src-tauri'), stdio: 'inherit' });
  }
  dir = fs.mkdtempSync(path.join(process.env.TEMP ?? process.env.TMPDIR ?? '/tmp', 'team-server-'));
  const out = execFileSync(SERVER, ['init', '--data', dir], { encoding: 'utf8' });
  token = /^\s{4}(\S+)\s*$/m.exec(out)![1];
  const port = await freePort();
  host = `127.0.0.1:${port}`;
  server = spawn(SERVER, ['serve', '--data', dir, '--listen', host], { stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`http://${host}/healthz`)).ok) return;
    } catch {
      // not yet
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`escouade-team-server did not answer on ${host}`);
});

test.afterAll(async () => {
  if (server?.pid) {
    const gone = new Promise((r) => server!.once('exit', r));
    if (process.platform === 'win32') {
      try {
        execFileSync('taskkill', ['/PID', String(server.pid), '/T', '/F'], { stdio: 'ignore' });
      } catch {
        // already gone
      }
    } else server.kill();
    await Promise.race([gone, new Promise((r) => setTimeout(r, 5000))]);
  }
  // Its base may be held a moment more by the system: what is left goes with the run's temporary folder.
  if (dir) fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

test.use({ appEnv: { ESCOUADE_TEAMS_EVERY_MS: '800' } });

/** « Ouvrir les équipes… » of Réglages › Avancé (the rail's « Ajouter » menu has it too, when it shows a menu). */
async function openTeams(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  await expect(page.getByRole('dialog', { name: 'Équipes' })).toBeVisible();
}

/** The window in a theme, as « Thème » sets it. */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** The dialog on screen, in both themes, when the run keeps the screens. */
async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const dialog = page.getByRole('dialog');
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(150);
    await dialog.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`), animations: 'disabled' });
  }
  await theme(page, 'dark');
}

/** The team's six words, as a card or a preview shows them. */
const sixWords = (page: Page) => page.getByRole('group', { name: 'Empreinte de l’équipe' }).first().getByRole('listitem').allTextContents();

test('a team created, an invitation, a second client that joins and is let in by the first', async ({ app }) => {
  test.setTimeout(240_000);
  let page = app.page;

  // Alice creates the team.
  await openTeams(page);
  await shoot(page, '01-equipes-vide');
  await page.getByRole('button', { name: /Créer une équipe/ }).click();
  await page.getByLabel('Adresse du serveur').fill(host);
  await page.getByLabel(/Jeton de création/).fill(token);
  await page.getByLabel('Nom de l’équipe').fill('ACME');
  await shoot(page, '02-creer');
  await page.getByRole('button', { name: 'Créer l’équipe' }).click();

  // Its 24 words, shown once; three typed back.
  const grid = page.getByRole('list', { name: 'La clé de secours' });
  await expect(grid.getByRole('listitem')).toHaveCount(24, { timeout: 30_000 });
  const words = await grid.locator('.w').allTextContents();
  await shoot(page, '03-mots');
  // The team's six words on the same sheet (SR2-09): the recovery asks for them too.
  await expect(page.getByRole('group', { name: 'Les six mots de l’équipe' }).getByRole('listitem')).toHaveCount(6);
  await page.getByLabel('J’ai noté les 24 mots et les six mots de l’équipe').check();
  await page.getByRole('button', { name: 'Continuer' }).click();
  const asked = page.getByLabel(/Mot n° \d+/);
  await expect(asked).toHaveCount(3);
  await shoot(page, '04-trois-mots');
  for (let i = 0; i < 3; i++) {
    const label = await asked.nth(i).evaluate((el) => el.closest('label')!.textContent!);
    const n = Number(/\d+/.exec(label)![0]);
    await asked.nth(i).fill(words[n - 1]);
  }
  await page.getByRole('button', { name: 'Confirmer' }).click();
  await expect(page.getByRole('dialog', { name: 'L’équipe est prête' })).toBeVisible();
  const six = await sixWords(page);
  expect(six).toHaveLength(6);
  await expect(page.getByText(/invite un second administrateur/)).toBeVisible();
  await shoot(page, '05-equipe-prete');

  // The invitation: 72 h, one use; its link and its QR code.
  await page.getByRole('button', { name: 'Inviter quelqu’un' }).click();
  await page.getByRole('button', { name: 'Créer le lien' }).click();
  const linkField = page.getByLabel('Le lien à envoyer');
  // The app's link (SR2-08): it opens Escouade, never a page of the server.
  await expect(linkField).toHaveValue(new RegExp(`^escouade://team/join/${host.replace(/\./g, '\\.')}/[0-9a-f]{32}#s=`));
  const link = await linkField.inputValue();
  await expect(page.getByRole('img', { name: 'Ou fais scanner ce QR code' }).locator('svg')).toBeVisible();
  await expect(page.getByText(/Prêt : l’invité peut venir/)).toBeVisible();
  await shoot(page, '06-invitation');
  expect(link).not.toMatch(/https?:/);
  await expect(page.getByText(/colle dans Escouade, « Rejoindre par un lien »/)).toBeVisible();
  // The page of a link of before: a fixed text, no script to read what follows the `#`.
  const id = /\/([0-9a-f]{32})#/.exec(link)![1];
  const old = `https://${host}/join/${id}#${link.split('#')[1]}`;
  const served = await fetch(`http://${host}/join/${id}`);
  expect(served.headers.get('content-security-policy')).toContain("script-src 'none'");
  const joinPage = await served.text();
  expect(joinPage).not.toMatch(/<script|location|hash/i);
  expect(joinPage).toContain('Demande un nouveau lien');

  // Bob: a data folder of his own.
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  fs.writeFileSync(path.join(bobData, 'settings.json'), fs.readFileSync(path.join(app.data, 'settings.json')));
  page = await app.restartAs(bobData);
  await openTeams(page);
  await expect(page.getByText('Cet appareil n’est dans aucune équipe.')).toBeVisible();
  await page.getByRole('button', { name: /Rejoindre par un lien/ }).click();
  // The link of before: refused, with the word to ask for a new one; nothing sent.
  await page.getByLabel('Lien d’invitation').fill(old);
  await expect(page.getByText(/a l’ancienne forme/)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Rejoindre' })).toBeDisabled();
  await page.getByLabel('Lien d’invitation').fill(link);
  // The six words of the link: the same as Alice's.
  await expect(page.getByText(/Vérifie ces six mots/)).toBeVisible();
  expect(await sixWords(page)).toEqual(six);
  await page.getByLabel('Nom de l’équipe sur cet appareil').fill('ACME');
  await shoot(page, '07-rejoindre');
  await page.getByRole('button', { name: 'Rejoindre' }).click();
  await expect(page.getByRole('dialog').getByRole('status')).toContainText('Un administrateur en ligne te fait entrer', {
    timeout: 30_000,
  });
  await shoot(page, '08-en-attente');

  // Alice's app, online: her watch lets Bob in; her invitation says it was used.
  page = await app.restartAs(app.data);
  await openTeams(page);
  const card = page.getByRole('region', { name: 'ACME' });
  await expect(card.getByText('Utilisée')).toBeVisible({ timeout: 30_000 });
  await expect(card.getByText('2 membres')).toBeVisible({ timeout: 30_000 });
  // « Ajouter un appareil » : the QR code of an own device, then a short code.
  await card.getByRole('button', { name: 'Ajouter un appareil…' }).click();
  await expect(page.getByLabel('Le lien à envoyer')).toHaveValue(/#s=.*&m=[0-9a-f]{32}$/);
  await shoot(page, '09-appareil-qr');
  await page.getByRole('tab', { name: 'Code court' }).click();
  await page.getByRole('button', { name: 'Afficher un code' }).click();
  await expect(page.getByTestId('pair-code')).toHaveText(/^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$/);
  await expect(page.getByRole('dialog').getByRole('status')).toContainText('En attente de l’autre appareil');
  await shoot(page, '10-appareil-code');

  // Bob is in: a member, the same six words.
  page = await app.restartAs(bobData);
  await openTeams(page);
  const bobCard = page.getByRole('region', { name: 'ACME' });
  await expect(bobCard.getByText('Membre', { exact: true })).toBeVisible({ timeout: 30_000 });
  expect(await sixWords(page)).toEqual(six);
  await shoot(page, '11-membre');
  // The screens of a new device by a code, and of a recovery.
  await page.getByRole('button', { name: /Ajouter cet appareil par un code/ }).click();
  await page.getByLabel('Code de 8 caractères').fill('k7q4m2xz');
  await expect(page.getByLabel('Code de 8 caractères')).toHaveValue('K7Q4-M2XZ');
  await shoot(page, '12-code-nouvel-appareil');
  await page.getByRole('button', { name: 'Retour' }).click();
  await page.getByRole('button', { name: /Récupérer une équipe/ }).click();
  await expect(page.getByLabel(/Les six mots de l’équipe/)).toBeVisible();
  await shoot(page, '13-recuperer');
  await page.getByRole('button', { name: 'Retour' }).click();
  // The link a second time (a third data folder would be told; here, Bob himself): refused, said.
  await page.getByRole('button', { name: /Rejoindre par un lien/ }).click();
  await page.getByLabel('Lien d’invitation').fill(link);
  await page.getByRole('button', { name: 'Rejoindre' }).click();
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText('déjà dans l’équipe');

  // The keychain keeps nothing of the test: each client forgets the team.
  const forget = (p: Page) =>
    p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id });
    });
  await forget(page);

  // The alarm of a fork, as Alice's window shows it: her teams.json says it, as her watch writes it (written while her
  // app is stopped, which would write the file again).
  const file = path.join(app.data, 'teams', 'teams.json');
  const known = JSON.parse(fs.readFileSync(file, 'utf8'));
  known[0].alarm = { kind: 'fork', epoch: 1, at: Math.floor(Date.now() / 1000) };
  fs.writeFileSync(file, JSON.stringify(known));
  page = await app.restartAs(app.data);
  await openTeams(page);
  const alarm = page.getByRole('region', { name: 'ACME' }).getByRole('alert');
  await expect(alarm).toContainText('Le serveur montre deux versions de l’équipe : contacte un administrateur.');
  await expect(alarm).toContainText('ni invitation');
  await expect(page.getByRole('button', { name: 'Inviter…' })).toHaveCount(0);
  await shoot(page, '14-alerte-fourche');
  await forget(page);
});
