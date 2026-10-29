// The view Plugins in the 2.0 design (PLG-4, ESC-176; choices 213 to 218): the installed plugins, a plugin's page and the
// install window of an update, compared to the preview (design/plugins/maquettes-b.html, `#b2`, `#b3`, `#b4`) at 1440 × 900:
// the sidebar's items, the table's columns and rows, the page's head, tabs and aside, the window's blocks, measured alike in
// both; at 1440 × 900 and at CI's 1028 × 779, in both themes, nothing overflows. The captures of the app and of the preview
// go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'plugins', 'maquettes-b.html');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
const invokeOn =
  (page: Page): Invoke =>
  (cmd, args = {}) =>
    page.evaluate(
      ([cmd, args]) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
          cmd as string,
          args,
        ),
      [cmd, args] as const,
    ) as never;

const git = (dir: string, ...args: string[]) => execFileSync('git', ['-C', dir, ...args], { encoding: 'utf8' }).trim();

/** A manifest of the Slack of the preview: a program, its events, its host, its secret and its settings. */
function slack(version: string, events: string[]) {
  return {
    id: 'dev.lmartin.slack',
    name: 'Slack',
    version,
    description: {
      fr: 'Les tickets à tester et les agents qui attendent, dans un salon Slack : un salon par projet, un fil par ticket.',
      en: 'The tickets to test and the agents waiting, in a Slack channel: a channel per project, a thread per ticket.',
    },
    author: { name: 'lea-martin' },
    license: 'MIT',
    escouade: { api: '1', min: '2.1.0' },
    run: { command: 'bin/escouade-slack', windows: 'bin/escouade-slack.exe' },
    activation: ['event:ticket.column'],
    permissions: { events, network: ['hooks.slack.com'], secrets: ['webhook'] },
    contributes: {
      webhooks: [{ id: 'post', name: 'Message Slack' }],
      settings: [{ key: 'channel', type: 'string', title: { fr: 'Salon', en: 'Channel' } }],
      secrets: [{ key: 'webhook', title: { fr: 'Adresse du webhook', en: 'Webhook URL' } }],
    },
  };
}

/** The repository of the Slack plugin, at v1.2.0 then at v1.3.0, which asks for one event more. */
function slackRepo(dir: string) {
  fs.mkdirSync(path.join(dir, 'bin'), { recursive: true });
  git(path.dirname(dir), 'init', '-q', '-b', 'main', dir);
  git(dir, 'config', 'user.email', 't@test');
  git(dir, 'config', 'user.name', 't');
  fs.writeFileSync(path.join(dir, 'bin', 'escouade-slack.exe'), '');
  fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify(slack('1.2.0', ['ticket.column', 'agent.waiting'])));
  git(dir, 'add', '-A');
  git(dir, 'commit', '-qm', 'v1.2.0');
  git(dir, 'tag', 'v1.2.0');
  fs.writeFileSync(
    path.join(dir, 'escouade-plugin.json'),
    JSON.stringify(slack('1.3.0', ['ticket.column', 'agent.waiting', 'agent.created'])),
  );
  git(dir, 'commit', '-qam', 'v1.3.0');
  git(dir, 'tag', 'v1.3.0');
}

/** A theme loaded in development: no program. */
function contrast(dir: string) {
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(
    path.join(dir, 'escouade-plugin.json'),
    JSON.stringify({
      id: 'dev.exemple.raccourcis',
      name: 'Mes raccourcis',
      version: '0.1.0',
      escouade: { api: '1', min: '2.1.0' },
      contributes: { themes: [{ id: 'contrast', name: 'Contraste élevé' }] },
    }),
  );
}

/** Where an element is and how big, rounded: what the app and the preview must share. */
const box = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    const s = getComputedStyle(node);
    return { w: Math.round(r.width), h: Math.round(r.height), font: `${s.fontWeight} ${s.fontSize}`, padding: s.padding };
  });

/** Nothing runs wider than the window. */
const overflow = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll('.plugins-work *, .plugins-side *')]
      .filter((n) => {
        const r = n.getBoundingClientRect();
        return r.width > 0 && r.right > window.innerWidth + 1;
      })
      .map((n) => `${n.tagName.toLowerCase()}.${[...n.classList].join('.')}`),
  );

const still = (page: Page) =>
  page.evaluate(() => {
    const style = document.createElement('style');
    style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
    document.head.append(style);
  });

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string, preview?: Page, shot?: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.mouse.move(2, 400);
  await page.screenshot({ path: path.join(SHOTS, `app-${name}.png`) });
  if (preview && shot) await preview.locator(shot).screenshot({ path: path.join(SHOTS, `preview-${name}.png`) });
}

