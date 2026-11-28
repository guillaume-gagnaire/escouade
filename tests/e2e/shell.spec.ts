// The shell of the 2.0 design (ESC-72, lot 2): the title bar, the projects' rail, the status bar and its quotas, as the
// preview draws them (design/app-redesign/index.html), read in both themes at 1440 × 900 and at CI's 1028 × 779; the
// English labels in their room; the motion stopped by prefers-reduced-motion. The captures of the app and of the preview
// go to ESCOUADE_E2E_SHOTS when it is set, to be looked at side by side.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
/** The preview loads Inter from the site's packages: without them, its text is not the app's. */
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];
/** The colour both are tinted with: the preview's first project's, the app's first project's. */
const TINT = 'oklch(0.72 0.12 48)';

/** The look of an element, as the browser computes it: what the app and the preview must share. */
function look(el: Locator) {
  return el.evaluate((node) => {
    const s = getComputedStyle(node);
    const r = node.getBoundingClientRect();
    return {
      width: Math.round(r.width),
      height: Math.round(r.height),
      background: s.backgroundColor,
      color: s.color,
      shadow: s.boxShadow,
      radius: s.borderRadius,
      border: `${s.borderTopWidth} ${s.borderTopStyle} ${s.borderTopColor}`,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
    };
  });
}

/** Where an element is, rounded: the places of the shell. */
const place = (el: Locator) =>
  el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    return [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)];
  });

/** Tints a page with `TINT` at once, nothing gliding nor breathing, for both to be read alike. */
const still = (page: Page) =>
  page.evaluate((p) => {
    if (!document.getElementById('still')) {
      const style = document.createElement('style');
      style.id = 'still';
      style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
      document.head.append(style);
    }
    document.documentElement.style.setProperty('--p', p);
  }, TINT);

/** The theme of the app, set as « Thème » would (its effect without its dialog). */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

test('the title bar, the projects’ rail and the status bar look as the preview draws them, in both themes, at both sizes', async ({
  app,
}) => {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}`);
        await preview.evaluate(() => document.fonts.ready);
        await theme(page, t);
        await still(page);
        await still(preview);
        await page.evaluate(() => document.fonts.ready);
        await page.mouse.move(size.width / 2, size.height / 2);
        const where = `${t} ${size.width}×${size.height}`;

        // The places: the title bar across the window, the rail under it on the left, the status bar at the bottom,
        // the sidebar beside the rail.
        expect(await place(page.locator('header.bar')), `title bar, ${where}`).toLookLike(await place(preview.locator('header.titlebar')));
        expect(await place(page.locator('nav.rail')), `rail, ${where}`).toLookLike(await place(preview.locator('nav.prail')));
        expect(await place(page.locator('footer.bar')), `status bar, ${where}`).toLookLike(
          await place(preview.locator('footer.statusbar')),
        );
        expect(await place(page.locator('aside.side')), `sidebar, ${where}`).toLookLike(await place(preview.locator('aside.sidebar')));

        // The tiles: the active project's, « + », the overview's and the settings', in their places.
        const appTile = page.getByRole('tab', { name: /^demo-api/ });
        const prevTile = preview.locator('.prail .tile.proj[aria-selected="true"]');
        expect(await look(appTile), `active tile, ${where}`).toLookLike(await look(prevTile));
        expect((await place(appTile)).slice(0, 1), `active tile's x, ${where}`).toLookLike((await place(prevTile)).slice(0, 1));
        for (const [name, sel] of [
          ['Vue d’ensemble', '#btn-overview'],
          ['Ajouter un projet', '#r-add'],
          ['Réglages (Ctrl+,)', '#btn-settings'],
        ]) {
          const a = page.getByRole('button', { name, exact: true });
          const p = preview.locator(sel);
          expect(await look(a), `${name}, ${where}`).toLookLike(await look(p));
          if (name !== 'Ajouter un projet') expect(await place(a), `${name}'s place, ${where}`).toLookLike(await place(p));
        }

        // The title bar: the project's name (its button), the search in the middle, the editor.
        const appName = page.locator('header .proj');
        const prevName = preview.locator('#tb-proj');
        const name = (l: Locator) => look(l).then(({ width, ...rest }) => (void width, rest));
        expect(await name(appName), `project's name, ${where}`).toLookLike(await name(prevName));
        expect((await place(appName)).slice(0, 2), `project's name's place, ${where}`).toLookLike((await place(prevName)).slice(0, 2));
        const appSearch = page.locator('header .search');
        const prevSearch = preview.locator('.titlebar .search');
        expect(await look(appSearch), `search, ${where}`).toLookLike(await look(prevSearch));
        expect(await place(appSearch), `search's place, ${where}`).toLookLike(await place(prevSearch));
        const editor = (l: Locator) => look(l).then(({ width, ...rest }) => (void width, rest));
        expect(await editor(page.getByRole('button', { name: 'Éditeur ou workspace du projet' })), `editor, ${where}`).toLookLike(
          await editor(preview.locator('.titlebar [data-go="editor"]')),
        );

        // The status bar: its text, the waiting button, a gauge.
        const bar = (l: Locator) => look(l).then(({ width, ...rest }) => (void width, rest));
        expect(await bar(page.locator('footer.bar')), `status bar, ${where}`).toLookLike(await bar(preview.locator('footer.statusbar')));
        const gauge = (l: Locator) => look(l).then(({ font, color, ...rest }) => (void font, void color, rest));
        expect(await gauge(page.locator('footer .quotas .gauge').first()), `gauge, ${where}`).toLookLike(
          await gauge(preview.locator('#sb-q .meter').first()),
        );

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `agents${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
  }
});

test('at the first start the rail keeps « + », « Aide » and the settings, the title bar its logo and the window’s buttons', async ({
  app,
}) => {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await preview.setViewportSize(size);
      await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=dark#welcome`);
      await preview.evaluate(() => document.fonts.ready);
      await theme(page, 'dark');
      await still(page);
      await still(preview);
      const rail = page.getByRole('navigation', { name: 'Projets' });
      // « Aide » (ESC-504) is not in the 2.0 preview: the others stay where it has them.
      await expect(rail.getByRole('button')).toHaveCount(3);
      expect(await rail.getByRole('button').evaluateAll((b) => b.map((x) => x.getAttribute('aria-label')))).toLookLike([
        'Ajouter un projet',
        'Aide',
        'Réglages (Ctrl+,)',
      ]);
      expect(await place(page.getByRole('button', { name: 'Ajouter un projet', exact: true }))).toLookLike(
        await place(preview.locator('#r-add')),
      );
      expect(await place(page.getByRole('button', { name: 'Réglages (Ctrl+,)' }))).toLookLike(
        await place(preview.locator('#btn-settings')),
      );
      await expect(page.locator('header .search')).toHaveCount(0);
      // The panel of the work, alone, where the preview has it.
      expect(await place(page.locator('.work'))).toLookLike(await place(preview.locator('main.panel')));
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `welcome${size.name}.app.png`) });
        await preview.screenshot({ path: path.join(SHOTS, `welcome${size.name}.preview.png`) });
      }
    }
  } finally {
    await browser.close();
  }
});

