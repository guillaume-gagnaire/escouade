// A project's label under its tile (ESC-168), set in Réglages › Projets › Projet, then the rail's width chosen by its right edge:
// the labels under the tiles on a narrow rail, beside them from 160 px, the width kept after a restart and given back by
// a double click, held to the window's room. At 1440 × 900 and in the runner's window (its screen is 1024 × 768).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';

const tile = (page: Page, name = 'demo-api') => page.getByRole('tab', { name: new RegExp(`^${name}`) });
const rail = (page: Page) => page.getByRole('navigation', { name: 'Projets' });
const edge = (page: Page) => page.getByRole('separator', { name: 'Largeur de la barre des projets' });
const width = async (page: Page) => Math.round((await rail(page).boundingBox())!.width);

/** A git repository with one commit, named `name`, in the test's folder. */
function repoNamed(root: string, name: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

/** Opens the settings on « Projets ». */
async function projectsSettings(page: Page) {
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  if (!(await settings.isVisible())) await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await openSettingsPage(settings, 'Projet');
  return settings;
}

/** Two frames: the container queries of a new width apply. */
const settle = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/**
 * What overflows in the window: the page scrolled sideways, a label on two lines or out of the rail, a label cut but
 * without its « … », the rail over the sidebar, the sidebar over the work, the agent's header cut.
 */
function overflows(page: Page) {
  return page.evaluate(() => {
    const out: string[] = [];
    const doc = document.scrollingElement!;
    if (doc.scrollWidth > window.innerWidth + 1) out.push(`the page scrolls: ${doc.scrollWidth} > ${window.innerWidth}`);
    const nav = document.querySelector('nav.rail')!.getBoundingClientRect();
    for (const l of document.querySelectorAll<HTMLElement>('nav.rail .lbl')) {
      const r = l.getBoundingClientRect();
      if (r.width < 1) continue;
      const tx = l.querySelector<HTMLElement>('.tx')!;
      if (r.height > 20) out.push(`« ${tx.textContent} » on two lines (${r.height} px)`);
      if (r.left < nav.left - 1 || r.right > nav.right + 1) out.push(`« ${tx.textContent} » out of the rail: ${r.left}–${r.right}`);
      if (tx.scrollWidth > tx.clientWidth + 1 && getComputedStyle(tx).textOverflow !== 'ellipsis')
        out.push(`« ${tx.textContent} » cut without « … »`);
    }
    const side = document.querySelector('.side')?.getBoundingClientRect();
    const work = document.querySelector('.work')!.getBoundingClientRect();
    if (side && side.left < nav.right - 1) out.push(`the sidebar under the rail: ${side.left} < ${nav.right}`);
    if (side && side.right > work.left + 1) out.push(`the sidebar over the work: ${side.right} > ${work.left}`);
    if (work.right > window.innerWidth + 1) out.push(`the work out of the window: ${work.right}`);
    const head = document.querySelector<HTMLElement>('main.conv header.head');
    if (head && head.scrollWidth > head.clientWidth + 1) out.push(`the agent's header cut: ${head.scrollWidth} > ${head.clientWidth}`);
    return out;
  });
}

/** The colour `value` (a CSS colour or variable) computes to, as the page writes a colour. */
const colorOf = (page: Page, value: string) =>
  page.evaluate((v) => {
    const probe = document.createElement('span');
    probe.style.color = v;
    document.querySelector('nav.rail')!.append(probe);
    const c = getComputedStyle(probe).color;
    probe.remove();
    return c;
  }, value);

for (const size of [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
])
  test(`a project's label is set in the settings and shown under its tile, then the rail is widened by its edge, at ${size.width} × ${size.height}`, async ({
    app,
  }) => {
    test.setTimeout(150_000);
    let { page } = app;
    await page.setViewportSize(size);
    await addProject(page, app.repo);
    await settle(page);
    // Off by default: the tile alone, the rail of 52 px.
    await expect(tile(page)).not.toHaveClass(/labelled/);
    await expect(tile(page).locator('.lbl')).toBeHidden();
    expect(await width(page)).toBe(52);

    // On: the name under the tile, the rail of 72 px, the active one's in the project's ink.
    let settings = await projectsSettings(page);
    const on = settings.getByRole('switch', { name: 'Libellé sous la tuile' });
    await on.scrollIntoViewIfNeeded();
    await expect(on).toHaveAttribute('aria-checked', 'false');
    await on.click();
    await closeSettings(settings);
    const label = tile(page).locator('.lbl');
    await expect(label).toBeVisible();
    await expect(label).toHaveText('demo-api');
    await settle(page);
    expect(await width(page)).toBe(72);
    // The settings were tinted with Escouade's teal (ESC-379): the project's colour glides back first (0.7 s).
    await expect
      .poll(async () => (await label.evaluate((l) => getComputedStyle(l).color)) === (await colorOf(page, 'var(--p-ink)')))
      .toBe(true);
    // The tile keeps its size, under it its label.
    const t = (await tile(page).boundingBox())!;
    expect([Math.round(t.width), Math.round(t.height)]).toEqual([36, 36]);
    expect((await label.boundingBox())!.y).toBeGreaterThan(t.y + t.height);
    expect(await overflows(page)).toEqual([]);

    // A text of one's own, twelve letters at most, cut in « … » under the tile.
    settings = await projectsSettings(page);
    const text = settings.getByRole('textbox', { name: 'Texte du libellé' });
    await text.scrollIntoViewIfNeeded();
    await expect(text).toHaveAttribute('placeholder', 'demo-api');
    await text.pressSequentially('pagination-service');
    await expect(text).toHaveValue('pagination-s');
    await closeSettings(settings);
    await expect(label).toHaveText('pagination-s');
    expect(await label.locator('.tx').evaluate((x) => x.scrollWidth > x.clientWidth)).toBe(true);
    expect(await overflows(page)).toEqual([]);
    expect(fs.existsSync(path.join(app.repo, '.escouade.yml'))).toBe(false);

    // Its edge dragged to the right: as wide as the window lets it, 300 px at most.
    const widest = Math.min(300, size.width - 948);
    let e = (await edge(page).boundingBox())!;
    await page.mouse.move(e.x + e.width / 2, e.y + e.height / 2);
    await page.mouse.down();
    await page.mouse.move(e.x + 120, e.y + e.height / 2, { steps: 4 });
    await page.mouse.move(e.x + 400, e.y + e.height / 2, { steps: 4 });
    await page.mouse.up();
    await settle(page);
    expect(await width(page)).toBe(widest);
    await expect(edge(page)).toHaveAttribute('aria-valuenow', String(widest));
    if (widest >= 160) {
      // A list: the name on the tile's right, whole now, the other tiles' names too.
      const l = (await label.boundingBox())!;
      const tt = (await tile(page).boundingBox())!;
      expect(l.x).toBeGreaterThan(tt.x + tt.width);
      expect(Math.abs(l.y + l.height / 2 - (tt.y + tt.height / 2))).toBeLessThan(2);
      expect(await label.locator('.tx').evaluate((x) => x.scrollWidth > x.clientWidth)).toBe(false);
      await expect(rail(page).getByRole('button', { name: 'Vue d’ensemble' }).locator('.lbl')).toBeVisible();
      await expect(rail(page).getByRole('button', { name: 'Ajouter un projet' }).locator('.lbl')).toBeVisible();
      // A click on a name opens what its tile opens, not only one on the tile.
      const overview = rail(page).getByRole('button', { name: 'Vue d’ensemble' });
      await overview.locator('.lbl').click();
      await expect(overview).toHaveAttribute('aria-pressed', 'true');
      await label.click();
      await expect(tile(page)).toHaveAttribute('aria-selected', 'true');
      await expect(overview).toHaveAttribute('aria-pressed', 'false');
      // Out of the button the click left the pointer on, for its tooltip to come anew.
      await page.mouse.move(size.width - 20, size.height / 2);
      // Its tooltip beside the rail's edge, over none of the names.
      await tile(page).hover();
      const tip = page.getByRole('tooltip');
      await expect(tip).toBeVisible();
      expect((await tip.boundingBox())!.x).toBeGreaterThanOrEqual((await rail(page).boundingBox())!.x + widest - 8);
      await page.mouse.move(size.width - 20, size.height / 2);
    } else {
      // The window has no room for a list: the labels stay under the tiles.
      expect((await label.boundingBox())!.y).toBeGreaterThan(t.y + t.height);
    }
    expect(await overflows(page)).toEqual([]);
    // A shortcut still chooses the project.
    await page.keyboard.press('Control+1');
    await expect(tile(page)).toHaveAttribute('aria-selected', 'true');

    // Kept after a restart, for the whole app.
    page = await app.restart();
    await page.setViewportSize(size);
    await settle(page);
    await expect(tile(page).locator('.lbl')).toHaveText('pagination-s');
    expect(await width(page)).toBe(widest);

    // The keyboard: an arrow moves it by 16 px.
    await edge(page).focus();
    await page.keyboard.press('ArrowLeft');
    await settle(page);
    expect(await width(page)).toBe(widest - 16);

    // A double click: its usual width, which follows the labels (72 px with one).
    e = (await edge(page).boundingBox())!;
    await page.mouse.dblclick(e.x + e.width / 2, e.y + e.height / 2);
    await settle(page);
    expect(await width(page)).toBe(72);
    expect(await overflows(page)).toEqual([]);
  });

test('twelve projects in the runner’s window: the rail at its widest scrolls, the active one in view, nothing overflows', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  const names = [
    'demo-api',
    'studio-web',
    'mobile-app',
    'infra',
    'billing',
    'auth',
    'design',
    'pipeline',
    'marketing',
    'models',
    'search',
    'payments',
  ];
  for (const name of names)
    await addProject(page, name === 'demo-api' ? app.repo : repoNamed(app.root, name), { firstAgent: name === 'demo-api' });
  // A label on the first, and the rail as wide as the window lets it.
  const settings = await projectsSettings(page);
  await openSettingsPage(settings, 'Projet', { project: 'demo-api' });
  const on = settings.getByRole('switch', { name: 'Libellé sous la tuile' });
  await on.scrollIntoViewIfNeeded();
  await on.click();
  await closeSettings(settings);
  await edge(page).focus();
  await page.keyboard.press('End');
  await settle(page);
  expect(await width(page)).toBe(76);
  // The list scrolls; the active project (the last added) in view.
  const list = page.getByRole('tablist', { name: 'Projets ouverts' });
  expect(await list.evaluate((l) => l.scrollHeight > l.clientHeight)).toBe(true);
  await expect(tile(page, 'payments')).toBeInViewport();
  await page.keyboard.press('Control+1');
  await expect(tile(page)).toHaveAttribute('aria-selected', 'true');
  await expect(tile(page)).toBeInViewport();
  await expect(tile(page).locator('.lbl')).toBeVisible();
  expect(await overflows(page)).toEqual([]);
});
