// « Ouvrir à son lancement » (ESC-560): a launch command with an address opens it by itself once it answers, as « ▶ Tester »
// does (the same bounded wait, no fixed delay): run in the project, in a tab of the project's « Pages » window; run in an
// agent's worktree, the address follows the agent's ports and opens in its view « Page ». A command that crashes before
// answering opens nothing.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test, unfoldSections } from './fixture';

/** A port nothing listens on now (the app's own blocks start at 4100). */
function freePort(): Promise<number> {
  return new Promise((done) => {
    const s = net.createServer().listen(0, '127.0.0.1', () => {
      const port = (s.address() as net.AddressInfo).port;
      s.close(() => done(port));
    });
  });
}

/** The developed app: it serves its page a little after it starts, on the worktree's first port or the one given. */
const SERVER = `const port = Number(process.env.ESCOUADE_PORT_BASE || process.argv[2]);
setTimeout(() => {
  require('node:http')
    .createServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      res.end('<!doctype html><title>accueil</title><h1>Accueil</h1>');
    })
    .listen(port, 'localhost');
}, 1500);
setInterval(() => {}, 1000);
`;

/** The pages open, as the backend keeps them (the window's own command). */
const pagesOpen = (page: Page, projectId?: string) =>
  page.evaluate(
    (p) =>
      (
        window as unknown as {
          __TAURI_INTERNALS__: {
            invoke: (c: string, a: unknown) => Promise<{ pages: { agentId: string; url: string; title: string; agentless?: boolean }[] }>;
          };
        }
      ).__TAURI_INTERNALS__.invoke('devpage_list', p ? { projectId: p } : {}),
    projectId,
  );

/** « Commande n » of the launch page of the settings, filled. */
async function addCommand(page: Page, n: number, name: string, command: string, url: string) {
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const cfg = dialog.getByRole('group', { name: `Commande ${n}` });
  await cfg.getByLabel('Nom').fill(name);
  await cfg.getByLabel('Commande', { exact: true }).fill(command);
  await cfg.getByRole('checkbox', { name: 'Ouvrir à son lancement' }).check();
  await cfg.getByLabel('Adresse à ouvrir').fill(url);
  await expect(cfg.getByLabel('Adresse à ouvrir')).toHaveAttribute('aria-invalid', 'false');
}

/** The captures of the window, when ESCOUADE_E2E_SHOTS is set: both themes, at 1440 × 900 and 1024 × 728. */
async function shots(page: Page, name: string) {
  const dir = process.env.ESCOUADE_E2E_SHOTS;
  if (!dir) return;
  const was = page.viewportSize();
  for (const size of [
    { width: 1440, height: 900 },
    { width: 1024, height: 728 },
  ])
    for (const t of ['light', 'dark'] as const) {
      await page.setViewportSize(size);
      await page.evaluate((t) => {
        document.documentElement.dataset.theme = t;
        document.documentElement.style.colorScheme = t;
      }, t);
      await page.evaluate(() =>
        Promise.all(
          document
            .getAnimations()
            .filter((a) => a.effect?.getTiming().iterations !== Infinity)
            .map((a) => a.finished.catch(() => null)),
        ),
      );
      await page.screenshot({ path: path.join(dir, `${name}-${size.width}x${size.height}-${t}.png`) });
    }
  if (was) await page.setViewportSize(was);
}

const row = (page: Page, name: string) => page.locator('.run').filter({ has: page.locator('.name', { hasText: new RegExp(`^${name}$`) }) });

