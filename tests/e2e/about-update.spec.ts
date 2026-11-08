// An update found by « Rechercher une mise à jour » shows in Réglages › Général › Mises à jour (ESC-250, R3), not in the status bar alone:
// downloading, then ready with « Redémarrer maintenant » and its notes. The backend's answers to the check and the download
// are given in the page, on the `fetch` of Tauri's IPC (`page.route` misses the WebView2's IPC requests from time to time):
// no release is looked for, nothing is downloaded. ESCOUADE_E2E_SHOTS names a folder for the screenshots of each state,
// in both themes.
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, test, openSettingsPage, settingsEntry } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** Answers update_check with an update found, and holds update_download until `release()`. */
async function fakeUpdate(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { esc250Release: () => void; esc250Restarts: number };
    let release!: () => void;
    const downloaded = new Promise<void>((r) => (release = r));
    w.esc250Release = release;
    w.esc250Restarts = 0;
    const answer = (body: unknown) =>
      new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } });
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.endsWith('/update_check')) {
        return Promise.resolve(
          answer({ kind: 'found', update: { id: 7, version: '9.9.0', notes: '- Une mise à jour trouvée se voit dans À propos' } }),
        );
      }
      if (command.endsWith('/update_download')) return downloaded.then(() => answer({ kind: 'ready' }));
      if (command.endsWith('/update_close')) return Promise.resolve(answer(null));
      if (command.endsWith('/update_restart')) {
        w.esc250Restarts++;
        return Promise.resolve(answer(null));
      }
      return fetch(input, init);
    };
  });
}

/** The settings, light then dark, into the folder of the screenshots. */
async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  for (const theme of ['dark', 'light']) {
    await page.evaluate((th) => (document.documentElement.dataset.theme = th), theme);
    await dialog.screenshot({ path: path.join(SHOTS, `${name}-${theme}.png`), animations: 'disabled' });
  }
  await page.evaluate(() => delete document.documentElement.dataset.theme);
}

test('an update found from Général shows there, downloading then ready, its restart through the update’s window', async ({ app }) => {
  const { page } = app;
  await fakeUpdate(page);
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  await dialog.getByRole('button', { name: 'Rechercher une mise à jour' }).click();

  // The row of the update, and where it stands.
  const row = dialog.locator('.sr', { has: page.locator('[data-update]') });
  const state = row.locator('[data-update]');
  await expect(state).toHaveAttribute('data-update', 'downloading');
  await expect(row).toContainText('Escouade 9.9.0 disponible');
  await expect(row).toContainText('Téléchargement…');
  await shoot(page, 'downloading');

  await page.evaluate(() => (window as unknown as { esc250Release: () => void }).esc250Release());
  await expect(state).toHaveAttribute('data-update', 'ready');
  await expect(row).toContainText('Escouade 9.9.0 prête');
  await row.getByRole('button', { name: 'Voir les nouveautés' }).click();
  await expect(row.getByText('Une mise à jour trouvée se voit dans À propos')).toBeVisible();
  await shoot(page, 'ready');

  // As the status bar's: the update's window, which says what a restart cuts; « Plus tard » comes back to Général.
  await row.getByRole('button', { name: 'Redémarrer maintenant' }).click();
  const upd = page.getByRole('dialog', { name: 'Escouade 9.9.0 est prête' });
  await expect(upd).toBeVisible();
  await upd.getByRole('button', { name: 'Plus tard' }).click();
  await expect(settingsEntry(dialog, 'Général')).toHaveAttribute('aria-current', 'page');
  await expect(dialog.locator('[data-update="ready"]')).toBeVisible();
  expect(await page.evaluate(() => (window as unknown as { esc250Restarts: number }).esc250Restarts)).toBe(0);
});
