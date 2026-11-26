import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';

// What Escouade puts in the box « À toi » of its own (ESC-513, docs/INBOX.md § 5, § 11), in the box's screen (ESC-510), with the fake `claude`: views
// on what already waits for the user. A permission allowed from the box lets its agent go on; one answered in the
// conversation leaves the box; a ticket approved from the box is done; the summary of the requests comes once; the squad
// lead takes a ticket in hand and settles it (ESC-557). The box
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

// ESC-557: the squad lead keeps the box true. Through the MCP server, it takes in hand a ticket it is checking (the
// count drops, the line stays to read in « Le chef s'en occupe » with its note), then ticks it as settled (the line goes
// to the trace « Réglé par … »), whence the user puts it back. Every agent may here (« Tous les agents »), the ticket's
// own agent excepted.
test.describe('the squad lead’s marks', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true, agentsAnswer: 'all' }) } });

  /** The captures of the screen as it is, dark then light, named by `name` and the window's width. */
  async function shoot(page: Page, name: string) {
    const dir = process.env.ESCOUADE_E2E_SHOTS;
    if (!dir) return;
    fs.mkdirSync(dir, { recursive: true });
    const width = page.viewportSize()?.width ?? (await page.evaluate(() => innerWidth));
    const size = width < 1100 ? '-1024' : '';
    for (const theme of ['dark', 'light'] as const) {
      await page.evaluate((th) => (document.documentElement.dataset.theme = th), theme);
      await page.waitForTimeout(150);
      await page.screenshot({ path: path.join(dir, `${name}${theme === 'light' ? '-clair' : ''}${size}.png`) });
    }
    await page.evaluate(() => (document.documentElement.dataset.theme = 'dark'));
  }

  test('the lead takes a ticket to approve in hand, then settles it, and the user puts it back', async ({ app }) => {
    test.slow();
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    const tile = page.getByTestId('inbox-tile');
    await expect(tile).toBeVisible();
    // A ticket « À tester »: it waits for the user, counted on the tile.
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Ajouter le fichier du ticket');
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    const review = page.getByRole('region', { name: 'À tester' });
    await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId('inbox-count')).toHaveText('1');

    // The lead, another agent than the ticket's, takes it in hand through the MCP server.
    await page.getByRole('button', { name: 'Agents', exact: true }).click();
    await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
    // Its own view first: until the new agent lands, the view is the ticket's agent's, and the message went to it,
    // which the box refuses as the lead (the ticket's own agent; ESC-650, under load and at 1024).
    await expect(page.getByRole('heading', { name: 'agent-2', level: 1 })).toBeVisible();
    await send(page, '[chef-prend DEM-1]');
    await expect(page.getByText('Je m’occupe de DEM-1 (il en reste 0).')).toBeVisible({ timeout: 30_000 });
    // No longer waiting for the user: the count drops; the line stays to read, with its note.
    await expect(page.getByTestId('inbox-count')).toHaveCount(0);
    await tile.click();
    const queue = page.getByTestId('inbox-queue');
    await expect(queue.getByTestId('ivw-claimed-group')).toContainText('Le chef s’en occupe');
    const line = queue.locator('[data-testid="ivw-row"][data-mark="claimed"]');
    await expect(line).toContainText('Ajouter le fichier du ticket');
    await expect(line).toContainText('vérification en cours, chaîne de tests');
    await line.click();
    const pane = page.getByTestId('ivw-pane');
    await expect(pane.getByTestId('ivw-mark')).toContainText('s’en occupe');
    await expect(pane.getByTestId('ivw-mark')).toContainText('« vérification en cours, chaîne de tests »');
    // The user may still act on it: asked first, the lead being on it; cancelled, nothing is done.
    await pane.getByRole('button', { name: 'Valider' }).click();
    const confirm = page.getByRole('dialog');
    await expect(confirm).toContainText('est en train de la vérifier');
    await shoot(page, 'inbox-chef-occupe');
    await confirm.getByRole('button', { name: 'Annuler' }).click();
    await page.keyboard.press('Escape');

    // Settled another way (merged by hand): out of the box, in its trace.
    await send(page, '[chef-regle DEM-1]');
    await expect(page.getByText('Réglé : DEM-1 (il en reste 0).')).toBeVisible({ timeout: 30_000 });
    await tile.click();
    const settled = queue.getByTestId('ivw-settled-line');
    await expect(settled).toContainText('fusionné à la main sur main en 79fe7aec');
    await expect(settled).toContainText('Réglé par');
    await expect(queue.getByTestId('ivw-claimed-group')).toHaveCount(0);
    await queue.locator('[data-testid="ivw-row"][data-mark="settled"]').click();
    await expect(pane.getByRole('button', { name: 'Valider' })).toHaveCount(0);
    await shoot(page, 'inbox-chef-regle');
    // The user puts it back: it waits for them again.
    await pane.getByRole('button', { name: 'Remettre dans la boîte' }).click();
    await expect(page.getByTestId('inbox-count')).toHaveText('1');
    await expect(queue.locator('[data-testid="ivw-row"][data-source="ticketReview"]:not([data-mark])')).toHaveCount(1);
    // The rows stay whole in the window, at either size.
    const outside = await page.evaluate(
      () =>
        [...document.querySelectorAll('[data-testid="inbox-queue"] *, [data-testid="inbox"] *')]
          .map((e) => e.getBoundingClientRect())
          .filter((r) => r.width && (r.right > innerWidth + 1 || r.left < -1)).length,
    );
    expect(outside).toBe(0);
  });
});