test('the tile’s ring of a project to look at and the waiting dot stop with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const motion = () =>
    page.evaluate(() => {
      const tile = document.querySelector<HTMLElement>('[role="tab"]')!;
      tile.classList.add('alert');
      tile.style.setProperty('--alert', 'var(--wait)');
      const out = [getComputedStyle(tile).animationName, getComputedStyle(document.documentElement).transitionDuration];
      tile.classList.remove('alert');
      return out;
    });
  expect(await motion()).toEqual([expect.stringMatching(/tile-alert/), '0.7s']);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  expect((await motion())[0]).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the shell fit in their place at 1028 × 779, the quota panel open', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await page.setViewportSize({ width: 1028, height: 779 });
    await expect(page.getByRole('button', { name: 'Search the conversations (Ctrl+K)' })).toContainText('Search');
    await expect(page.locator('header.bar')).toContainText('Editor');
    await page.getByRole('button', { name: /^Quota by account/ }).click();
    const panel = page.getByRole('dialog', { name: 'Accounts quota' });
    await expect(panel).toBeVisible();
    // What is cut short, or overflows its box: none of it.
    const cut = await page.evaluate(() =>
      [
        ...document.querySelectorAll<HTMLElement>(
          'header button, header kbd, nav.rail button, footer button, footer .it, [role="dialog"] *',
        ),
      ]
        .filter(
          (el) =>
            el.offsetParent && el.children.length === 0 && (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
        )
        .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
    );
    expect(cut).toEqual([]);
    // The panel stays in the window.
    const box = (await panel.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(1028);
    // Captured once risen (a panel half-way up shows what is under it).
    await page.evaluate(() =>
      Promise.all(
        document
          .getAnimations()
          .filter((a) => a.effect?.getComputedTiming().iterations !== Infinity)
          .map((a) => a.finished.catch(() => {})),
      ),
    );
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'quota-small-en.app.png') });
  });
});
