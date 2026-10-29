import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

// The themes of plugins (PLG-7, ESC-179): a declarative plugin that brings a theme, installed from a folder, chosen in
// Réglages › Application › Couleurs. The window, a terminal kept open and the project colours take it at once: the
// computed colour of a few surfaces is the theme's, a value that is no colour is left out (Escouade's kept), and what
// the theme leaves out (the light panel) keeps Escouade's. « Escouade » chosen again, then the plugin uninstalled, the
// window comes back to Escouade's colours.

const ID = 'dev.test.night';
const KEY = `${ID}/night`;
const THEME = {
  id: 'night',
  name: { fr: 'Nuit bleue', en: 'Blue night' },
  dark: {
    '--frame': '#102030',
    '--panel': '#203040',
    '--well': '#304050',
    '--text': '#f0e0d0',
    '--line': 'url(https://example.com/x.png)',
  },
  light: { '--frame': '#fafaf0' },
  projectColors: ['#e03030', '#30a030', '#3060e0'],
};

function themePlugin(dir: string) {
  fs.mkdirSync(dir, { recursive: true });
  const manifest = {
    id: ID,
    name: 'Thèmes de test',
    version: '1.0.0',
    description: { fr: 'Un thème pour les tests d’Escouade.', en: 'A theme for Escouade’s tests.' },
    license: 'MIT',
    escouade: { api: '1', min: '2.1.0' },
    contributes: { themes: [THEME] },
  };
  fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify(manifest, null, 2));
}

/** The folder dialog answered with `dir`, in the page, on the `fetch` of Tauri's IPC: no native window opens. */
async function pickAnswers(page: Page, dir: string) {
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
  }, dir);
}

