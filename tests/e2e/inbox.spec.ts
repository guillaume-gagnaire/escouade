import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';

// The box « À toi » (ESC-510, docs/INBOX.md, direction B « La file »): an agent leaves a choice of three renders (dark and
// light) with a PDF of three pages through Escouade's MCP server; the box shows it and counts it, the user compares the
// renders full size, reads the PDF page by page and chooses; the answer comes back to the agent and the box empties. A
// go-ahead, a task and a request put off that comes back at its time; the MCP server off, nothing of the box shows, on
// again, it comes back with its requests. At the window's size of each run (1440 × 900, or ESCOUADE_E2E_VIEWPORT).
// `ESCOUADE_E2E_SHOTS`: where the captures go (dark and light), for the website and the review.

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SWITCH = 'Les agents peuvent piloter Escouade';

// The server on from the start: the agent's process is given the tools as it starts.
test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

/** A command of the window, as its screen calls it. */
async function invoke<T>(page: Page, cmd: string, args: Record<string, unknown> = {}) {
  return page.evaluate(
    ([cmd, args]) =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
        cmd as string,
        args,
      ),
    [cmd, args] as const,
  ) as Promise<T>;
}

/** The captures of the screen as it is, dark then light, named by `name` and the window's width. */
async function shoot(page: Page, name: string, lang: 'fr' | 'en' = 'fr') {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const width = page.viewportSize()?.width ?? (await page.evaluate(() => innerWidth));
  // At the website's size (`ESCOUADE_E2E_VIEWPORT=1920x960`): its picture, dark, in JPEG, by language.
  if (width >= 1900) {
    await page.evaluate(() => (document.documentElement.dataset.theme = 'dark'));
    await page.waitForTimeout(150);
    const dir = path.join(SHOTS, 'site', lang === 'en' ? 'en' : '');
    fs.mkdirSync(dir, { recursive: true });
    await page.screenshot({ path: path.join(dir, `${name}.jpg`), type: 'jpeg', quality: 86 });
    return;
  }
  const size = width < 1100 ? '-1024' : '';
  for (const theme of ['dark', 'light'] as const) {
    await page.evaluate((th) => (document.documentElement.dataset.theme = th), theme);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}${theme === 'light' ? '-clair' : ''}${size}.png`) });
  }
  await page.evaluate(() => (document.documentElement.dataset.theme = 'dark'));
}

/** A project with an agent whose process has Escouade's tools. */
async function withAgent(page: Page, repo: string, lang: 'fr' | 'en' = 'fr') {
  await addProject(page, repo, { firstAgent: false, lang });
  await page
    .getByRole('button', { name: lang === 'en' ? 'New agent' : 'Nouvel agent', exact: true })
    .first()
    .click();
}

const tile = (page: Page) => page.getByTestId('inbox-tile');

test('a choice of three renders with a PDF: the box shows and counts it, the user compares, reads the PDF and chooses, the agent gets the answer', async ({
  app,
}) => {
  test.setTimeout(180_000);
  const { page } = app;
  await withAgent(page, app.repo);
  await send(page, '[demande] rends les trois directions du tableau de bord');
  await expect(page.getByText('Demande a-1 déposée (7 fichiers), je continue mon travail.')).toBeVisible({ timeout: 30_000 });

  // Counted in the rail, without a restart.
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await tile(page).click();
  const queue = page.getByTestId('inbox-queue');
  const row = queue.getByTestId('inbox-row').filter({ hasText: 'Choisis la direction du tableau de bord' });
  await expect(row).toContainText('1 PDF, 6 images');
  // The renders of its options, under its line.
  await expect(row.locator('img')).toHaveCount(3);
  await expect
    .poll(() =>
      row
        .locator('img')
        .first()
        .evaluate((i: HTMLImageElement) => i.naturalWidth),
    )
    .toBe(480);

  const pane = page.getByTestId('ask-pane');
  await expect(pane.getByRole('heading', { name: 'Choisis la direction du tableau de bord' })).toBeVisible();
  await expect(pane.getByTestId('ask-plate')).toHaveCount(3);
  await expect(pane.getByRole('img', { name: 'Recommandé par l’agent' })).toBeVisible();
  await expect(pane.getByTestId('ask-choose')).toContainText('Choisir B');

  // The PDF, read in place: its first page drawn, then the next.
  const reader = pane.getByTestId('pdf-reader');
  await expect(reader).toContainText('sur 3');
  const canvas = reader.getByTestId('pdf-canvas');
  await expect.poll(() => canvas.evaluate((c: HTMLCanvasElement) => c.width)).toBeGreaterThan(100);
  // Drawn: not a blank page (its bars are grey and blue).
  await expect
    .poll(() =>
      canvas.evaluate((c: HTMLCanvasElement) => {
        const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data;
        let ink = 0;
        for (let i = 0; i < d.length; i += 4 * 97) if (d[i] < 200 || d[i + 2] < 200) ink++;
        return ink;
      }),
    )
    .toBeGreaterThan(10);
  // Its pages' thumbnails drawn too, the first one included (the page's drawing never cancels theirs).
  await expect
    .poll(() =>
      reader
        .getByRole('button', { name: 'Page 1' })
        .locator('canvas')
        .evaluate((c: HTMLCanvasElement) => {
          const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data;
          let ink = 0;
          for (let i = 0; i < d.length; i += 4) if (d[i] < 200 || d[i + 2] < 200) ink++;
          return ink;
        }),
    )
    .toBeGreaterThan(3);
  await shoot(page, 'inbox-pdf');
  await reader.getByTestId('pdf-next').click();
  await expect(reader.getByTestId('pdf-page')).toHaveValue('2');
  await expect(canvas).toHaveAttribute('data-page', '2');

  // Compared full size: dark and light side by side, moved through by the keyboard, closed by Échap.
  await pane.getByTestId('ask-compare').click();
  const compare = page.getByRole('dialog', { name: 'Comparer les rendus' });
  await expect(compare.getByTestId('ask-plate')).toHaveCount(3);
  await compare.getByRole('radio', { name: 'Les deux' }).click();
  await expect(compare.getByTestId('ask-plate').first().locator('img')).toHaveCount(2);
  await shoot(page, 'inbox-comparer');
  await page.keyboard.press('ArrowRight');
  await expect(compare.getByTestId('compare-choose')).toContainText('Choisir C');
  await page.keyboard.press('1');
  await expect(compare.getByTestId('compare-choose')).toContainText('Choisir A');
  await page.keyboard.press('Escape');
  await expect(compare).toBeHidden();
  await shoot(page, 'inbox-choix');

  // Chosen from the keyboard, with a word.
  await page.keyboard.press('1');
  await pane.getByTestId('ask-note').fill('garde la courbe de B');
  await pane.getByTestId('ask-note').press('Enter');
  await expect(page.getByTestId('inbox-sending')).toContainText('part dans');
  // The 10 seconds over, the agent reads it, and the box is empty.
  await expect
    .poll(async () => (await invoke<{ state: string }>(page, 'inbox_get', { id: 'a-1' })).state, { timeout: 40_000 })
    .toBe('read');
  await expect(page.getByTestId('inbox-empty')).toContainText('Rien ne t’attend');
  await expect(page.getByTestId('inbox-count')).toHaveCount(0);
  await expect(queue.getByTestId('inbox-recent-row')).toContainText('A · Colonnes de chiffres · toi');
  await expect(queue.getByTestId('inbox-recent-row')).toContainText('lue par');
  await shoot(page, 'inbox-vide');
  // The agent has it, marked as the user's.
  await page.getByTestId('inbox-recent-row').click();
  await page.getByTestId('inbox-to-agent').click();
  await expect(page.getByText('Réponse reçue pour a-1 : A')).toBeVisible();
});

test('a go-ahead, a task, an answer, and a request put off that comes back at its time', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await withAgent(page, app.repo);
  await send(page, '[demande-oui]');
  await expect(page.getByText(/Demande a-1 déposée/)).toBeVisible({ timeout: 30_000 });
  await send(page, '[demande-tache]');
  await expect(page.getByText(/Demande a-2 déposée/)).toBeVisible({ timeout: 30_000 });
  await send(page, '[demande-texte]');
  await expect(page.getByText(/Demande a-3 déposée/)).toBeVisible({ timeout: 30_000 });
  await expect(page.getByTestId('inbox-count')).toHaveText('3');

  await page.keyboard.press('Control+Shift+J');
  const pane = page.getByTestId('ask-pane');
  // The go-ahead first (« Aujourd'hui »): its labels, its notes drawn, what it says of the gate.
  await expect(pane).toContainText('Publier atlas-web 1.4.0 en production ?');
  await expect(pane.getByTestId('ask-yes')).toContainText('Oui, publie');
  await expect(pane.getByRole('heading', { name: 'Notes de la 1.4.0' })).toBeVisible();
  await expect(pane).toContainText('Un feu vert : toi seul y réponds.');
  await expect(page.getByTestId('inbox-nth')).toHaveText('1 sur 3');
  await shoot(page, 'inbox-validation');

  // J goes to the next, K back.
  await page.getByTestId('inbox-nth').click();
  await page.keyboard.press('j');
  await expect(page.getByTestId('inbox-nth')).toHaveText('2 sur 3');
  await page.keyboard.press('k');
  await expect(page.getByTestId('inbox-nth')).toHaveText('1 sur 3');

  // « Plus tard » by L, in an hour: it leaves the queue for « Plus tard ».
  await page.keyboard.press('l');
  await page.getByRole('menuitem', { name: /Dans 1 heure/ }).click();
  await expect(page.getByTestId('inbox-later')).toContainText('1');
  await expect(page.getByTestId('inbox-count')).toHaveText('2');

  // The task: its steps ticked, « C'est fait ».
  await expect(pane).toContainText('Essaie la connexion par lien magique');
  await shoot(page, 'inbox-tache');
  await pane.getByTestId('ask-step').first().check();
  await pane.getByTestId('ask-done').click();
  // The answer: the text the agent proposed, sent by Ctrl Entrée.
  await expect(pane).toContainText('Quelle phrase d’accueil');
  await expect(pane.getByTestId('ask-text')).toHaveValue('Bienvenue au club : ton espace adhérent t’attend.');
  await pane.getByTestId('ask-text').press('Control+Enter');
  await expect(page.getByTestId('inbox-empty')).toBeVisible();

  // The go-ahead put off comes back at its time (here in a few seconds), marked so, at the head of « Aujourd'hui ».
  await invoke(page, 'inbox_snooze', { id: 'a-1', until: Date.now() + 2000 });
  await expect(page.getByTestId('inbox-row').filter({ hasText: 'Publier atlas-web' })).toContainText('revenue de Plus tard', {
    timeout: 40_000,
  });
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await page.getByTestId('inbox-row').filter({ hasText: 'Publier atlas-web' }).click();
  await pane.getByTestId('ask-yes').click();
  await expect
    .poll(async () => (await invoke<{ state: string }>(page, 'inbox_get', { id: 'a-1' })).state, { timeout: 40_000 })
    .toBe('read');
  await expect
    .poll(async () => (await invoke<{ state: string }>(page, 'inbox_get', { id: 'a-2' })).state, { timeout: 40_000 })
    .toBe('read');
});

test('the MCP server off, nothing of the box shows; on again, it comes back with its requests, without a restart', async ({ app }) => {
  test.setTimeout(120_000);
  const { page } = app;
  await withAgent(page, app.repo);
  await send(page, '[demande-tache]');
  await expect(page.getByText(/Demande a-1 déposée/)).toBeVisible({ timeout: 30_000 });
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await tile(page).click();
  await expect(page.getByTestId('ask-pane')).toBeVisible();

  // Turned off: asked first (a request waits), then the box closes, its tile and count go.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  let dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  const ask = page.getByRole('dialog', { name: 'Éteindre le serveur MCP ?' });
  await expect(ask).toContainText('1 demande de tes agents t’attend');
  await expect(ask).toContainText('Essaie la connexion par lien magique');
  await shoot(page, 'inbox-eteindre');
  await ask.getByRole('button', { name: 'Éteindre quand même' }).click();
  dialog = page.getByRole('dialog', { name: 'Réglages' });
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByTestId('inbox-settings')).toContainText('Éteinte avec le serveur');
  await closeSettings(dialog);
  await expect(tile(page)).toHaveCount(0);
  await expect(page.getByTestId('inbox-queue')).toHaveCount(0);
  await page.keyboard.press('Control+Shift+J');
  await expect(page.getByTestId('inbox-queue')).toHaveCount(0);
  // Kept, frozen.
  expect(fs.existsSync(path.join(app.data, 'asks'))).toBe(true);

  // On again: the tile comes back with its request.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'true');
  await closeSettings(dialog);
  await expect(page.getByTestId('inbox-count')).toHaveText('1');
  await tile(page).click();
  await expect(page.getByTestId('ask-pane')).toContainText('Essaie la connexion par lien magique');
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the box says it in English: its tile, its queue, a choice and its PDF', async ({ app }) => {
    test.setTimeout(120_000);
    const { page } = app;
    await withAgent(page, app.repo, 'en');
    await send(page, '[demande] render the three directions');
    await expect(page.getByText(/Demande a-1 déposée/)).toBeVisible({ timeout: 30_000 });
    await expect(tile(page)).toHaveAccessibleName('Your turn, 1 thing waits for you');
    await tile(page).click();
    const queue = page.getByTestId('inbox-queue');
    await expect(queue.getByRole('heading', { name: /Your turn/ })).toBeVisible();
    await expect(queue.getByTestId('inbox-row')).toContainText('1 PDF, 6 images');
    const pane = page.getByTestId('ask-pane');
    await expect(pane.getByTestId('ask-choose')).toContainText('Choose B');
    await expect(pane).toContainText('Recommended: B · Cartes empilées.');
    await expect(pane.getByTestId('pdf-reader')).toContainText('of 3');
    await expect(page.getByTestId('inbox-nth')).toHaveText('1 of 1');
    await expect.poll(() => pane.getByTestId('pdf-canvas').evaluate((c: HTMLCanvasElement) => c.width)).toBeGreaterThan(100);
    await shoot(page, 'inbox', 'en');
    await pane.getByTestId('ask-compare').click();
    await expect(page.getByRole('dialog', { name: 'Compare the renders' })).toBeVisible();
    await page.getByRole('dialog', { name: 'Compare the renders' }).getByRole('radio', { name: 'Both' }).click();
    await shoot(page, 'inbox-compare', 'en');
    await page.keyboard.press('Escape');
    await pane.getByTestId('ask-choose').click();
    await expect(page.getByTestId('inbox-sending')).toContainText('goes in');
  });
});
