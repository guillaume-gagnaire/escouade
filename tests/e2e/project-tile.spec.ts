// The project's tile made one's own (ESC-167): its letters chosen in Réglages › Projets › Projet, then a logo in their place, on
// the rail's tile, kept after a restart and erased with the project. At 1440 × 900 and in the runner's window (its
// screen is 1024 × 768).

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';

const tile = (page: Page) => page.getByRole('tab', { name: /^demo-api/ });

/** Opens the settings on « Projets ». */
async function projectsSettings(page: Page) {
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  if (!(await settings.isVisible())) await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await openSettingsPage(settings, 'Projet');
  return settings;
}

/**
 * The file dialog answered with `file`, in the page, on the `fetch` of Tauri's IPC (as links.spec.ts does): no native
 * window opens.
 */
async function pickAnswers(page: Page, file: string) {
  await page.evaluate((answer) => {
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('plugin:dialog|open'))
        return Promise.resolve(
          new Response(JSON.stringify(answer), { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }),
        );
      return fetch(input, init);
    };
  }, file);
}

/** Its id in the saved state. */
const projectId = (data: string) => JSON.parse(fs.readFileSync(path.join(data, 'state.json'), 'utf8')).projects[0].id as string;

for (const size of [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
])
  test(`the tile’s letters then its logo are set in the settings and shown on the rail, at ${size.width} × ${size.height}`, async ({
    app,
  }) => {
    test.setTimeout(120_000);
    let { page } = app;
    await page.setViewportSize(size);
    await addProject(page, app.repo);
    await expect(tile(page)).toContainText('DA');

    // The letters: empty shows the initials; three typed, a fourth left out.
    let settings = await projectsSettings(page);
    const letters = settings.getByRole('textbox', { name: 'Lettres de la tuile' });
    await letters.scrollIntoViewIfNeeded();
    await expect(letters).toHaveAttribute('placeholder', 'DA');
    await letters.pressSequentially('apiv');
    await expect(letters).toHaveValue('api');
    await expect(settings.getByRole('img', { name: 'Tuile du projet' })).toHaveText('api');
    await closeSettings(settings);
    await expect(tile(page).locator('.ini')).toHaveText('api');

    // The logo: an SVG whose script never runs, copied by the app; the file chosen then goes.
    const svg = path.join(app.root, 'logo.svg');
    fs.writeFileSync(
      svg,
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" onload="window.esc167=1"><script>window.esc167=1</script><rect width="10" height="10" fill="#e33"/></svg>',
    );
    await pickAnswers(page, svg);
    settings = await projectsSettings(page);
    await settings.getByRole('button', { name: 'Choisir une image…' }).click();
    await expect(settings.getByRole('img', { name: 'Tuile du projet' }).locator('img')).toHaveAttribute(
      'src',
      /^data:image\/svg\+xml;base64,/,
    );
    await closeSettings(settings);
    fs.rmSync(svg);
    const logo = tile(page).locator('img.logo');
    await expect(logo).toHaveAttribute('src', /^data:image\/svg\+xml;base64,/);
    await expect(tile(page).locator('.ini')).toHaveCount(0);
    // Drawn whole, in the tile's 36 px.
    await expect.poll(() => logo.evaluate((i: HTMLImageElement) => i.complete && i.naturalWidth > 0)).toBe(true);
    const box = (await logo.boundingBox())!;
    expect(Math.round(box.width)).toBe(36);
    expect(Math.round(box.height)).toBe(36);
    expect(await page.evaluate(() => (window as unknown as { esc167?: number }).esc167)).toBeUndefined();
    // Its state still over it: the agent's work at the end of its first turn is no badge, its name is said.
    await expect(tile(page)).toHaveAccessibleName(/^demo-api/);

    // Copied under the project in the app's data, not in its .escouade.yml.
    const id = projectId(app.data);
    const copies = fs.readdirSync(path.join(app.data, 'projects', id));
    expect(copies).toHaveLength(1);
    expect(copies[0]).toMatch(/^logo-.+\.svg$/);
    expect(fs.existsSync(path.join(app.repo, '.escouade.yml'))).toBe(false);

    // Kept after a restart.
    page = await app.restart();
    await page.setViewportSize(size);
    await expect(tile(page).locator('img.logo')).toHaveAttribute('src', /^data:image\/svg\+xml;base64,/);

    // « Retirer »: the letters chosen again.
    settings = await projectsSettings(page);
    await settings.getByRole('button', { name: 'Retirer' }).click();
    await closeSettings(settings);
    await expect(tile(page).locator('.ini')).toHaveText('api');
    await expect(tile(page).locator('img.logo')).toHaveCount(0);
    expect(fs.readdirSync(path.join(app.data, 'projects', id))).toHaveLength(0);
  });

test('a project closed with its history erased erases its logo', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const png = path.join(app.root, 'logo.png');
  // A PNG of one pixel.
  fs.writeFileSync(
    png,
    Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64'),
  );
  await pickAnswers(page, png);
  const settings = await projectsSettings(page);
  await settings.getByRole('button', { name: 'Choisir une image…' }).click();
  await expect(settings.getByRole('img', { name: 'Tuile du projet' }).locator('img')).toHaveAttribute('src', /^data:image\/png;base64,/);
  await closeSettings(settings);
  await expect(tile(page).locator('img.logo')).toHaveAttribute('src', /^data:image\/png;base64,/);
  const dir = path.join(app.data, 'projects', projectId(app.data));
  expect(fs.readdirSync(dir)).toHaveLength(1);

  await tile(page).click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Fermer le projet…' }).click();
  const dialog = page.getByRole('dialog', { name: 'Fermer « demo-api » ?' });
  await dialog.getByRole('checkbox', { name: 'Effacer aussi son historique (conversations, tickets, réglages)' }).check();
  await dialog.getByRole('button', { name: 'Fermer le projet', exact: true }).click();
  await expect(tile(page)).toHaveCount(0);
  await expect.poll(() => fs.existsSync(dir)).toBe(false);
});