test('the view Plugins looks as the preview: the installed ones, a plugin’s page, the window of an update', async ({ app }) => {
  test.setTimeout(240_000);
  // As the other looks: without Playwright's Chromium (CI's Windows runner), the preview cannot be drawn.
  test.skip(!fs.existsSync(chromium.executablePath()), 'the preview needs Playwright’s Chromium');
  const { page } = app;
  await addProject(page, app.repo);
  const invoke = invokeOn(page);
  const repo = path.join(app.root, 'escouade-slack');
  slackRepo(repo);
  const dev = path.join(app.root, 'escouade-raccourcis');
  contrast(dev);
  const p = await invoke<{ token: string }>('plugin_inspect_git', { url: repo, tag: 'v1.2.0' });
  await invoke('plugin_install', { token: p.token });
  const d = await invoke<{ token: string }>('plugin_inspect_dev', { path: dev });
  await invoke('plugin_install', { token: d.token });
  await invoke('plugins_check_updates');

  const browser = await chromium.launch();
  const preview = await browser.newPage({ viewport: { width: 1500, height: 900 * 4 + 200 } });
  await preview.goto(pathToFileURL(PREVIEW).href);
  await preview.evaluate(() => document.fonts.ready);

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await still(page);
  const table = page.getByRole('table', { name: 'Ajoutés' });
  await expect(table).toContainText('v1.3.0 disponible');
  await expect(table).toContainText('Mes raccourcis');

  // The installed ones (#b3): the sidebar's items, the head's title, the table's rows and tiles.
  const b3 = preview.locator('#b3');
  expect(await box(page.locator('.plugins-side .it').first())).toMatchObject({ h: (await box(b3.locator('.side .it').first())).h });
  const appH2 = await box(page.locator('.plugins-work h2'));
  const preH2 = await box(b3.locator('.head h2'));
  expect(appH2.font).toBe(preH2.font);
  const appRow = await box(table.locator('.tr:not(.h)').first());
  const preRow = await box(b3.locator('.tr:not(.h)').first());
  expect(Math.abs(appRow.h - preRow.h)).toBeLessThanOrEqual(4);
  expect(appRow.padding).toBe(preRow.padding);
  expect((await box(table.locator('.tr:not(.h) .tile').first())).w).toBe((await box(b3.locator('.tr .ptile').first())).w);
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await shoot(page, `installed-${t}`, preview, '#b3');
  }
  await theme(page, 'dark');

  // A plugin's page (#b2): its tile, its title, its tabs, its aside.
  await table.getByRole('button', { name: /Slack/ }).click();
  const b2 = preview.locator('#b2');
  await expect(page.getByRole('tab', { name: /Présentation/ })).toHaveAttribute('aria-selected', 'true');
  expect((await box(page.locator('.top .tile'))).w).toBe((await box(b2.locator('.phead .ptile'))).w);
  expect((await box(page.locator('.phead h2'))).font).toBe((await box(b2.locator('.phead h2'))).font);
  const appTab = await box(page.getByRole('tab', { name: /Présentation/ }));
  const preTab = await box(b2.locator('.tabs span.on'));
  expect(appTab.padding).toBe(preTab.padding);
  expect(Math.abs(appTab.h - preTab.h)).toBeLessThanOrEqual(2);
  expect((await box(page.locator('aside.pp-aside'))).w).toBe((await box(b2.locator('.pgrid > div').nth(1))).w);
  await expect(page.locator('.act')).toContainText('Mettre à jour vers la v1.3.0');
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await shoot(page, `plugin-${t}`, preview, '#b2');
  }
  await theme(page, 'dark');

  // The window of the update (#b4): what it asks for more, marked; its facts; nothing installed until « Mettre à jour ».
  await page.getByRole('button', { name: 'Mettre à jour vers la v1.3.0' }).click();
  const dialog = page.getByRole('dialog', { name: 'Mettre à jour Slack ?' });
  await expect(dialog).toContainText('Cette version demande plus que la v1.2.0 installée : ton accord est redemandé.');
  await expect(dialog.locator('.asks li:has(.new)')).toContainText('agent.created');
  const b4 = preview.locator('#b4');
  expect((await box(dialog)).w).toBe((await box(b4.locator('.modal'))).w);
  expect((await box(dialog.locator('.facts'))).padding).toBe((await box(b4.locator('.ifacts'))).padding);
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await shoot(page, `install-${t}`, preview, '#b4');
  }
  await dialog.getByRole('button', { name: 'Annuler' }).click();

  // Nothing overflows, at either size, in either theme, on each page.
  for (const size of SIZES) {
    await page.setViewportSize(size);
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      for (const tab of ['Présentation', 'Permissions', 'Versions', 'Projets', 'Réglages', 'Journal']) {
        await page.getByRole('tab', { name: new RegExp(tab) }).click();
        expect(await overflow(page), `${tab} at ${size.width}, ${t}`).toEqual([]);
      }
      await page
        .locator('.plugins-side')
        .getByRole('button', { name: /Installés/ })
        .click();
      expect(await overflow(page), `Installés at ${size.width}, ${t}`).toEqual([]);
      if (size.name) await shoot(page, `installed${size.name}-${t}`);
      await table.getByRole('button', { name: /Slack/ }).click();
      if (size.name) await shoot(page, `plugin${size.name}-${t}`);
    }
  }
  await browser.close();
});