/** A colour as the page paints it, in 8-bit sRGB. */
const rgb = (page: Page, css: string) =>
  page.evaluate((css) => {
    const probe = document.createElement('div');
    probe.style.color = css;
    document.body.append(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    const c = document.createElement('canvas');
    c.width = c.height = 1;
    const ctx = c.getContext('2d', { willReadFrequently: true })!;
    ctx.fillStyle = color;
    ctx.fillRect(0, 0, 1, 1);
    return [...ctx.getImageData(0, 0, 1, 1).data.slice(0, 3)].join(',');
  }, css);

/** What a few surfaces are painted with now: the frame, the work's panel, the text, a line, a terminal's well. */
async function surfaces(page: Page, box: Locator) {
  const style = (sel: string, prop: 'backgroundColor' | 'color' | 'boxShadow') =>
    page
      .locator(sel)
      .first()
      .evaluate((el, prop) => getComputedStyle(el)[prop], prop);
  return {
    frame: await rgb(page, await style('body', 'backgroundColor')),
    panel: await rgb(page, await style('.work', 'backgroundColor')),
    text: await rgb(page, await style('body', 'color')),
    line: await rgb(page, 'var(--line)'),
    terminal: await rgb(page, await box.locator('.xterm-viewport').evaluate((v) => (v as HTMLElement).style.backgroundColor)),
  };
}

/** The first project's tile, in the projects' rail. */
const projectTile = (page: Page) => page.locator('[role="tablist"][aria-orientation="vertical"] [role="tab"]').first();

async function openApplication(page: Page) {
  await page.keyboard.press('Control+,');
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Application' }).click();
  return dialog;
}

test('a plugin’s theme is put on the window, a terminal and the project colours, then taken away', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo);

  // A terminal kept open throughout: it takes each theme without being opened again.
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem').first().click();
  const box = page.locator('main.tv .box');
  await expect(box.locator('.xterm')).toBeVisible();

  // Escouade's dark colours, as they are before any theme.
  let settings = await openApplication(page);
  await settings.getByRole('group', { name: 'Thème' }).getByRole('button', { name: 'Sombre' }).click();
  await expect(settings.getByRole('button', { name: 'Voir les thèmes' })).toBeVisible();
  await settings.getByRole('button', { name: 'Fermer' }).click();
  const escouade = await surfaces(page, box);
  expect(escouade.frame).not.toBe('16,32,48');

  // The plugin installed from a folder: nothing changes before its theme is chosen.
  const src = path.join(app.root, 'night-theme');
  themePlugin(src);
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await pickAnswers(page, src);
  await page.getByRole('button', { name: 'Depuis un dossier…' }).click();
  const install = page.getByRole('dialog', { name: 'Installer Thèmes de test ?' });
  await expect(install).toContainText('Nuit bleue');
  await install.getByRole('button', { name: /^Installer/ }).click();
  await expect(install).toBeHidden();
  await projectTile(page).click();
  await expect(box.locator('.xterm')).toBeVisible();
  // Back from the view Plugins, the project colour glides back (0.7 s).
  await expect.poll(() => surfaces(page, box)).toEqual(escouade);

  // Chosen in Réglages › Application › Couleurs: applied at once, saved by itself.
  settings = await openApplication(page);
  const colors = settings.getByRole('combobox', { name: 'Couleurs' });
  await expect(colors.locator('option')).toHaveText(['Escouade', 'Nuit bleue · Thèmes de test']);
  await colors.selectOption(KEY);
  await expect(settings).toContainText('Une valeur de ce thème est écartée');
  await settings.getByRole('button', { name: 'Fermer' }).click();
  await expect
    .poll(() => surfaces(page, box))
    .toEqual({
      frame: '16,32,48',
      panel: '32,48,64',
      text: '240,224,208',
      // url() is left out: Escouade's line is kept.
      line: escouade.line,
      terminal: '48,64,80',
    });
  await expect.poll(() => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')).pluginTheme).toBe(KEY);
  // The same terminal throughout.
  await expect(page.locator('main.tv .xterm')).toHaveCount(1);

  // Its palette in place of the project colours.
  await projectTile(page).click({ button: 'right' });
  const swatches = page.locator('.swatches .swatch');
  await expect(swatches).toHaveCount(THEME.projectColors.length + 1);
  await page.keyboard.press('Escape');

  // In the light theme: its light frame, and Escouade's light panel, which it does not give.
  settings = await openApplication(page);
  await settings.getByRole('group', { name: 'Thème' }).getByRole('button', { name: 'Clair' }).click();
  await settings.getByRole('button', { name: 'Fermer' }).click();
  const light = await surfaces(page, box);
  expect(light.frame).toBe('250,250,240');
  const escouadeLightPanel = await page.evaluate(() => {
    const sheet = document.getElementById('escouade-plugin-theme')!;
    sheet.setAttribute('media', 'not all');
    const panel = getComputedStyle(document.querySelector('.work')!).backgroundColor;
    sheet.removeAttribute('media');
    return panel;
  });
  expect(light.panel).toBe(await rgb(page, escouadeLightPanel));

  // « Escouade » chosen again: its colours back, in both themes.
  settings = await openApplication(page);
  await settings.getByRole('group', { name: 'Thème' }).getByRole('button', { name: 'Sombre' }).click();
  await settings.getByRole('combobox', { name: 'Couleurs' }).selectOption('');
  await settings.getByRole('button', { name: 'Fermer' }).click();
  await expect.poll(() => surfaces(page, box)).toEqual(escouade);
  expect(await page.locator('#escouade-plugin-theme').count()).toBe(0);

  // Chosen once more, then the plugin uninstalled: Escouade's colours back, and the settings say why.
  settings = await openApplication(page);
  await settings.getByRole('combobox', { name: 'Couleurs' }).selectOption(KEY);
  await settings.getByRole('button', { name: 'Fermer' }).click();
  await expect.poll(async () => (await surfaces(page, box)).frame).toBe('16,32,48');
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page.getByText('Thèmes de test').first().click();
  await page.getByRole('button', { name: 'Désinstaller' }).click();
  await page.getByRole('dialog', { name: 'Désinstaller Thèmes de test ?' }).getByRole('button', { name: 'Désinstaller' }).click();
  await expect(page.getByText('Aucun plugin installé.')).toBeVisible();
  await projectTile(page).click();
  await expect(box.locator('.xterm')).toBeVisible();
  await expect.poll(() => surfaces(page, box)).toEqual(escouade);
  settings = await openApplication(page);
  await expect(settings).toContainText(`Le thème choisi n’est plus là : son plugin (${ID}) est désactivé ou désinstallé.`);
});
