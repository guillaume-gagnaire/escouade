// The conversation of the 2.0 design (ESC-76, lot 4): its header (name, state in a pill, path, figures, actions), the
// thread of 780 px (the user's bubble, the calls of a turn in one thread, « Claude travaille… ») and the floating
// composer (model, effort and mode in chips, Stop with Échap, Envoyer in white), as the preview draws them
// (design/app-redesign/index.html), read in both themes at 1440 × 900 and at CI's 1028 × 779; the English labels in
// their room; the motion stopped by prefers-reduced-motion. The captures of the app and of the preview go to
// ESCOUADE_E2E_SHOTS when it is set, to be looked at side by side.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

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
      padding: s.padding,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
    };
  });
}

/** Its look without some of its keys: what differs by content (a width, a height). */
const lookBut = (el: Locator, ...keys: string[]) =>
  look(el).then((l) => Object.fromEntries(Object.entries(l).filter(([k]) => !keys.includes(k))));

/** Where an element is, rounded. */
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

/** An agent at work, after a turn that edited a file: a bubble, a call, an answer, then « Claude travaille… ». */
async function atWork(page: Page) {
  await send(page, 'edit');
  await expect(page.getByText('Fichier modifié.')).toBeVisible();
  await send(page, 'slow 600s');
  await expect(page.locator('main.conv .working')).toBeVisible();
}

