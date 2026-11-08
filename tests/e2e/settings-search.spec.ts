// The search of the settings window (R8 of the 2.2 settings, ESC-386; design/settings-2-2/PROPOSITION.md §2 « La
// recherche »): Ctrl+F in the window, a word typed, a hit opens its page with « Avancé » unfolded when the row is there,
// the row lit. « proxy » is Avancé › Réseau, « bêta » is Général; « editeur » is in Général's « Avancé ».

import { expect, openSettingsPage, settingsEntry, test } from './fixture';

test('« proxy » opens Avancé › Réseau, its row lit', async ({ app }) => {
  const dialog = await openSettingsPage(app.page, 'Général');
  await app.page.keyboard.press('Control+F');
  const field = dialog.getByRole('searchbox', { name: 'Rechercher un réglage' });
  await expect(field).toBeFocused();
  await field.fill('PROXY');
  const hits = dialog.getByRole('listbox', { name: 'Réglages trouvés' }).getByRole('option');
  await expect(hits.first()).toContainText('Proxy HTTP(S)');
  await expect(hits.first()).toContainText('Avancé');
  await app.page.keyboard.press('Enter');
  await expect(dialog.getByRole('heading', { level: 1 })).toHaveText('Avancé');
  // Lit for a moment, in its group Réseau, in view.
  const lit = dialog
    .locator('#settings-panel section')
    .filter({ has: app.page.getByRole('heading', { name: 'Réseau' }) })
    .locator('.s-found');
  await expect(lit).toContainText('Proxy HTTP(S)');
  await expect(lit).toBeInViewport();
  // Escape empties the field and gives the pages back; a second closes the window.
  await app.page.keyboard.press('Escape');
  await expect(field).toHaveValue('');
  await expect(settingsEntry(dialog, 'Avancé')).toHaveAttribute('aria-current', 'page');
  await app.page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
});

test('« bêta » opens Général, without its accent too', async ({ app }) => {
  const dialog = await openSettingsPage(app.page, 'Avancé');
  const field = dialog.getByRole('searchbox', { name: 'Rechercher un réglage' });
  for (const q of ['bêta', 'beta']) {
    await field.fill(q);
    await dialog.getByRole('option').first().click();
    await expect(dialog.getByRole('heading', { level: 1 })).toHaveText('Général');
    await expect(dialog.locator('#settings-panel .s-found')).toContainText('Recevoir les versions bêta');
  }
});

test('a row of « Avancé » unfolds it: « éditeur » in Général', async ({ app }) => {
  const dialog = await openSettingsPage(app.page, 'Agents');
  await dialog.getByRole('searchbox', { name: 'Rechercher un réglage' }).fill('editeur');
  const hit = dialog.getByRole('option').filter({ hasText: 'Taille du texte de l’éditeur' });
  await expect(hit).toContainText('Général › Avancé');
  await hit.click();
  const fold = dialog.locator('#settings-panel').getByRole('button', { name: /^Avancé/ });
  await expect(fold).toHaveAttribute('aria-expanded', 'true');
  await expect(dialog.locator('#settings-panel .s-found')).toContainText('Taille du texte de l’éditeur');
});
