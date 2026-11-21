// The captures of ESC-561 in the real app, in both themes, at 1440×900 and 1024×728: Tous les projets › Git (a value
// brought back to its bound, said under its field), a project's own fetch in its Git page, and the Git panel's line under
// « Fetch » (a repository that asks for credentials, then fetched). They go to design/auto-fetch/captures/.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from '../../../tests/e2e/fixture';

// A minute of the interval is a second: the background fetch runs within the capture.
test.use({ appEnv: { ESCOUADE_E2E_FETCH_MINUTE_MS: '1000' } });

const OUT = path.resolve(import.meta.dirname, '..', 'captures');
const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

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

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

/** `el` in both themes, at each size, `prepare` run again after a resize. */
async function shoot(page: Page, name: string, el: () => Locator, prepare: () => Promise<void> = async () => {}) {
  for (const size of SIZES) {
    await page.setViewportSize(size);
    await prepare();
    await page.mouse.move(2, 2);
    await page.waitForTimeout(400);
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      await el().screenshot({ path: path.join(OUT, `${name}-${size.width}x${size.height}-${t}.png`) });
    }
  }
  await theme(page, 'dark');
}

test('the automatic fetch’s settings and its line in the Git panel, captured', async ({ app }) => {
  test.setTimeout(600_000);
  const { page } = app;
  fs.mkdirSync(OUT, { recursive: true });
  await page.setViewportSize(SIZES[0]);
  await addProject(page, app.repo, { firstAgent: false, name: 'escouade' });
  await still(page);
  // A remote over ssh that refuses the key, through the user's own ssh (`core.sshCommand`).
  const bare = `${app.repo}-remote.git`;
  git(app.repo, 'init', '-q', '--bare', '-b', 'main', bare);
  git(app.repo, 'remote', 'add', 'origin', bare);
  git(app.repo, 'push', '-qu', 'origin', 'main');
  const script = path.join(path.dirname(app.repo), 'refusing-ssh.sh').replace(/\\/g, '/');
  fs.writeFileSync(script, "#!/bin/sh\necho 'git@example.invalid: Permission denied (publickey).' >&2\nexit 255\n");
  git(app.repo, 'config', 'core.sshCommand', `sh '${script}'`);
  git(app.repo, 'config', 'ssh.variant', 'ssh');
  git(app.repo, 'remote', 'set-url', 'origin', 'ssh://git@example.invalid/escouade.git');

  // The Git panel: never fetched, it asks for credentials.
  const branch = page.locator('footer .sync');
  const picker = page.getByRole('dialog', { name: 'Branches' });
  const openPicker = async () => {
    if (!(await picker.isVisible())) await branch.click();
    await expect(picker.getByTestId('fetch-line')).toContainText('il demande des identifiants', { timeout: 60_000 });
  };
  await openPicker();
  await shoot(page, 'panneau-identifiants', () => picker, openPicker);
  await page.keyboard.press('Escape');

  // The user's credentials given (the remote reachable again), « Fetch » by hand: fetched, and the background goes on.
  git(app.repo, 'config', '--unset', 'core.sshCommand');
  git(app.repo, 'remote', 'set-url', 'origin', bare);
  await branch.click();
  await picker.getByRole('button', { name: /^Fetch/ }).click();
  const fetched = async () => {
    if (!(await picker.isVisible())) await branch.click();
    await expect(picker.getByTestId('fetch-line')).toContainText(/Récupéré/);
  };
  await fetched();
  await shoot(page, 'panneau-recupere', () => picker, fetched);
  await page.keyboard.press('Escape');

  // Tous les projets › Git, a value out of its bounds brought back.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Tous les projets');
  const every = settings.getByRole('textbox', { name: 'Intervalle', exact: true });
  await every.fill('5000');
  await every.press('Enter');
  await expect(settings.getByRole('status')).toContainText('ramené à 24 h');
  await shoot(page, 'reglages-tous-les-projets', () => settings, async () => {
    await every.scrollIntoViewIfNeeded();
  });
  await settings.getByRole('group', { name: 'Intervalle', exact: true }).getByRole('button', { name: '5 min', exact: true }).click();

  // The project's own, in its Git page: always, every 30 min.
  await openSettingsPage(settings, 'Git et worktrees', { project: 'escouade' });
  await settings
    .getByRole('group', { name: 'Récupérer automatiquement (git fetch)' })
    .getByRole('button', { name: 'Toujours' })
    .click();
  await settings.getByRole('group', { name: 'Intervalle propre au projet' }).getByRole('button', { name: '30 min', exact: true }).click();
  const own = settings.getByRole('textbox', { name: 'Intervalle propre au projet' });
  await expect(own).toHaveValue('30');
  await shoot(page, 'reglages-projet', () => settings, async () => {
    await own.scrollIntoViewIfNeeded();
  });
  await closeSettings(settings);
});