test('the conversation looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  await atWork(page);
  const conv = page.locator('main.conv:not(.subview)');
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
        await page.mouse.move(size.width / 2, 20);
        // The field without the focus: its ring is not the preview's at rest.
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        await preview.mouse.move(size.width / 2, 20);
        const where = `${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#view-agents');

        // The header: 60 px, the name over the path, the state in a pill, the figures, the actions.
        const head = conv.locator('header.head');
        const phead = pv.locator('.conv > .head').first();
        expect.soft(await place(head), `header's place, ${where}`).toEqual(await place(phead));
        expect.soft(await look(head), `header, ${where}`).toEqual(await look(phead));
        expect.soft(await lookBut(head.locator('.name'), 'width'), `name, ${where}`).toEqual(await lookBut(pv.locator('#h-name'), 'width'));
        expect
          .soft(await lookBut(head.locator('.state'), 'width'), `state, ${where}`)
          .toEqual(await lookBut(pv.locator('#h-pill'), 'width'));
        expect
          .soft(await lookBut(head.locator('.crumbs'), 'width'), `path, ${where}`)
          .toEqual(await lookBut(pv.locator('.crumbs').first(), 'width'));
        expect
          .soft(await lookBut(head.locator('.fig dt').first(), 'width'), `figure's label, ${where}`)
          .toEqual(await lookBut(pv.locator('.fig dt').first(), 'width'));
        expect
          .soft(await lookBut(head.locator('.fig dd').first(), 'width'), `figure's value, ${where}`)
          .toEqual(await lookBut(pv.locator('.fig dd').first(), 'width'));
        expect
          .soft(await lookBut(head.locator('.ctx-meter'), 'width'), `context's gauge, ${where}`)
          .toEqual(await lookBut(pv.locator('.ctx-meter'), 'width'));
        expect
          // The cache's degrees (ESC-398) are not in the 2.0 preview.
          .soft(await head.locator('.fig:visible:not(.fig-heat)').count(), `figures shown, ${where}`)
          .toBe(await pv.locator('.figures .fig:visible').count());
        const launch = head.locator('.actions .btn.test');
        if (await launch.count())
          expect
            .soft(await lookBut(launch, 'width'), `« Préparer le lancement », ${where}`)
            .toEqual(await lookBut(pv.locator('#h-launch'), 'width'));
        expect.soft(await look(head.locator('.vseg')), `the views, ${where}`).toEqual(await look(pv.locator('#vseg')));
        expect.soft(await look(head.locator('.actions .more')), `« … », ${where}`).toEqual(await look(pv.locator('#h-more')));
        expect
          .soft(await look(head.locator('.layout-seg')), `the layouts, ${where}`)
          .toEqual(await look(pv.locator('.layout-seg').first()));
        const [hx, , hw] = await place(head.locator('.actions'));
        const [px, , pw] = await place(pv.locator('.conv > .head .actions').first());
        expect.soft(hx + hw, `actions' right edge, ${where}`).toBe(px + pw);

        // The thread: one column of 780 px, the user's bubble, the calls of a turn without a box each.
        const [tx, , tw] = await place(conv.locator('.msgs'));
        const [ptx, , ptw] = await place(pv.locator('#th-main'));
        expect.soft([tx, tw], `thread's column, ${where}`).toEqual([ptx, ptw]);
        expect
          .soft(await lookBut(conv.locator('.bubble').first(), 'width', 'height'), `user's bubble, ${where}`)
          .toEqual(await lookBut(pv.locator('#th-main .msg-user'), 'width', 'height'));
        expect
          .soft(await lookBut(conv.locator('.tool .row').first(), 'width'), `a call, ${where}`)
          .toEqual(await lookBut(pv.locator('#th-main .trace .tool').first(), 'width'));
        expect
          .soft(await look(conv.locator('.tool .ic').first()), `a call's icon, ${where}`)
          .toEqual(await look(pv.locator('#th-main .trace .tool .ic').first()));
        expect
          .soft(await lookBut(conv.locator('.tool .badge').first(), 'width'), `a call's verb, ${where}`)
          .toEqual(await lookBut(pv.locator('#th-main .trace .tool .verb').first(), 'width'));
        expect
          .soft(await lookBut(conv.locator('.md p').first(), 'width', 'height'), `the agent's text, ${where}`)
          .toEqual(await lookBut(pv.locator('#th-main .msg-ai p').first(), 'width', 'height'));
        expect
          .soft(await lookBut(conv.locator('.working .shimmer'), 'width'), `« Claude travaille… », ${where}`)
          .toEqual(await lookBut(pv.locator('#th-main .working .shimmer'), 'width'));

        // The composer: floating, the width of the thread; chips, Stop with Échap, Envoyer in white.
        const composer = conv.locator('.composer');
        const pc = pv.locator('#composer');
        const [cx, cy, cw, ch] = await place(composer);
        const [pcx, pcy, pcw, pch] = await place(pc);
        expect.soft([cx, cw, cy + ch], `composer's place, ${where}`).toEqual([pcx, pcw, pcy + pch]);
        expect.soft(await lookBut(composer, 'height'), `composer, ${where}`).toEqual(await lookBut(pc, 'height'));
        expect
          .soft(await lookBut(composer.locator('textarea'), 'height'), `its field, ${where}`)
          .toEqual(await lookBut(pv.locator('#c-text'), 'height'));
        expect
          .soft(await lookBut(composer.locator('.bar'), 'height'), `its bar, ${where}`)
          .toEqual(await lookBut(pc.locator('.c-bar'), 'height'));
        expect
          .soft(await lookBut(composer.locator('.bar .trigger').first(), 'width'), `the model's chip, ${where}`)
          .toEqual(await lookBut(pv.locator('#chip-model'), 'width'));
        expect.soft(await look(composer.locator('.attach')), `join a file, ${where}`).toEqual(await look(pc.locator('.chip.ic').first()));
        expect
          .soft(await lookBut(composer.locator('.btn.stop'), 'width'), `Stop, ${where}`)
          .toEqual(await lookBut(pc.locator('.btn.stop'), 'width'));
        expect
          .soft(await lookBut(composer.locator('.btn.send'), 'width'), `Envoyer, ${where}`)
          .toEqual(await lookBut(pc.locator('.btn.send'), 'width'));

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `agents${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
    await page.getByRole('button', { name: 'Stop' }).click();
  }
});

test('« … » of the header opens the agent’s menu under it, the one of its card', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'bonjour');
  await expect(page.getByText('Bonjour, tu as dit : bonjour')).toBeVisible();
  const more = page.locator('main.conv header.head').getByRole('button', { name: 'Actions de l’agent' });
  await more.click();
  const menu = page.getByRole('menu');
  await expect(menu).toBeVisible();
  await expect(menu.getByRole('menuitem', { name: 'Dupliquer la conversation' })).toBeVisible();
  await expect(menu.getByRole('menuitem', { name: 'Supprimer…' })).toBeVisible();
  // Under the button, inside the window.
  const [m, b] = [(await menu.boundingBox())!, (await more.boundingBox())!];
  expect(m.y).toBeGreaterThanOrEqual(b.y + b.height);
  expect(m.x + m.width).toBeLessThanOrEqual(await page.evaluate(() => innerWidth));
  await page.keyboard.press('Escape');
  await expect(menu).toBeHidden();
});

test('« Claude travaille… » and the spark stop with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'slow 600s');
  const working = page.locator('main.conv .working');
  await expect(working).toBeVisible();
  const motion = () =>
    working.evaluate((el) => [el.querySelector('.spark')!, el.querySelector('.shimmer')!].map((e) => getComputedStyle(e).animationName));
  // Their keyframes, named by Svelte after the component.
  const [spin, shimmer] = await motion();
  expect(spin).toMatch(/spin/);
  expect(shimmer).toMatch(/shimmer/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  expect(await motion()).toEqual(['none', 'none']);
  // Still read: its words in the muted grey, no reflection over them.
  expect(await working.locator('.shimmer').evaluate((e) => getComputedStyle(e).backgroundImage)).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.getByRole('button', { name: 'Stop' }).click();
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the header and of the composer fit in their place, at both sizes', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await send(page, 'edit');
    await expect(page.getByText('Fichier modifié.')).toBeVisible();
    await send(page, 'slow 600s');
    const conv = page.locator('main.conv:not(.subview)');
    await expect(conv.locator('.working')).toContainText('Claude is working');
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await expect(conv.locator('.composer .btn.send')).toContainText('Send');
      await expect(conv.locator('header.head .fig dt').first()).toHaveText('Context');
      // What is cut short, or overflows its box: none of it; the header holds on one line.
      const cut = await conv.evaluate((root) =>
        [...root.querySelectorAll<HTMLElement>('header.head *, .composer *')]
          .filter(
            (el) =>
              el.offsetParent &&
              el.children.length === 0 &&
              !el.closest('.mono, .name, .branch, textarea') &&
              (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
          )
          .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
      );
      expect(cut, `${size.width}×${size.height}`).toEqual([]);
      const head = conv.locator('header.head');
      expect(await head.evaluate((h) => h.scrollWidth <= h.clientWidth && h.getBoundingClientRect().height < 70)).toBe(true);
      // The composer's bar: its chips, then its buttons, on two lines at most.
      const [bar, chip] = [
        (await conv.locator('.composer .bar').boundingBox())!,
        (await conv.locator('.composer .bar .trigger').first().boundingBox())!,
      ];
      expect(bar.height).toBeLessThanOrEqual(2 * chip.height + 20);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `agents${size.name}-en.app.png`) });
    }
    await page.getByRole('button', { name: 'Stop' }).click();
  });
});
