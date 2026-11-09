import { addProject, expect, openSettingsPage, test } from './fixture';

// ESC-380 (R2b of the 2.2 redesign): every setting is saved as it changes, with no « Annuler » nor « Enregistrer » —
// a choice at once, a text when it loses the focus or after a pause in the typing; a value that cannot be is said on its
// row and not saved. The window is closed at once, without a question, and the app started again finds it all.

test('a setting changed, the window closed at once, the app started again: it is still there', async ({ app }) => {
  let page = app.page;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  let dialog = page.getByRole('dialog', { name: 'Réglages' });
  // No foot: nothing to confirm, nothing to cancel.
  await expect(dialog.getByRole('button', { name: 'Enregistrer', exact: true })).toHaveCount(0);
  await expect(dialog.getByRole('button', { name: 'Annuler', exact: true })).toHaveCount(0);

  // A switch: saved at once.
  await openSettingsPage(dialog, 'Notifications');
  const sound = dialog.getByRole('switch', { name: 'Son activé' });
  const was = await sound.getAttribute('aria-checked');
  const now = was === 'true' ? 'false' : 'true';
  await sound.click();
  await expect(sound).toHaveAttribute('aria-checked', now);

  // A number that cannot be: said on its row, the one saved before kept.
  await openSettingsPage(dialog, 'Agents');
  await dialog
    .locator('#settings-panel')
    .getByRole('button', { name: /^Avancé/ })
    .click();
  const idle = dialog.getByRole('spinbutton', { name: /Arrêter les processus des agents inactifs/ });
  const before = await idle.inputValue();
  await idle.fill('');
  await idle.blur();
  await expect(idle).toHaveAttribute('aria-invalid', 'true');
  await expect(dialog.getByText(`Un nombre entier, 0 ou plus : ${before} reste enregistré.`)).toBeVisible();

  // A text of a project, typed, then the window closed at once with Échap, the field never left.
  await openSettingsPage(dialog, 'Projet');
  const name = dialog.getByRole('textbox', { name: 'Nom du projet' });
  await name.fill('demo-api-renommé');
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  // No question on the way out.
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('navigation', { name: 'Projets' }).getByRole('tab', { name: /demo-api-renommé/ })).toBeVisible();

  page = await app.restart();
  await expect(page.getByRole('navigation', { name: 'Projets' }).getByRole('tab', { name: /demo-api-renommé/ })).toBeVisible();
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Notifications');
  await expect(dialog.getByRole('switch', { name: 'Son activé' })).toHaveAttribute('aria-checked', now);
  await openSettingsPage(dialog, 'Projet');
  await expect(dialog.getByRole('textbox', { name: 'Nom du projet' })).toHaveValue('demo-api-renommé');
  await openSettingsPage(dialog, 'Agents');
  await dialog
    .locator('#settings-panel')
    .getByRole('button', { name: /^Avancé/ })
    .click();
  await expect(dialog.getByRole('spinbutton', { name: /Arrêter les processus des agents inactifs/ })).toHaveValue(before);
});

test('a text is saved after a pause in the typing, its field still focused', async ({ app }) => {
  let page = app.page;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Projet');
  const name = dialog.getByRole('textbox', { name: 'Nom du projet' });
  await name.fill('');
  await name.pressSequentially('api-v2');
  await expect(name).toBeFocused();
  // Saved while it is still being edited: the project's tile on the rail takes its name.
  await expect(page.getByRole('navigation', { name: 'Projets' }).getByRole('tab', { name: /api-v2/ })).toBeVisible();
  await expect(dialog).toHaveAttribute('data-saving', 'false');
  await expect(name).toBeFocused();
  page = await app.restart();
  await expect(page.getByRole('navigation', { name: 'Projets' }).getByRole('tab', { name: /api-v2/ })).toBeVisible();
});