test('a launch command in the project opens its address in the « Pages » window once it answers, and one that crashes opens nothing', async ({
  app,
}) => {
  const { page } = app;
  const port = await freePort();
  const dead = await freePort();
  test.setTimeout(240_000);
  fs.writeFileSync(path.join(app.repo, 'serveur.js'), SERVER);
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
  await addProject(page, app.repo);

  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  // A scheme other than http(s) is refused as it is typed.
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const first = dialog.getByRole('group', { name: 'Commande 1' });
  await first.getByRole('checkbox', { name: 'Ouvrir à son lancement' }).check();
  await first.getByLabel('Adresse à ouvrir').fill('file:///C:/Windows/win.ini');
  await expect(first.getByLabel('Adresse à ouvrir')).toHaveAttribute('aria-invalid', 'true');
  await expect(first.getByText('Seule une adresse http:// ou https:// s’ouvre.')).toBeVisible();
  await first.getByRole('button', { name: 'Supprimer' }).click();
  await addCommand(page, 1, 'Site', `node serveur.js ${port}`, `http://localhost:\${ESCOUADE_PORT_BASE:-${port}}/accueil`);
  await addCommand(page, 2, 'Casse', 'node -e "setTimeout(() => process.exit(3), 800)"', `http://localhost:${dead}/`);
  await closeSettings(dialog);
  await unfoldSections(page);

  // The one that crashes before answering: nothing opens.
  // From its log, whatever the width of the bar.
  await row(page, 'Casse').locator('.name').click();
  await page.locator('main.rv').getByRole('button', { name: 'Lancer', exact: true }).first().click();
  await expect(row(page, 'Casse')).toContainText('planté (code 3)', { timeout: 30_000 });

  // The other: once it answers (it waits 1.5 s before serving), its page opens by itself in the project's window.
  // From its log, whatever the width of the bar.
  await row(page, 'Site').locator('.name').click();
  await page.locator('main.rv').getByRole('button', { name: 'Lancer', exact: true }).first().click();
  await expect(row(page, 'Site')).toContainText('en cours');
  await expect
    .poll(async () => (await pagesOpen(page)).pages.map((p) => [p.title, p.url, !!p.agentless]), { timeout: 60_000 })
    .toEqual([['Site', `http://localhost:${port}/accueil`, true]]);

  // Started again: the same page, not a second one.
  await row(page, 'Site').click();
  await page.locator('main.rv').getByRole('button', { name: 'Relancer' }).click();
  await expect(row(page, 'Site')).toContainText('en cours');
  await expect.poll(async () => (await pagesOpen(page)).pages.length, { timeout: 30_000 }).toBe(1);

  // The window, for the captures.
  await openSettingsPage(page, 'Lancement', { project: 'demo-api' });
  await expect(page.getByRole('dialog', { name: 'Réglages' }).getByLabel('Adresse à ouvrir').first()).toBeVisible();
  await shots(page, 'launch-url');
});

test('a launch command in an agent’s worktree opens its address on the agent’s ports, in its view « Page »', async ({ app }) => {
  const { page } = app;
  const fallback = await freePort();
  test.setTimeout(240_000);
  fs.writeFileSync(path.join(app.repo, 'serveur.js'), SERVER);
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
  await addProject(page, app.repo, { worktrees: true });
  const agentName = (await page.locator('.card .name').first().textContent())!.trim();

  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await addCommand(page, 1, 'Site', `node serveur.js ${fallback}`, `http://localhost:\${ESCOUADE_PORT_BASE:-${fallback}}/accueil`);
  await closeSettings(dialog);
  await unfoldSections(page);

  await row(page, 'Site').hover();
  await row(page, 'Site').getByRole('button', { name: 'Lancer' }).click();
  await page.getByRole('menuitem', { name: `Dans le worktree de ${agentName}` }).click();
  await expect(page.locator('.run', { hasText: `Site · ${agentName}` })).toContainText('en cours');

  // Its page opens by itself, in the agent, on the agent's first port (not the project's).
  await expect
    .poll(async () => (await pagesOpen(page)).pages.map((p) => [p.title, p.url]), { timeout: 60_000 })
    .toEqual([[`${agentName} · Site`, expect.stringMatching(/^http:\/\/localhost:41\d0\/accueil$/)]]);
  await page.locator('.card').first().click();
  await expect(page.getByRole('button', { name: 'La page de son lancement, dans Escouade' })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('textbox', { name: 'Adresse de la page' })).toHaveValue(/^http:\/\/localhost:41\d0\/accueil$/);
});
