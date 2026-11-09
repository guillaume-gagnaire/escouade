// Each link of the app to a project's settings leads to its page (R6 of the 2.2 settings, ESC-385; PROPOSITION.md,
// annexe B): « Projets › demo-api › Lancement », no longer the middle of a scroll of the 2.0 tab « Projets ». One test
// per link: the tile's « Réglages du projet… », the side bar's « Harness par défaut du projet… », Lancement's ⚙,
// « Configurer » and « Proposer des commandes », the Kanban's « Après validation », the Workflow panel, an account
// reserved to a private project, and « Tous les projets fermés… » of the rail. (« Les lire » of the Kanban, for the
// commands of .escouade.yml, is driven by escouade-yml.spec.ts; « Lier une source » of an import by integrations.spec.ts.)

import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openAdvanced, openSettingsPage, settingsEntry, test, unfoldSections } from './fixture';

/** The settings window, open on `page` of demo-api: its entry current in the bar, its title over the page. */
async function landsOn(app: Page, page: string, title = page) {
  const dialog = app.getByRole('dialog', { name: 'Réglages' });
  await expect(dialog).toBeVisible();
  await expect(settingsEntry(dialog, page)).toHaveAttribute('aria-current', 'page');
  // The page is demo-api's: unfolded under its name in the bar.
  await expect(dialog.getByRole('navigation').getByRole('group', { name: 'demo-api' })).toBeVisible();
  await expect(dialog.getByRole('heading', { level: 1 })).toHaveText(title);
  return dialog;
}

test.beforeEach(async ({ app }) => {
  await addProject(app.page, app.repo);
  // In a short window (the CI's 1028 × 779), Lancement and Terminaux are folded, their rows (« Configurer », « Proposer
  // des commandes ») hidden.
  await unfoldSections(app.page);
});

test('the tile’s « Réglages du projet… » opens Projets › demo-api › Projet', async ({ app }) => {
  const page = app.page;
  await page
    .getByRole('navigation', { name: 'Projets' })
    .getByRole('tab', { name: /demo-api/ })
    .click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Réglages du projet…' }).click();
  const dialog = await landsOn(page, 'Projet', 'demo-api');
  await expect(dialog.getByRole('textbox', { name: 'Nom du projet' })).toHaveValue('demo-api');
});

test('the side bar’s « Harness par défaut du projet… » opens Projets › demo-api › Projet', async ({ app }) => {
  const page = app.page;
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page
    .getByRole('menu')
    .getByRole('menuitem', { name: /Harness par défaut du projet/ })
    .click();
  const dialog = await landsOn(page, 'Projet', 'demo-api');
  await expect(dialog.getByRole('heading', { name: 'Identité' })).toBeVisible();
});

test('Lancement’s ⚙ and « Configurer » open Projets › demo-api › Lancement', async ({ app }) => {
  const page = app.page;
  const runs = page.getByRole('region', { name: 'Lancement' });
  await runs.getByRole('button', { name: 'Configurer' }).click();
  let dialog = await landsOn(page, 'Lancement');
  await expect(dialog.getByRole('button', { name: 'Ajouter une commande' })).toBeVisible();
  // The page starts at its top: its group is the first thing seen.
  await expect(dialog.locator('#settings-panel')).toHaveJSProperty('scrollTop', 0);
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await runs.getByRole('button', { name: 'Commandes de lancement…' }).click();
  dialog = await landsOn(page, 'Lancement');
  await expect(dialog.getByText('Aucune commande pour l’instant.').or(dialog.getByText("Aucune commande pour l'instant."))).toBeVisible();
});

test('Lancement’s « Proposer des commandes » opens Projets › demo-api › Lancement, Claude reading the project', async ({ app }) => {
  const page = app.page;
  await page.getByRole('region', { name: 'Lancement' }).getByRole('button', { name: 'Proposer des commandes' }).click();
  const dialog = await landsOn(page, 'Lancement');
  // Asked at once: the reading's button says it, on this page (not the worktrees', on « Git et worktrees »).
  await expect(
    dialog.locator('#settings-panel').getByRole('button', { name: /^(Claude lit le projet…|Remplir automatiquement)$/ }),
  ).toHaveCount(1);
  await expect(dialog.getByRole('heading', { name: 'Worktrees' })).toHaveCount(0);
});

test('the Kanban’s « Après validation » opens Projets › demo-api › Kanban', async ({ app }) => {
  const page = app.page;
  // At the runner's 1024 × 728, the board is under 1000 px: « Après validation : » is no longer shown, only its summary,
  // and the button keeps its name, in view (ESC-393).
  await page.setViewportSize({ width: 1024, height: 728 });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const button = page.getByRole('button', { name: /^Après validation :/ });
  await expect(button).toBeInViewport({ ratio: 1 });
  await expect(button.getByText('Après validation :')).toBeHidden();
  await button.click();
  const dialog = await landsOn(page, 'Kanban');
  await expect(dialog.getByRole('radiogroup', { name: /Quand je valide un ticket/ })).toBeVisible();
  // The workflow of the project is on this page now, at its foot.
  await expect(dialog.getByRole('heading', { name: 'Workflow' })).toBeAttached();
});

test('the Workflow panel opens Projets › demo-api › Kanban, on its workflow', async ({ app }) => {
  const page = app.page;
  await page.keyboard.press('Control+Shift+B');
  const side = page.locator('aside.git-side');
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  await side.getByRole('button', { name: 'Décrire un workflow…' }).click();
  const dialog = await landsOn(page, 'Kanban');
  // Scrolled to its group: « Workflow » in view, not the top of the page.
  await expect(dialog.locator('#sg-workflow')).toBeInViewport();
  await expect(dialog.locator('#settings-panel')).not.toHaveJSProperty('scrollTop', 0);
});

test('an account reserved to a private project comes back to Projets › demo-api › Confidentialité', async ({ app }) => {
  const page = app.page;
  const dialog = await openSettingsPage(page, 'Confidentialité');
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  await dialog.getByRole('button', { name: 'Ajouter un compte réservé…' }).click();
  const account = page
    .getByRole('dialog')
    .filter({ hasNot: page.locator('.s-nav') })
    .last();
  await expect(account).toBeVisible();
  await page.keyboard.press('Escape');
  const back = await landsOn(page, 'Confidentialité');
  // The draft as it was: still private.
  await expect(back.getByRole('switch', { name: 'Projet privé' })).toHaveAttribute('aria-checked', 'true');
  await closeSettings(back);
});

test('the rail’s « Tous les projets fermés… » opens Tous les projets', async ({ app }) => {
  const page = app.page;
  // A project closed with its history (from Projet › Avancé, « Zone sensible » of the 2.0 window): the menu of « + ».
  const settings = await openSettingsPage(page, 'Projet');
  await openAdvanced(settings);
  await settings.getByRole('button', { name: 'Fermer le projet…' }).click();
  const ask = page.getByRole('dialog', { name: 'Fermer « demo-api » ?' });
  await ask.getByRole('button', { name: 'Fermer le projet', exact: true }).click();
  await expect(ask).toBeHidden();
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();
  await page.getByRole('button', { name: 'Ajouter un projet', exact: true }).click();
  await page.getByRole('menuitem', { name: /Tous les projets fermés/ }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await expect(settingsEntry(dialog, 'Tous les projets')).toHaveAttribute('aria-current', 'page');
  await expect(dialog.getByRole('heading', { level: 1 })).toHaveText('Tous les projets');
  await expect(dialog.getByRole('switch', { name: 'Nouveaux projets privés' })).toBeVisible();
});
