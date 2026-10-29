// « Planifier » (ESC-262), with the fake claude: a brainstorm proposes pages and a decision, poured into the spec; then
// « Planifier » on the discussion has the planner make one ticket per page in « À faire », not started, each linked to
// its page (the ticket's card names it, the subject's card shows the keys), the ticket of the board it proposes again
// left out. « Annuler » removes them and the changes are to plan again: on top of the Specification, « 3 changements
// sans tickets · Planifier » makes them again. Nothing committed. The captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.mouse.move(0, 0);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `plan-${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

test('« Planifier » makes the tickets of the changes poured, linked to their pages, not started, and « Annuler » undoes it', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: app.repo, encoding: 'utf8' });
  const notif = path.join(app.repo, '.escouade', 'spec', 'notifications', 'general.md');
  fs.mkdirSync(path.dirname(notif), { recursive: true });
  fs.writeFileSync(
    notif,
    '---\ntitle: "Notifications"\nsubject: "Notifications"\nsources: []\ntickets: []\ndecisions: []\ncompared: null\n---\n\nAucune notification n’est planifiée.\n',
  );
  git('add', '.');
  git('commit', '-qm', 'spec');
  const head = git('rev-parse', 'HEAD').trim();
  await addProject(page, app.repo, { firstAgent: false, name: 'Portail' });

  // The board: nothing starts by itself, and a ticket the planner will propose again.
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Archive [doublon]');
  await page.keyboard.press('Control+Enter');
  await expect(page.locator('.card', { hasText: 'Archive [doublon]' })).toBeVisible();

  // A brainstorm proposes two pages and a decision, poured.
  await tabs.getByRole('button', { name: 'Docs' }).click();
  await page.keyboard.press('Control+b');
  const dialog = page.getByRole('dialog', { name: 'Nouvelle discussion' });
  await dialog.getByRole('radio', { name: /Brainstorm/ }).click();
  await dialog.getByRole('textbox').first().fill('Relance des adhésions [proposer:Portail]');
  await dialog.getByRole('button', { name: 'Commencer' }).click();
  const view = page.locator('.discussion[data-kind="brainstorm"]');
  const panel = page.getByRole('complementary', { name: 'Ce qui ira dans la spec' });
  await expect(panel.locator('[data-page="espace-adherent/relances"]')).toBeVisible({ timeout: 60_000 });
  await expect(panel.locator('[data-decision="1"]')).toBeVisible();
  // Nothing poured: no « Planifier » yet.
  await expect(view.locator('[data-plan-discussion]')).toHaveCount(0);
  await view.getByRole('button', { name: 'Verser dans la spec' }).click();
  await expect(panel.locator('[data-page]')).toHaveCount(0);

  // « Planifier » on the discussion: the planner's tickets, told under the discussion.
  const plan = view.getByRole('button', { name: 'Planifier' });
  await expect(plan).toBeVisible();
  await plan.click();
  const made = panel.getByRole('region', { name: 'Tickets' });
  await expect(made).toContainText('2 tickets créés', { timeout: 60_000 });
  await expect(made.locator('[data-made="POR-2"]')).toContainText('Implémenter : Relances');
  await expect(made.locator('[data-made="POR-3"]')).toContainText('Implémenter : Notifications');
  await expect(made.locator('[data-made="POR-2"]')).toContainText('À faire');
  await expect(made.locator('[data-skipped]')).toContainText('Déjà au Kanban : POR-1');
  await expect(made.locator('[data-plan-state]')).toHaveText('À lancer');
  await expect(made.locator('[data-plan-note]')).toContainText('Pas encore lancés');
  await expect(view.getByRole('button', { name: 'Planifier' })).toHaveCount(0);
  // Each page names its ticket; nothing committed.
  expect(fs.readFileSync(path.join(app.repo, '.escouade', 'spec', 'espace-adherent', 'relances.md'), 'utf8')).toContain('POR-2');
  expect(fs.readFileSync(notif, 'utf8')).toContain('POR-3');
  expect(git('rev-parse', 'HEAD').trim()).toBe(head);
  await shoot(page, 'discussion');

  // On the board: in « À faire », not started, each naming its page, which opens in the Docs tab.
  await made.getByRole('button', { name: 'Ouvrir POR-2 au Kanban' }).click();
  const card = page.locator('.card', { hasText: 'Implémenter : Relances' });
  await expect(card).toBeVisible();
  await expect(card.locator('[data-ticket-pages]')).toContainText('Relances');
  await expect(page.locator('.card', { hasText: 'Implémenter : Notifications' })).toContainText('Notifications');
  await expect(card).not.toContainText('Boucle');
  await expect(page.locator('.card[aria-label^="POR-"]')).toHaveCount(3);
  await shoot(page, 'board');
  await card.getByRole('button', { name: 'Ouvrir la page Relances de la spec' }).click();
  await expect(page.locator('[data-page="espace-adherent/relances"] button.pg.on')).toBeVisible();

  // The map: the subject's card shows its tickets.
  await page
    .getByRole('region', { name: 'Espaces des docs' })
    .getByRole('button', { name: /Spécification/ })
    .click();
  await expect(page.locator('[data-subject-card="espace-adherent"] [data-subject-tickets]')).toContainText('POR-2');
  await expect(page.locator('section[data-plan]')).toContainText('2 tickets créés');
  await shoot(page, 'map');

  // « Annuler »: the tickets go, the changes are to plan again, on top of the Specification.
  await page.locator('section[data-plan] [data-plan-cancel]').click();
  const bar = page.locator('[data-plan-bar]');
  await expect(bar).toContainText('3 changements sans tickets');
  await expect(page.locator('[data-subject-card="espace-adherent"] [data-subject-tickets]')).toHaveCount(0);
  await shoot(page, 'unplanned');
  await bar.getByRole('button', { name: 'Planifier' }).click();
  await expect(bar).toHaveCount(0, { timeout: 60_000 });
  await expect(page.locator('[data-subject-card="espace-adherent"] [data-subject-tickets]')).toContainText('POR-4');
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.locator('.card[aria-label^="POR-"]')).toHaveCount(3);
  await expect(page.locator('.card', { hasText: 'Implémenter : Relances' })).toContainText('POR-4');
  expect(git('rev-parse', 'HEAD').trim()).toBe(head);
});
