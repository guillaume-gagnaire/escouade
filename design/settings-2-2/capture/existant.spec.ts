// The settings window of main, tab by tab, in both themes at the app's default size (1440 × 900), as the user sees it
// (existant/<tab>.png, existant/<tab>-light.png); each long tab also whole, its body unrolled (existant/<tab>-entier.png);
// and what each tab holds, measured (existant/mesures.json): the height of its body, its groups, its rows.

import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from '../../../tests/e2e/fixture';

const OUT = path.resolve(import.meta.dirname, '..', 'existant');
const TABS = [
  ['app', 'Application'],
  ['claude', 'Harness'],
  ['accounts', 'Comptes'],
  ['voice', 'Voix'],
  ['notifications', 'Notifications'],
  ['projects', 'Projets'],
  ['board', 'Kanban'],
  ['integrations', 'Intégrations'],
  ['plugins', 'Plugins'],
  ['mcp', 'Serveur MCP'],
  ['terminals', 'Terminaux'],
  ['network', 'Réseau'],
  ['about', 'À propos'],
] as const;

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

const still = (page: Page) =>
  page.evaluate(() => {
    if (document.getElementById('still')) return;
    const style = document.createElement('style');
    style.id = 'still';
    style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
    document.head.append(style);
  });

/** The folder of the fake harnesses, shown as the demo's: nothing of the machine in a capture (as tests/site-shots). */
const FIXTURES = path.resolve(import.meta.dirname, '..', '..', '..', 'tests', 'fixtures');

async function openTab(page: Page, name: string) {
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  if (!(await dialog.isVisible())) await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await dialog.getByRole('tab', { name, exact: true }).click();
  await page.mouse.move(2, 400);
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.waitForTimeout(400);
  await page.evaluate((fixtures) => {
    const hide = (s: string) => s.split(fixtures).join('C:\\escouade-demo\\bin');
    const walk = document.createTreeWalker(document.querySelector('.settings') as Node, NodeFilter.SHOW_TEXT);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) if (n.nodeValue?.includes(fixtures)) n.nodeValue = hide(n.nodeValue);
    for (const i of document.querySelectorAll<HTMLInputElement>('.settings input')) if (i.value.includes(fixtures)) i.value = hide(i.value);
  }, FIXTURES);
}

test('the settings of main, captured', async ({ app }) => {
  const { page } = app;
  fs.mkdirSync(OUT, { recursive: true });
  await page.setViewportSize({ width: 1440, height: 900 });
  await addProject(page, app.repo);
  await still(page);
  await page.evaluate(() => document.fonts.ready);
  const measures: Record<string, unknown> = {};
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    for (const [id, name] of TABS) {
      await openTab(page, name);
      const win = page.locator('.settings[role="dialog"]');
      await expect(win).toBeVisible();
      await win.screenshot({ path: path.join(OUT, `${id}${t === 'light' ? '-light' : ''}.png`) });
      if (t === 'dark') {
        measures[id] = await page.evaluate(() => {
          const body = document.querySelector('.s-body') as HTMLElement;
          const groups = [...body.querySelectorAll('.sg')].map((g) => g.querySelector('h2')?.textContent?.trim() || g.getAttribute('aria-label') || '(sans titre)');
          return {
            visible: body.clientHeight,
            total: body.scrollHeight,
            screens: Math.round((body.scrollHeight / body.clientHeight) * 10) / 10,
            groups,
            rows: body.querySelectorAll('.sr').length,
            controls: body.querySelectorAll('input, select, textarea, button, [role="switch"]').length,
          };
        });
      }
    }
  }
  // The long tabs, whole: the window as tall as its body (dark only).
  await theme(page, 'dark');
  for (const [id, name] of TABS) {
    const m = measures[id] as { screens: number };
    if (m.screens < 1.3) continue;
    await openTab(page, name);
    const total = await page.evaluate(() => (document.querySelector('.s-body') as HTMLElement).scrollHeight);
    await page.setViewportSize({ width: 1440, height: Math.min(total + 320, 12000) });
    await page.addStyleTag({ content: '.settings { height: auto !important; max-height: none !important; } .s-body { overflow: visible !important; max-height: none !important; }' });
    await page.waitForTimeout(300);
    await page.locator('.settings[role="dialog"]').screenshot({ path: path.join(OUT, `${id}-entier.png`) });
    await page.evaluate(() => document.querySelectorAll('style').forEach((s) => s.textContent?.includes('.settings { height: auto') && s.remove()));
    await page.setViewportSize({ width: 1440, height: 900 });
  }
  fs.writeFileSync(path.join(OUT, 'mesures.json'), JSON.stringify(measures, null, 2) + '\n');
});
