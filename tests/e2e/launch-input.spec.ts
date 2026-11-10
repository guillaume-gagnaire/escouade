// ESC-396: a launch command's view takes the keyboard while it runs (an answer to its prompt, Ctrl+C), as a terminal of
// the sidebar does; once it ended, its log is read-only again. Run at the CI's 1024 × 728 too (ESCOUADE_E2E_VIEWPORT).

import fs from 'node:fs';
import path from 'node:path';
import { addProject, closeSettings, expect, settingsEntry, test, unfoldSections } from './fixture';

test('a launch command reads the answer typed in its view, and stops on Ctrl+C', async ({ app }) => {
  const { page } = app;
  const ready = path.join(app.repo, 'pret.txt');
  const answer = path.join(app.repo, 'reponse.txt');
  await addProject(page, app.repo);

  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await expect(settingsEntry(dialog, 'Lancement')).toHaveAttribute('aria-current', 'page');
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const cfg = dialog.getByRole('group', { name: 'Commande 1' });
  await cfg.getByLabel('Nom').fill('Question');
  // It asks, writes what it was answered, then runs until it is stopped.
  await cfg
    .getByLabel('Commande')
    .fill(
      `Set-Content pret.txt ok; $r = Read-Host 'Ton prenom'; Set-Content reponse.txt "bonjour-$r"; while ($true) { Start-Sleep -Milliseconds 200 }`,
    );
  await closeSettings(dialog);
  await unfoldSections(page);
  const row = page.locator('.run', { hasText: 'Question' });
  await page.getByRole('button', { name: 'Tout lancer', exact: true }).click();
  await expect(row).toContainText('en cours');

  // Its view: what is typed in it reaches the prompt.
  await row.click();
  const view = page.locator('main.rv');
  await expect(view.locator('.xterm')).toBeVisible();
  await expect.poll(() => fs.existsSync(ready), { timeout: 30_000 }).toBe(true);
  // Read-Host is up a moment after the file is written.
  await page.waitForTimeout(1000);
  await view.locator('.xterm').click();
  await page.keyboard.type('Escouade');
  await page.keyboard.press('Enter');
  await expect
    .poll(() => (fs.existsSync(answer) ? fs.readFileSync(answer, 'utf8').trim() : ''), { timeout: 20_000 })
    .toBe('bonjour-Escouade');
  await expect(row).toContainText('en cours');

  // Ctrl+C, nothing selected, goes to the program, which stops.
  await view.locator('.xterm').click();
  await page.keyboard.press('Control+C');
  // The user stopped it: « arrêté », as with « Stopper », not a crash.
  await expect(view.locator('.state')).toHaveText('arrêté', { timeout: 20_000 });
  await expect(page.getByText("« Question » s'est arrêté en erreur")).toHaveCount(0);
  await expect(view.getByRole('button', { name: 'Relancer' })).toBeVisible();
});
