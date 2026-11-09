// The captures « après » of ESC-390, in the real app with the tooling of the e2e tests, against the fake GitHub of
// tests/fixtures/fake-trackers.mjs: `npx playwright test -c design/kanban-envs-2-2/capture/playwright.config.ts`
// (ESCOUADE_E2E_EXE for a debug build made elsewhere). They go to design/kanban-envs-2-2/apres/, in both themes: the
// sidebar's tabs with and without a code host, the Kanban's header at three widths, the view « Environnements ».

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from '../../../tests/e2e/fixture';
import { startFakeTrackers } from '../../../tests/fixtures/fake-trackers.mjs';

const OUT = path.resolve(import.meta.dirname, '..', 'apres');
let github: { url: string; close: () => Promise<void> };
const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const post = (route: string, body: unknown) => fetch(`${github.url}${route}`, { method: 'POST', body: JSON.stringify(body) });

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 7 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
});
test.afterAll(async () => {
  await github?.close();
});

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

const still = (page: Page) =>
  page.evaluate(() => {
    if (document.getElementById('still')) return;
    const style = document.createElement('style');
    style.id = 'still';
    style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
    document.head.append(style);
  });

async function shoot(page: Page, name: string, el: ReturnType<Page['locator']>) {
  await page.mouse.move(2, 600);
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.waitForTimeout(300);
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await el.screenshot({ path: path.join(OUT, `${name}${t === 'light' ? '-light' : ''}.png`) });
  }
  await theme(page, 'dark');
}

test('the tabs, the Kanban’s header and the environments, captured', async ({ app }) => {
  test.setTimeout(600_000);
  const { page } = app;
  fs.mkdirSync(OUT, { recursive: true });
  await page.setViewportSize({ width: 1440, height: 900 });
  await addProject(page, app.repo, { firstAgent: false, name: 'escouade' });
  await still(page);
  // Without a code host: three tabs.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.locator('.side .seg button')).toHaveCount(3);
  await shoot(page, 'onglets-sans-hebergeur', page.locator('aside.side .seg'));

  // On GitHub: two tickets done and merged, main pushed and deployed.
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', '-b', 'main', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  const before = git(app.repo, 'rev-parse', 'HEAD');
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await form.getByLabel(/Jeton/).fill('ghp_jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
  await closeSettings(settings);
  // Back from the terminal where the remote was added: the window comes back to the front.
  await page.evaluate(() => window.dispatchEvent(new Event('focus')));
  await expect(page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true })).toBeVisible({ timeout: 30_000 });

  const col = (name: string) => page.getByRole('region', { name });
  for (const title of [
    'Réglages R8 — la recherche dans les réglages et les points ambre [ok]',
    'Antigravity 2.2.4 : une écriture qui nomme un sous-dossier [ok]',
  ]) {
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
    await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
    await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
    await col('À tester').getByRole('button', { name: /^Valider et merger/ }).first().click({ timeout: 120_000 });
    await expect(col('À tester').locator('.card')).toHaveCount(0, { timeout: 60_000 });
  }
  const first = git(app.repo, 'rev-parse', 'HEAD~1');
  const head = git(app.repo, 'rev-parse', 'HEAD');
  git(app.repo, 'push', '-q', 'origin', 'main');
  await post('/__github/deployments', [
    { environment: 'github-pages', sha: head, state: 'in_progress' },
    { environment: 'production', sha: head, state: 'failure', log: 'https://github.com/acme/demo/actions/runs/9/job/90' },
    { environment: 'staging', sha: head, state: 'success', url: 'https://staging.escouade.dev', author: 'ada' },
    { environment: 'npm', sha: head, state: 'success', author: 'ada' },
    { environment: 'production', sha: first, state: 'success', url: 'https://escouade.dev', author: 'grace' },
  ]);
  void before;

  // The tabs with a code host, Kanban open with two tickets to read.
  await shoot(page, 'onglets', page.locator('aside.side .seg'));
  // The header at three widths.
  for (const width of [1440, 1250, 1110]) {
    await page.setViewportSize({ width, height: 900 });
    await shoot(page, `en-tete-${width}`, page.locator('main.board > .head'));
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  // The view « Environnements »: github-pages left out, npm hidden from its menu, the menu open.
  await page.getByRole('button', { name: 'Environnements' }).click();
  await page.getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(page.locator('[data-env-lane="staging"]')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: /environnements? sur/ }).click();
  await page.getByRole('dialog', { name: 'Environnements montrés' }).getByRole('checkbox', { name: 'npm' }).click();
  await shoot(page, 'environnements-menu', page.locator('main.board'));
  await page.keyboard.press('Escape');
  await shoot(page, 'environnements', page.locator('main.board'));
  await shoot(page, 'fenetre', page.locator('body'));
});
