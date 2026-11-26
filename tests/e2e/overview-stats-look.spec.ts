// « Vue d’ensemble » and « Statistiques » in the 2.0 design (ESC-82, lot 10): the overview full frame, « Attend ta réponse »
// first with Autoriser/Refuser or Répondre on the spot, then a group per project, its columns activity, model, cost,
// context and since when; the statistics with a band of four figures, stacked columns (2 px between segments, a day's
// detail on hover), bars by project and by model and the tables. Compared to the preview (design/app-redesign/index.html,
// `#overview`, `#stats`) in both themes at 1440 × 900 and at CI's 1028 × 779; the English labels in their room at 1028,
// 1280 and 1440 px, nothing overflowing its box; the motion stopped by prefers-reduced-motion. The captures of the app
// and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

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
/** Escouade's teal, the colour of the views over every project, in both. */
const TINT = 'oklch(0.74 0.115 190)';

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
      border: `${s.borderBottomWidth} ${s.borderBottomStyle} ${s.borderBottomColor}`,
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

/** Tints a page with `TINT` at once, nothing gliding, for both to be read alike. */
const still = (page: Page) =>
  page.evaluate((p) => {
    if (!document.getElementById('still')) {
      const style = document.createElement('style');
      style.id = 'still';
      // The preview's Chromium hides its scrollbars; the app's window shows them: hidden in both, for both to be measured alike.
      style.textContent =
        '*, *::before, *::after { transition: none !important; animation: none !important; } main .scroll { scrollbar-width: none; }';
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

/** The preview at `#route`, in theme `t`: from a blank page, for its script to open the route. */
async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

/** The left edges of the cells of a row, from the left of the row. */
const edges = (row: Locator) =>
  row.evaluate((node) => {
    const left = node.getBoundingClientRect().left;
    return [...node.children]
      .filter((c) => getComputedStyle(c).display !== 'none')
      .map((c) => Math.round(c.getBoundingClientRect().left - left));
  });

/**
 * What overflows in `root`: a leaf whose text is wider or taller than its box, or a box out of its parent's, but the
 * names of the user's agents, projects, models and tickets, and what they asked, cut by design.
 */
const overflows = (root: Locator, cut: string) =>
  root.evaluate((root, cut) => {
    const out: string[] = [];
    for (const el of root.querySelectorAll<HTMLElement>('*')) {
      if (!el.offsetParent || el.closest(cut) || el.closest('svg')) continue;
      const name = `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`;
      if (el.children.length === 0 && (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1)) out.push(name);
      const parent = el.parentElement;
      if (!parent || parent === root || getComputedStyle(parent).overflow !== 'visible') continue;
      const r = el.getBoundingClientRect();
      const p = parent.getBoundingClientRect();
      if (r.width && (r.left < p.left - 1 || r.right > p.right + 1)) out.push(`${name} out of its parent`);
    }
    return out;
  }, cut);

/** Agents as the preview's: one asking a permission, one a question, one at work and one ready. */
async function squad(page: Page) {
  await send(page, 'permission');
  await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build');
  await page.keyboard.press('Control+N');
  await expect(page.locator('aside.side .agent')).toHaveCount(2);
  await expect(page.getByText('Agent prêt')).toBeVisible();
  await send(page, 'question');
  await expect(page.getByTestId('question-pending')).toBeVisible();
  await page.keyboard.press('Control+N');
  await expect(page.locator('aside.side .agent')).toHaveCount(3);
  await expect(page.getByText('Agent prêt')).toBeVisible();
  await send(page, 'slow 600s');
  await expect(page.getByText('Je commence…')).toBeVisible();
  await page.keyboard.press('Control+N');
  await expect(page.locator('aside.side .agent')).toHaveCount(4);
  await expect(page.getByText('Agent prêt')).toBeVisible();
}

test('the overview looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(240_000);
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  await squad(page);
  await page.getByRole('button', { name: /^Vue d’ensemble/ }).click();
  const ov = page.locator('main.overview');
  await expect(ov.locator('li.row')).toHaveCount(4);
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await theme(page, t);
        await still(page);
        await page.evaluate(() => document.fonts.ready);
        await page.mouse.move(size.width / 2, 20);
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        await previewAt(preview, 'overview', t);
        const where = `${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#full-overview');

        // The header: its place, its title, what it counts, its keys.
        const head = ov.locator('.head');
        const phead = pv.locator('.full-head');
        expect.soft(await place(head), `header's place, ${where}`).toLookLike(await place(phead));
        expect.soft(await look(head), `header, ${where}`).toLookLike(await look(phead));
        expect.soft(await lookBut(head.locator('h1'), 'width'), `title, ${where}`).toLookLike(await lookBut(phead.locator('h1'), 'width'));
        expect
          .soft(await lookBut(head.locator('.sub'), 'width'), `count, ${where}`)
          .toLookLike(await lookBut(phead.locator('.sub'), 'width'));
        expect
          .soft(await lookBut(head.locator('.keys kbd').first(), 'width'), `a key, ${where}`)
          .toLookLike(await lookBut(phead.locator('.keys kbd').first(), 'width'));
        // The header ends where the preview's keys end: since the Workspace (#45), on its « Workspace » button, after
        // the keys, which the preview does not draw.
        const canvas = head.locator('button.canvas');
        const [ex, , ew] = await place((await canvas.count()) ? canvas : head.locator('.keys'));
        const [pkx, , pkw] = await place(phead.locator('.keys'));
        expect.soft(ex + ew, `header's right edge, ${where}`).toBe(pkx + pkw);

        // The headings of the columns, and where each column starts.
        const cols = ov.locator('.cols');
        const pcols = pv.locator('.ov-cols');
        expect.soft(await place(cols), `columns' place, ${where}`).toLookLike(await place(pcols));
        expect.soft(await look(cols), `columns, ${where}`).toLookLike(await look(pcols));
        expect.soft(await edges(cols), `columns' edges, ${where}`).toLookLike(await edges(pcols));

        // « Attend ta réponse » and a project's group.
        const gh = (el: Locator, waiting: boolean) => el.locator(waiting ? '.waiting h2' : 'section:not(.waiting) h2').first();
        expect
          .soft(await lookBut(gh(ov, true), 'width'), `« Attend ta réponse », ${where}`)
          .toLookLike(await lookBut(gh(pv, true), 'width'));
        expect.soft(await lookBut(gh(ov, false), 'width'), `a project's group, ${where}`).toLookLike(await lookBut(gh(pv, false), 'width'));
        expect
          .soft(await lookBut(ov.locator('.gh .n').first(), 'width'), `a group's count, ${where}`)
          .toLookLike(await lookBut(pv.locator('.ov-group h2 .n').first(), 'width'));
        const [gx, gy] = await place(gh(ov, true));
        const [pgx, pgy] = await place(gh(pv, true));
        expect.soft([gx, gy], `« Attend ta réponse »'s place, ${where}`).toEqual([pgx, pgy]);

        // The permission waiting: its row, its line, its request in a well, its tool, its buttons.
        const perm = ov.locator('li.row.waiting').filter({ has: page.locator('.badge') });
        const pperm = pv.locator('.ov-row.waiting').first();
        expect.soft(await lookBut(perm, 'height'), `the permission's row, ${where}`).toLookLike(await lookBut(pperm, 'height'));
        expect.soft(await look(perm.locator('.line')), `its line, ${where}`).toLookLike(await look(pperm.locator('.ov-line')));
        expect
          .soft(await lookBut(perm.locator('.req'), 'height'), `its request, ${where}`)
          .toLookLike(await lookBut(pperm.locator('.req'), 'height'));
        expect
          .soft(await lookBut(perm.locator('.badge'), 'width'), `its tool, ${where}`)
          .toLookLike(await lookBut(pperm.locator('.badge'), 'width'));
        expect
          .soft(await lookBut(perm.locator('.arg'), 'width', 'height'), `its command, ${where}`)
          .toLookLike(await lookBut(pperm.locator('.arg'), 'width', 'height'));
        expect
          .soft(await lookBut(perm.locator('.btn.primary'), 'width'), `« Autoriser », ${where}`)
          .toLookLike(await lookBut(pperm.locator('.btn.primary'), 'width'));
        expect
          .soft(await lookBut(perm.locator('.btn:not(.primary)'), 'width'), `« Refuser », ${where}`)
          .toLookLike(await lookBut(pperm.locator('.btn:not(.primary)'), 'width'));
        const [bx, , bw] = await place(perm.locator('.btn:not(.primary)'));
        const [pbx, , pbw] = await place(pperm.locator('.btn:not(.primary)'));
        expect.soft(bx + bw, `« Refuser »'s right edge, ${where}`).toBe(pbx + pbw);
        expect
          .soft(await lookBut(perm.locator('.act .txt'), 'width'), `« Demande une autorisation », ${where}`)
          .toLookLike(await lookBut(pperm.locator('.ov-act'), 'width'));
        // The question: its text and « Répondre ».
        const ask = ov.locator('li.row.waiting').filter({ has: page.locator('.ask') });
        const pask = pv.locator('.ov-row.waiting').nth(1);
        expect
          .soft(await lookBut(ask.locator('.ask'), 'width', 'height'), `a question, ${where}`)
          .toLookLike(await lookBut(pask.locator('.q'), 'width', 'height'));

        // An agent at work: its name, its activity, its model, its figures.
        const busy = ov.locator('section:not(.waiting) li.row').filter({ has: page.locator('.dots') });
        const pbusy = pv.locator('.ov-group:not(.waiting) .ov-row:not(.cur)').first();
        expect.soft(await look(busy.locator('.line')), `a line, ${where}`).toLookLike(await look(pbusy.locator('.ov-line')));
        expect
          .soft(await lookBut(busy.locator('.name'), 'width'), `a name, ${where}`)
          .toLookLike(await lookBut(pbusy.locator('.name'), 'width'));
        expect
          .soft(await lookBut(busy.locator('.act'), 'width'), `an activity, ${where}`)
          .toLookLike(await lookBut(pbusy.locator('.ov-act'), 'width'));
        expect
          .soft(await lookBut(busy.locator('.dots span').first(), 'width'), `its dots, ${where}`)
          .toLookLike(await lookBut(pbusy.locator('.dots3 i').first(), 'width'));
        expect
          .soft(await lookBut(busy.locator('.num').first(), 'width'), `a figure, ${where}`)
          .toLookLike(await lookBut(pbusy.locator('.num-c').first(), 'width'));
        expect.soft(await busy.locator('.model').isVisible(), `the model shown, ${where}`).toBe(await pbusy.locator('.mdl').isVisible());
        if (await pbusy.locator('.mdl').isVisible())
          expect
            .soft(await lookBut(busy.locator('.model'), 'width'), `a model, ${where}`)
            .toLookLike(await lookBut(pbusy.locator('.mdl'), 'width'));
        expect.soft(await edges(busy.locator('.line')), `a line's columns, ${where}`).toLookLike(await edges(pbusy.locator('.ov-line')));
        // The line the arrows are on, once chosen (as the preview's, a project's first).
        const chosen = ov.locator('section:not(.waiting) li.row').first();
        await chosen.locator('.line').focus();
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        expect
          .soft(await lookBut(chosen, 'height', 'width'), `the line chosen, ${where}`)
          .toLookLike(await lookBut(pv.locator('.ov-row.cur'), 'height', 'width'));
        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `overview${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
  }
});

test('the statistics look as the preview draws them, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(240_000);
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Tâche terminée')).toBeVisible();
  await page.getByRole('button', { name: 'Stats' }).click();
  const st = page.locator('main.stats');
  await expect(st.locator('.kpi').filter({ hasText: 'Prompts' })).toContainText('1');
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await theme(page, t);
        await still(page);
        await page.evaluate(() => document.fonts.ready);
        await page.mouse.move(size.width / 2, 20);
        await previewAt(preview, 'stats', t);
        const where = `${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#full-stats');

        // The header and the period in three segments.
        const head = st.locator('.head');
        const phead = pv.locator('.full-head');
        expect.soft(await place(head), `header's place, ${where}`).toLookLike(await place(phead));
        expect.soft(await look(head), `header, ${where}`).toLookLike(await look(phead));
        expect.soft(await lookBut(head.locator('h1'), 'width'), `title, ${where}`).toLookLike(await lookBut(phead.locator('h1'), 'width'));
        expect
          .soft(await lookBut(head.locator('.sub'), 'width'), `what it counts, ${where}`)
          .toLookLike(await lookBut(phead.locator('.sub'), 'width'));
        const seg = head.locator('.ranges');
        const pseg = phead.locator('.seg');
        expect.soft(await place(seg), `period's place, ${where}`).toLookLike(await place(pseg));
        expect.soft(await look(seg), `period, ${where}`).toLookLike(await look(pseg));
        for (const sel of ['true', 'false'])
          expect
            .soft(await look(seg.locator(`[aria-selected="${sel}"]`).first()), `a period ${sel}, ${where}`)
            .toLookLike(await look(pseg.locator(`[aria-selected="${sel}"]`).first()));

        // The four figures.
        const kpis = st.locator('.kpis');
        const pkpis = pv.locator('.kpis');
        expect.soft(await place(kpis), `figures' place, ${where}`).toLookLike(await place(pkpis));
        expect.soft(await look(kpis), `figures, ${where}`).toLookLike(await look(pkpis));
        for (const [i, part] of [
          [0, '.lb'],
          [1, '.vl'],
          [1, '.vl small'],
          [1, '.kd'],
        ] as const)
          expect
            .soft(await lookBut(kpis.locator('.kpi').nth(i).locator(part).first(), 'width', 'height'), `a figure's ${part}, ${where}`)
            .toLookLike(await lookBut(pkpis.locator('.kpi').nth(i).locator(part).first(), 'width', 'height'));
        // Their lines, whatever the text of the app's figures: as tall as the preview's.
        expect
          .soft(
            await kpis.locator('.kpi .lb, .kpi .vl').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().height))),
            `figures' lines, ${where}`,
          )
          .toEqual(
            await pkpis.locator('.kpi .lb, .kpi .vl').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().height))),
          );
        expect
          .soft(await look(kpis.locator('.kpi').nth(1)), `a figure's box, ${where}`)
          .toLookLike(await look(pkpis.locator('.kpi').nth(1)));

        // The chart: its heading, legend, « Tableau », its place and its axis.
        const sec = st.locator('section').filter({ has: page.locator('.chart') });
        const psec = pv.locator('section').filter({ has: preview.locator('#chart') });
        expect
          .soft(await lookBut(sec.locator('h2'), 'width'), `chart's title, ${where}`)
          .toLookLike(await lookBut(psec.locator('h2'), 'width'));
        expect.soft(await place(sec.locator('h2')), `its place, ${where}`).toLookLike(await place(psec.locator('h2')));
        expect
          .soft(await lookBut(sec.locator('.legend span').first(), 'width'), `legend, ${where}`)
          .toLookLike(await lookBut(psec.locator('.legend span').first(), 'width'));
        expect
          .soft(await lookBut(sec.locator('.sec-h .btn'), 'width'), `« Tableau », ${where}`)
          .toLookLike(await lookBut(psec.locator('.sec2-h .btn'), 'width'));
        expect.soft(await place(sec.locator('svg')), `chart's place, ${where}`).toLookLike(await place(psec.locator('svg')));
        expect
          .soft(await lookBut(sec.locator('text.ax').first(), 'width', 'height'), `axis, ${where}`)
          .toLookLike(await lookBut(psec.locator('text.ax').first(), 'width', 'height'));
        // The detail of a day on hover.
        await sec.locator('.col-hit').last().hover({ force: true });
        await psec.locator('.col-hit').last().hover({ force: true });
        await expect(sec.locator('.tip')).toBeVisible();
        expect
          .soft(await lookBut(sec.locator('.tip'), 'width', 'height'), `a day's detail, ${where}`)
          .toLookLike(await lookBut(psec.locator('.tip'), 'width', 'height'));
        expect
          .soft(await lookBut(sec.locator('.tip .tot'), 'width'), `its total, ${where}`)
          .toLookLike(await lookBut(psec.locator('.tip .tot'), 'width'));
        await page.mouse.move(size.width / 2, 20);

        // The bars by project, the tables.
        const bar = st.locator('.hbar').first();
        const pbar = pv.locator('.hbar').first();
        expect.soft(await lookBut(bar, 'width'), `a bar's row, ${where}`).toLookLike(await lookBut(pbar, 'width'));
        expect.soft(await edges(bar), `its columns, ${where}`).toLookLike(await edges(pbar));
        for (const part of ['.nm', '.tr', '.vl'])
          expect
            .soft(await lookBut(bar.locator(part), 'width'), `a bar's ${part}, ${where}`)
            .toLookLike(await lookBut(pbar.locator(part), 'width'));
        const [b1x, , b1w] = await place(st.locator('.two > section').last());
        const [pb1x, , pb1w] = await place(pv.locator('.two > section').last());
        expect.soft([b1x, b1w], `« Par modèle »'s place, ${where}`).toEqual([pb1x, pb1w]);
        expect
          .soft(await lookBut(st.locator('.tbl th').first(), 'width'), `a table's heading, ${where}`)
          .toLookLike(await lookBut(pv.locator('.tbl th').first(), 'width'));
        expect
          .soft(await lookBut(st.locator('.tbl td.ttl').first(), 'width'), `a table's cell, ${where}`)
          .toLookLike(await lookBut(pv.locator('.tbl td.ttl').first(), 'width'));
        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `stats${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
  }
});

test('the overview and the statistics stand still with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await squad(page);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.getByRole('button', { name: /^Vue d’ensemble/ }).click();
  const ov = page.locator('main.overview');
  const motion = (el: Locator) => el.evaluate((e) => getComputedStyle(e).animationName);
  const ring = (el: Locator) => el.evaluate((e) => getComputedStyle(e, '::after').animationName);
  await expect(ov.locator('.dots span').first()).toBeVisible();
  expect(await motion(ov.locator('.dots span').first())).toBe('none');
  expect(await ring(ov.locator('.gh .state-dot'))).toBe('none');
  // Its arrows drawn, not written.
  await expect(ov.locator('.keys kbd [data-icon="up"]')).toBeVisible();
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  expect(await motion(ov.locator('.dots span').first())).not.toBe('none');
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the overview and of the statistics fit in their place, at 1028, 1280 and 1440 px', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await send(page, 'permission');
    await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build');
    await page.keyboard.press('Control+N');
    await expect(page.locator('aside.side .agent')).toHaveCount(2);
    await expect(page.getByText('Agent ready')).toBeVisible();
    await send(page, 'Bonjour');
    await expect(page.getByText('Task done')).toBeVisible();
    for (const width of [1440, 1280, 1028]) {
      const size = { width, height: width === 1028 ? 779 : 900 };
      await page.setViewportSize(size);
      await page.getByRole('button', { name: /^Overview/ }).click();
      const ov = page.locator('main.overview');
      await expect(ov.locator('li.row')).toHaveCount(2);
      await page.mouse.move(width / 2, 20);
      expect(await overflows(ov, '.name, .txt, .sub, .arg, .ask, .model, .said, .sr, .state-dot'), `overview ${width}`).toEqual([]);
      await expect(ov.getByRole('button', { name: 'Allow' })).toBeVisible();
      await expect(ov.locator('.cols')).toContainText('Context');
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `overview-${width}-en.app.png`) });
      }
      await page.getByRole('button', { name: 'Stats' }).click();
      const st = page.locator('main.stats');
      await expect(st.locator('.kpi').filter({ hasText: 'Prompts' })).toContainText(/[0-9]/);
      expect(await overflows(st, '.sub, .nm, .cut, .kd, .sr')).toEqual([]);
      // The figures and their labels on one line each, the periods too.
      for (const el of await st.locator('.kpi .lb, .kpi .vl, .ranges button, .tbl th').all())
        expect((await el.boundingBox())!.height, `${await el.textContent()} at ${width}`).toBeLessThanOrEqual(36);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `stats-${width}-en.app.png`) });
    }
  });
});
