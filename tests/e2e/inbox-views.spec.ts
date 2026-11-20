import fs from 'node:fs';
import path from 'node:path';
import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';

// What Escouade puts in the box « À toi » of its own (ESC-513, docs/INBOX.md § 5, § 11), in the box's screen (ESC-510), with the fake `claude`: views
// on what already waits for the user. A permission allowed from the box lets its agent go on; one answered in the
// conversation leaves the box; a ticket approved from the box is done; the summary of the requests comes once. The box
// rests on the MCP server: off, nothing of it shows.

const SWITCH = 'Les agents peuvent piloter Escouade';

// The summary gathers for 1.5 s here, not 2 minutes.
test.use({ appEnv: { ESCOUADE_INBOX_GROUP_MS: '1500' } });

/** The number of summaries the app told, as its log says them (never what they say). */
function summaries(data: string): number {
  const log = path.join(data, 'app.log');
  if (!fs.existsSync(log)) return 0;
  return fs
    .readFileSync(log, 'utf8')
    .split('\n')
    .filter((l) => l.includes('inbox summary told')).length;
}

test('the box shows what waits elsewhere and settles it at its source, both ways', async ({ app }) => {
  // A ticket goes to « À tester » and is merged on the way: longer than a test of the box alone.
  test.slow();
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  // The server off: no tile, nothing of the box.
  await expect(page.getByTestId('inbox-tile')).toHaveCount(0);
  // The server on before the agents: their processes start with its tools.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'true');
  await closeSettings(dialog);
  const tile = page.getByTestId('inbox-tile');
  await expect(tile).toBeVisible();

  // A permission its agent stopped for: in the box, at the head, and allowed there.
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await send(page, 'permission');
  await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build');
  await tile.click();
  // In the box's screen (ESC-510): the view in its queue, among the agents stopped, opened in its panel.
  const queue = page.getByTestId('inbox-queue');
  const pane = page.getByTestId('ivw-pane');
  const perm = queue.locator('[data-testid="ivw-row"][data-source="permission"]');
  await expect(perm).toHaveCount(1);
  await expect(perm).toContainText('rm -rf build');
  await expect(perm).toContainText('Autorisation');
  // One count, the stopped agent's (« 1 en attente »): its view is not counted again.
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await perm.click();
  await expect(pane).toContainText('Autorisation · l’agent s’est arrêté');
  await pane.getByRole('button', { name: 'Autoriser', exact: true }).click();
  await expect(perm).toHaveCount(0);
  await expect(page.getByTestId('inbox-count')).toHaveCount(0);
  // Its agent went on: its conversation says so.
  await page.keyboard.press('Escape');
  await expect(page.getByText('Commande exécutée.')).toBeVisible();

  // Another, answered in the conversation: it leaves the box.
  await send(page, 'permission');
  const card = page.getByTestId('permission-pending');
  await expect(card).toContainText('rm -rf build');
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await card.getByRole('button', { name: 'Autoriser', exact: true }).click();
  await expect(page.getByTestId('inbox-count')).toHaveCount(0);
  await tile.click();
  await expect(queue.locator('[data-testid="ivw-row"][data-source="permission"]')).toHaveCount(0);
  await page.keyboard.press('Escape');

  // A ticket « À tester »: approved from the box, it is done.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Ajouter le fichier du ticket');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  // Counted on the tile, besides the requests and the agents stopped.
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await tile.click();
  const ticket = queue.locator('[data-testid="ivw-row"][data-source="ticketReview"]');
  await expect(ticket).toContainText('Ticket à valider');
  await expect(ticket).toContainText('Ajouter le fichier du ticket');
  await ticket.click();
  await expect(pane).toContainText('DEM-1');
  await expect(page.getByTestId('ivw-source')).toHaveText('Voir dans le Kanban');
  await pane.getByRole('button', { name: 'Valider' }).click();
  await expect(ticket).toHaveCount(0, { timeout: 60_000 });
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('Mergé dans main · squash')).toBeVisible({ timeout: 60_000 });

  // The rows stay whole in the window, at either size.
  await tile.click();
  const outside = await page.evaluate(
    () =>
      [...document.querySelectorAll('[data-testid="inbox-queue"] *, [data-testid="inbox"] *')]
        .map((e) => e.getBoundingClientRect())
        .filter((r) => r.width && (r.right > innerWidth + 1 || r.left < -1)).length,
  );
  expect(outside).toBe(0);
  await page.keyboard.press('Escape');

  // The server off: nothing of the box shows, its views included; a request still waits where it is today.
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await send(page, 'permission');
  await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build');
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  await closeSettings(dialog);
  await expect(page.getByTestId('inbox-tile')).toHaveCount(0);
  await expect(page.locator('[data-testid="ivw-row"]')).toHaveCount(0);
  await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build');
});

test('the summary of the box comes once, never one alert per request', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  // Its setting is under the notifications, now that the server is on.
  await openSettingsPage(dialog, 'Notifications');
  // The summary is a system notification: the tests keep them off, turned on here (out of sight, the app shows none).
  await dialog.getByRole('switch', { name: 'Notifications système' }).click();
  await expect(dialog.getByRole('switch', { name: 'Notifications système' })).toHaveAttribute('aria-checked', 'true');
  await expect(dialog.getByRole('combobox', { name: 'Résumé de la boîte « À toi »' })).toHaveValue('every30');
  await closeSettings(dialog);
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();

  expect(summaries(app.data)).toBe(0);
  await send(page, '[demande] rends les trois directions du tableau de bord');
  await expect(page.getByText('Demande a-1 déposée (7 fichiers), je continue mon travail.')).toBeVisible({ timeout: 30_000 });
  // Gathered, then told once.
  await expect.poll(() => summaries(app.data), { timeout: 20_000 }).toBe(1);
  // Another request soon after: no second alert within the 30 minutes.
  await send(page, '[demande] rends encore les directions');
  await expect(page.getByText('Demande a-2 déposée (7 fichiers), je continue mon travail.')).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(4_000);
  expect(summaries(app.data)).toBe(1);
  // What it told is the shape of the summary, never a request's text.
  const log = fs.readFileSync(path.join(app.data, 'app.log'), 'utf8');
  expect(log).toContain('inbox summary told (1 requests, 1 projects)');
  expect(log).not.toContain('tableau de bord');
});
