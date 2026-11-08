import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, send, test, openSettingsPage, settingsEntry, closeSettings } from './fixture';

// The app started in English: its words, then French at once when the setting changes, with no restart.
test.use({ language: 'en' });

test('the interface in English, then in French without a restart', async ({ app }) => {
  const { page } = app;
  const sidebar = page.locator('aside.side');
  const footer = page.locator('footer');

  // The welcome screen, before any project.
  await expect(page.getByText(/^Add a project to run agents, follow their questions, and open terminals\./)).toBeVisible();

  // A project with one agent, which answers.
  await addProject(page, app.repo, { lang: 'en' });
  await expect(sidebar.getByRole('button', { name: 'New agent' })).toBeVisible();
  await expect(page.getByText('Agent ready')).toBeVisible();
  await send(page, 'Hello');
  await expect(page.getByText('Bonjour, tu as dit : Hello')).toBeVisible();
  // The fake CLI answers « Tâche terminée » whatever the language, and names the agent after its first request: in
  // English, because Claude writes in the language of the interface.
  await expect(page.locator('.card .name').first()).toHaveText('hello-fake-en');

  // Sidebar, composer and status bar.
  await expect(sidebar).toContainText('Terminals');
  await expect(sidebar).toContainText('Done');
  await expect(page.getByRole('textbox').last()).toHaveAttribute('placeholder', 'Send a message to hello-fake-en…');
  await expect(page.getByRole('button', { name: 'Send' })).toBeVisible();
  await expect(footer).toContainText('1 done');
  await expect(footer).toContainText('5h');
  const quota = page.getByRole('meter', { name: '5-hour quota' });
  await expect(quota).toHaveAttribute('aria-valuenow', '12');
  await expect(page.getByRole('meter', { name: '7-day quota' })).toBeVisible();
  // The percentage is in a tooltip over the bar. The processes' item comes with the first sample of the resources (every
  // 2 s) and pushes the quota 190 px to the right: hovered before it, the pointer would rest on it, and show its tooltip.
  await expect(footer).toContainText('1 agent ·');
  await quota.hover();
  await expect(page.getByRole('tooltip')).toContainText(/^12%/);

  // The settings, in English, on the tab of the languages.
  await page.getByRole('button', { name: 'Settings (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Settings' });
  await openSettingsPage(dialog, 'General');
  await expect(settingsEntry(dialog, 'General')).toHaveAttribute('aria-current', 'page');
  const ui = dialog.getByRole('group', { name: 'Interface language' });
  await expect(ui.getByRole('button', { name: 'English', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await expect(dialog.getByRole('group', { name: 'Language of texts written by the agents' })).toBeVisible();

  // « Français »: saved, and the same words are French at once.
  await ui.getByRole('button', { name: 'Français', exact: true }).click();
  // Saved at once: the window is « Réglages » already.
  await closeSettings(page.getByRole('dialog', { name: 'Réglages' }));
  await expect(sidebar.getByRole('button', { name: 'Nouvel agent' })).toBeVisible();
  await expect(sidebar).toContainText('Terminaux');
  await expect(sidebar).toContainText('Terminé');
  await expect(page.getByRole('textbox').last()).toHaveAttribute('placeholder', 'Envoyer un message à hello-fake-en…');
  await expect(page.getByRole('button', { name: 'Envoyer' })).toBeVisible();
  await expect(footer).toContainText('1 terminé');
  await expect(footer).toContainText('5 h');
  await expect(page.getByRole('meter', { name: 'Quota sur 5 heures' })).toHaveAttribute('aria-valuenow', '12');
  await expect(page.getByRole('meter', { name: 'Quota sur 7 jours' })).toBeVisible();
  expect(JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')).language).toBe('fr');

  // What Claude said stays what it was; the settings are French too.
  await expect(page.getByText('Bonjour, tu as dit : Hello')).toBeVisible();
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const reglages = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(reglages, 'Général');
  await expect(
    reglages.getByRole('group', { name: 'Langue de l’interface' }).getByRole('button', { name: 'Français', exact: true }),
  ).toHaveAttribute('aria-pressed', 'true');
  await page.keyboard.press('Escape');
  await expect(reglages).toBeHidden();

  // Claude writes in French now too: the next agent is named in French (the fake CLI marks an English name with « -en »).
  await page.keyboard.press('Control+n');
  // The new agent first: a message sent before its composer is there would go to the old one.
  await expect(page.locator('.card')).toHaveCount(2);
  await expect(page.getByText('Agent prêt')).toBeVisible();
  await send(page, 'Bonjour');
  await expect(page.locator('.card .name').filter({ hasText: /^bonjour-fake$/ })).toHaveCount(1);

  // The menu and the tooltip of the icon in the notification area are native: Playwright cannot read them. Their words
  // in each language, and their being made again when the language changes, are tested in the backend (menus.rs).
});

test('a superpowers plan run reads in English, and in French as soon as the language changes', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { lang: 'en' });
  await send(page, 'plan superpowers');

  // The third task waits for an answer: the banner, its list and the agent's card, in English.
  const card = page.getByTestId('question-pending');
  await expect(card).toContainText('Quelle base de données ?');
  const plan = page.locator('main.conv .plan');
  const head = plan.getByRole('button', { name: 'Plan: Démo' });
  await expect(head).toBeVisible();
  await expect(plan).toContainText('2/3 tasks');
  await expect(plan).toContainText('67%');
  await expect(plan).toContainText('1 subagent running');
  if ((await head.getAttribute('aria-expanded')) !== 'true') await head.click();
  const third = plan.locator('li[data-plan-task="03"]');
  await expect(plan.getByRole('list', { name: 'Plan tasks' })).toBeVisible();
  await expect(third).toContainText('Blocked');
  await expect(third).toContainText('Waiting for your answer');
  await expect(plan.getByRole('separator', { name: 'Height of the task list' })).toBeVisible();
  await expect(plan.locator('li[data-plan-task="01"]')).toContainText('Done');
  await expect(page.locator('.card .plan-mini .plan-text').first()).toHaveText('Plan 2/3 · 1 subagent');

  // Switched to French while the banner is open: its words change at once, nothing is read again.
  await page.getByRole('button', { name: 'Settings (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Settings' });
  await openSettingsPage(dialog, 'General');
  await dialog.getByRole('group', { name: 'Interface language' }).getByRole('button', { name: 'Français', exact: true }).click();
  // Saved at once: the window is « Réglages » already.
  await closeSettings(page.getByRole('dialog', { name: 'Réglages' }));
  await expect(plan).toContainText('2/3 tâches');
  await expect(plan).toContainText('1 sous-agent actif');
  await expect(third).toContainText('En attente de ta réponse');
  await expect(page.locator('.card .plan-mini .plan-text').first()).toHaveText('Plan 2/3 · 1 sous-agent');

  await card.getByRole('button', { name: 'SQLite' }).click();
  await expect(page.getByText('Plan terminé')).toBeVisible();
  await expect(plan).toContainText('3/3 tâches');
  await expect(plan).toContainText('100 %');
});

test('the window of a new ticket is in English, its key hint included', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { lang: 'en', firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'New ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'New ticket' });
  await expect(dialog.getByRole('textbox', { name: 'Ticket title' })).toBeFocused();
  await expect(dialog.getByRole('textbox', { name: 'Description' })).toHaveAttribute('placeholder', 'Description (optional)');
  await expect(dialog.getByRole('textbox', { name: 'Acceptance criteria' })).toBeVisible();
  await expect(dialog.getByRole('group', { name: 'Max loops' })).toBeVisible();
  await expect(dialog.getByText('Ctrl+Enter to add')).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Add', exact: true })).toBeDisabled();
  // Something typed: leaving asks, in English.
  await dialog.getByRole('textbox', { name: 'Ticket title' }).fill('Started');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Discard your changes?' })).toBeVisible();
  await page.getByRole('dialog', { name: 'Discard your changes?' }).getByRole('button', { name: 'Discard' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});
