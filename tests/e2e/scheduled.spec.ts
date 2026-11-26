// The scheduled tasks (ESC-448), in the real app: a task made from « Programmées », its ticket made and launched at its
// due time, marked « ⏱ programmée », and the next due time passing its turn while that ticket runs. The app's clock
// for the scheduled tasks runs 600 times faster (`ESCOUADE_SCHEDULE_SPEED`, debug builds only): an hour goes by in 6 s.
import { addProject, expect, test, unfoldSections } from './fixture';

test.use({ appEnv: { ESCOUADE_SCHEDULE_SPEED: '600', ESCOUADE_SCHEDULE_TICK_MS: '250' } });

test('a scheduled task makes and launches its ticket at its due time, once while it runs', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await unfoldSections(page);
  const section = page.locator('[data-scheduled-section]');
  await expect(section).toContainText('Aucune tâche programmée');

  // « ＋ » opens the ticket's form in « Programmer ».
  await section.getByRole('button', { name: 'Programmer une tâche' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouvelle tâche programmée' });
  // [lent]: the fake agent's turn lasts 30 s, so the ticket is still running at the next due times.
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Revue des dépendances [lent]');
  await dialog.getByRole('textbox', { name: 'Consigne' }).fill('Mets à jour les dépendances.');
  await dialog.getByRole('group', { name: 'Rythme' }).getByRole('button', { name: 'Cron' }).click();
  await dialog.getByRole('textbox', { name: 'Expression' }).fill('0 * * * *');
  await expect(dialog.locator('[data-upcoming]')).toContainText('0 * * * *');
  await dialog.getByRole('button', { name: 'Programmer' }).click();
  await expect(dialog).toBeHidden();

  const row = section.locator('[data-task]');
  await expect(row).toHaveCount(1);
  await expect(row).toContainText('Revue des dépendances [lent]');
  await expect(row).toContainText('Cron 0 * * * *');
  await expect(row.getByRole('switch')).toHaveAttribute('aria-checked', 'true');

  // At its due time (within an hour of the fast clock, 6 s), its ticket is made and launched: « en cours » with its key.
  await expect(row).toContainText('en cours (DEM-1)', { timeout: 30_000 });
  // Its agent's card says it, once that agent is made and started: seconds on a loaded machine (14 s seen, more at
  // times), as long as the ticket itself may take.
  await expect(page.locator('.side [data-scheduled-mark]').first()).toHaveText('⏱ programmée', { timeout: 30_000 });

  // Two more hours of the fast clock: their due times pass their turn while DEM-1 runs, no second ticket.
  await page.waitForTimeout(13_000);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const marks = page.locator('main [data-scheduled-mark], .board [data-scheduled-mark]');
  await expect(marks).toHaveCount(1);
  await expect(marks.first()).toHaveText('⏱ programmée');
  await expect(page.getByText('DEM-2')).toHaveCount(0);

  // Its mark opens its task.
  await marks.first().click();
  await expect(page.getByRole('dialog', { name: 'Modifier la tâche programmée' })).toBeVisible();
});
