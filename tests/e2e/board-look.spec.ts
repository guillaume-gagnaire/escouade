// The Kanban of the 2.0 design (ESC-78, lot 6): columns without a box, cards with their key, their loop in squares, their
// criteria checked, what their agent does and what it costs, the ticket to test in amber with « Valider » and « Tester »,
// the band of the .escouade.yml commands that wait (choice 29), and the ticket's form. Compared to the preview
// (design/app-redesign/index.html, `#escouade-yml-board` and `#ticket`) in both themes at 1440 × 900 and at CI's
// 1028 × 779; the English labels in their room; the motion stopped by prefers-reduced-motion. The captures of the app and
// of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, FAKE_CODEX, test } from './fixture';

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

/** A command of the repository's .escouade.yml, which waits for the user's trust: the band of the Kanban says so. */
const SHARED = `version: 1
worktrees:
  setup:
    - node -e "1"
`;

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

/** Writes `.escouade.yml` in the repository, committed, before the project is added. */
function share(repo: string) {
  fs.writeFileSync(path.join(repo, '.escouade.yml'), SHARED);
  execFileSync('git', ['add', '-A'], { cwd: repo });
  execFileSync('git', ['commit', '-qm', 'escouade'], { cwd: repo });
}

/** A ticket made from the « + » of « À faire »; `after`, the key of a ticket it comes after. */
async function newTicket(page: Page, title: string, after?: string) {
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
  if (after) {
    // In the popover of « Après », which a click elsewhere closes.
    await dialog.getByRole('button', { name: /^Après / }).click();
    await dialog.getByRole('checkbox', { name: new RegExp(after) }).check();
  }
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(dialog).toBeHidden();
}

/**
 * A board as the preview's: one ticket done (merged), one to test, one under way (a slow turn), one to do that waits for
 * it. Returns its columns.
 */
async function fullBoard(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  await newTicket(page, 'Réinitialisation du mot de passe [ok]');
  await col('À tester').getByRole('button', { name: 'Valider et merger' }).click({ timeout: 60_000 });
  await expect(col('Terminé').locator('.card')).toHaveCount(1, { timeout: 60_000 });
  await newTicket(page, 'Recherche par e-mail dans /users [ok]');
  await expect(col('À tester').locator('.card')).toHaveCount(1, { timeout: 60_000 });
  await newTicket(page, 'Limiter les tentatives de connexion [lent]');
  await expect(col('En cours').locator('.card')).toHaveCount(1, { timeout: 60_000 });
  await newTicket(page, 'Journal d’audit des connexions', 'DEM-3');
  await expect(col('À faire').locator('.card')).toContainText('Après DEM-3');
  return { todo: col('À faire'), doing: col('En cours'), review: col('À tester'), done: col('Terminé') };
}

test('the Kanban looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(240_000);
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  share(app.repo);
  await addProject(page, app.repo, { firstAgent: false });
  const cols = await fullBoard(page);
  const board = page.locator('main.board');
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#escouade-yml-board`);
        await preview.evaluate(() => document.fonts.ready);
        await theme(page, t);
        await still(page);
        await still(preview);
        await page.evaluate(() => document.fonts.ready);
        await page.mouse.move(size.width / 2, size.height - 4);
        await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
        await preview.mouse.move(size.width / 2, size.height - 4);
        const where = `${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#view-board');

        // The header, on two rows since ESC-390 (direction A of design/kanban-envs-2-2): the title, its line, « Importer »
        // and the autopilot; then the views, the places and what validating does. Its first row keeps the 2.0 preview's
        // title, line, « Importer » and switch; the second is the new design's, checked by its places.
        const head = board.locator('> .head');
        const phead = pv.locator('.kb-head');
        const [hx, hy, hw] = await place(head);
        const [px, py, pw] = await place(phead);
        expect.soft([hx, hy, hw], `header's place, ${where}`).toEqual([px, py, pw]);
        expect.soft((await place(head.locator('.h1')))[3], `its first row, ${where}`).toBe(52);
        expect.soft((await place(head.locator('.h2')))[3], `its second row, ${where}`).toBe(40);
        expect.soft(await lookBut(head.locator('h1'), 'width'), `title, ${where}`).toEqual(await lookBut(phead.locator('h1'), 'width'));
        expect
          .soft(await lookBut(head.locator('.crumbs'), 'width'), `its line, ${where}`)
          .toEqual(await lookBut(phead.locator('.crumbs'), 'width'));
        if (size.width > 1100) {
          expect.soft(await look(head.locator('.imp')), `« Importer », ${where}`).toEqual(await look(phead.locator('.actions > .btn')));
          expect
            .soft(await look(head.locator('.imp .icon')), `its icon, ${where}`)
            .toEqual(await look(phead.locator('.actions > .btn svg')));
        }
        // The second row: the views on the left, the places and « Après validation » on the right, nothing over another.
        const row = await head.locator('.h2').evaluate((r) =>
          [...r.querySelectorAll(':scope > *:not(.sp)')]
            .map((e) => e.getBoundingClientRect())
            .filter((b) => b.width > 0)
            .map((b) => [Math.round(b.left), Math.round(b.right)]),
        );
        for (let i = 1; i < row.length; i++)
          expect.soft(row[i][0], `second row, item ${i}, ${where}`).toBeGreaterThanOrEqual(row[i - 1][1]);
        expect.soft(await head.locator('.cfg .v').isVisible(), `its summary, ${where}`).toBe(true);
        expect.soft(await look(head.locator('.auto')), `autopilot, ${where}`).toEqual(await look(phead.locator('.switch')));
        expect.soft(await look(head.locator('.knob')), `its switch, ${where}`).toEqual(await look(phead.locator('.switch .knob')));
        // Right edges: the controls end where the preview's do.
        const [ax, , aw] = await place(head.locator('.auto'));
        const [pax, , paw] = await place(phead.locator('.switch'));
        expect.soft(ax + aw, `autopilot's right edge, ${where}`).toBe(pax + paw);

        // The band of the commands of .escouade.yml, under the header.
        const band = board.locator('.band.shared');
        const pband = pv.locator('#yml-banner');
        // Below the header's two rows (ESC-390), 32 px lower than the preview's: its place across, its size.
        const xwh = (el: Locator) => place(el).then(([x, , w, h]) => [x, w, h]);
        expect.soft(await xwh(band), `band's place, ${where}`).toEqual(await xwh(pband));
        expect.soft(await look(band), `band, ${where}`).toEqual(await look(pband));
        expect.soft(await look(band.locator('.icon')), `its icon, ${where}`).toEqual(await look(pband.locator('svg')));
        expect.soft(await look(band.locator('.btn')), `« Les lire », ${where}`).toEqual(await look(pband.locator('.btn')));

        // The columns: their place, their head (a ring of their tone, name, count, « + »).
        // Their heights aside: the preview's Chromium hides its scrollbars, the app's window shows the board's.
        const xyw = (el: Locator) => place(el).then(([x, , w]) => [x, w]);
        expect.soft(await xyw(board.locator('.cols')), `board's place, ${where}`).toEqual(await xyw(pv.locator('.board')));
        const columns = [cols.todo, cols.doing, cols.review, cols.done];
        for (const [i, col] of columns.entries()) {
          const pcol = pv.locator('.col').nth(i);
          expect.soft(await xyw(col), `column ${i}'s place, ${where}`).toEqual(await xyw(pcol));
          const h = col.locator('.head');
          const ph = pcol.locator('.col-h');
          expect.soft(await lookBut(h, 'width'), `column ${i}'s head, ${where}`).toEqual(await lookBut(ph, 'width'));
          expect.soft(await look(h.locator('.ring')), `its ring, ${where}`).toEqual(await look(ph.locator('.ring')));
          expect.soft(await look(h.locator('.count')), `its count, ${where}`).toEqual(await look(ph.locator('.n')));
        }
        expect.soft(await look(cols.todo.locator('.add')), `« + », ${where}`).toEqual(await look(pv.locator('#add-ticket')));

        // The cards: raised; to test in amber; done flat.
        const card = (col: Locator) => col.locator('.card').first();
        const pcard = (i: number) => pv.locator('.col').nth(i).locator('.card').first();
        for (const [i, col] of columns.entries()) {
          const [cx, , cw] = await place(card(col));
          const [pcx, , pcw] = await place(pcard(i));
          expect.soft([cx, cw], `card ${i}'s place, ${where}`).toEqual([pcx, pcw]);
          expect
            .soft(await lookBut(card(col), 'width', 'height'), `card ${i}, ${where}`)
            .toEqual(await lookBut(pcard(i), 'width', 'height'));
          expect
            .soft(await lookBut(card(col).locator('.top'), 'width', 'height'), `card ${i}'s first line, ${where}`)
            .toEqual(await lookBut(pcard(i).locator('.card-top'), 'width', 'height'));
          expect
            .soft(await lookBut(card(col).locator('.title'), 'width', 'height'), `card ${i}'s title, ${where}`)
            .toEqual(await lookBut(pcard(i).locator('h2'), 'width', 'height'));
        }
        // To do: its criteria, none met; why it waits under a hairline, with its link.
        const crit = card(cols.todo).locator('.crit li').first();
        const pcrit = pcard(0).locator('.crit li').first();
        expect.soft(await lookBut(crit, 'width', 'height'), `a criterion, ${where}`).toEqual(await lookBut(pcrit, 'width', 'height'));
        expect.soft(await look(crit.locator('.icon')), `its circle, ${where}`).toEqual(await look(pcrit.locator('svg')));
        expect
          .soft(await lookBut(card(cols.todo).locator('.act'), 'width', 'height'), `why it waits, ${where}`)
          .toEqual(await lookBut(pcard(0).locator('.card-act'), 'width', 'height'));
        expect
          .soft(await look(card(cols.todo).locator('.act .icon')), `its link, ${where}`)
          .toEqual(await look(pcard(0).locator('.card-act svg')));
        // Under way: the loop and its squares, what the agent does.
        expect
          .soft(await lookBut(card(cols.doing).locator('.loop'), 'width', 'height'), `the loop, ${where}`)
          .toEqual(await lookBut(pcard(1).locator('.loop'), 'width', 'height'));
        expect
          .soft(await look(card(cols.doing).locator('.loops i').first()), `a loop gone through, ${where}`)
          .toEqual(await look(pcard(1).locator('.loops i.on').first()));
        expect
          .soft(await look(card(cols.doing).locator('.loops i').last()), `a loop to come, ${where}`)
          .toEqual(await look(pcard(1).locator('.loops i:not(.on)').last()));
        expect
          .soft(await lookBut(card(cols.doing).locator('.act'), 'width', 'height'), `what its agent does, ${where}`)
          .toEqual(await lookBut(pcard(1).locator('.card-act'), 'width', 'height'));
        expect
          .soft(await look(card(cols.doing).locator('.act .dot')), `its dot, ${where}`)
          .toEqual(await look(pcard(1).locator('.card-act .dot')));
        // To test: its criteria met, « Valider » and « Tester ».
        const met = card(cols.review).locator('.crit li.met').first();
        const pmet = pv.locator('.card.attn .crit li.met').first();
        expect.soft(await lookBut(met, 'width', 'height'), `a criterion met, ${where}`).toEqual(await lookBut(pmet, 'width', 'height'));
        expect.soft(await look(met.locator('.icon')), `its check, ${where}`).toEqual(await look(pmet.locator('svg')));
        expect
          .soft(await lookBut(card(cols.review).locator('.sum'), 'width'), `its loops and cost, ${where}`)
          .toEqual(await lookBut(pv.locator('.card.attn .loop'), 'width'));
        const approve = card(cols.review).getByRole('button', { name: 'Valider et merger' });
        const papprove = pv.locator('.card.attn .btn.primary');
        expect.soft(await look(approve), `« Valider », ${where}`).toEqual(await look(papprove));
        expect.soft(await look(approve.locator('.icon')), `its icon, ${where}`).toEqual(await look(papprove.locator('svg')));
        // Done: what came of it.
        expect
          .soft(await lookBut(card(cols.done).locator('.outcome'), 'width'), `its outcome, ${where}`)
          .toEqual(await lookBut(pv.locator('.card.past .outcome').last(), 'width'));
        expect
          .soft(await look(card(cols.done).locator('.outcome .icon')), `its icon, ${where}`)
          .toEqual(await look(pv.locator('.card.past .outcome svg').last()));

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `kanban${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
  }
});

/** A PNG of one pixel: an image of the ticket. */
const PIXEL = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64');

test.describe('the form of a ticket', () => {
  // Two harnesses found: « Harness » shows, as in the preview.
  test.use({ appEnv: { ESCOUADE_CODEX: FAKE_CODEX } });

  test('looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
    const exe = chromium.executablePath();
    test.skip(
      !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
      'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
    );
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
    await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Recherche plein texte dans les factures');
    await dialog.getByRole('group', { name: 'Harness' }).getByRole('button').click();
    await page.getByRole('menuitemradio', { name: 'Codex', exact: true }).click();
    await dialog.locator('input[type=file]').setInputFiles({ name: 'recherche-factures.png', mimeType: 'image/png', buffer: PIXEL });
    await expect(dialog.getByRole('list', { name: 'Images du ticket' }).locator('img')).toBeVisible();
    const browser = await chromium.launch();
    const preview = await browser.newPage();
    try {
      for (const t of ['dark', 'light'] as const)
        for (const size of SIZES) {
          await page.setViewportSize(size);
          await preview.setViewportSize(size);
          await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#ticket`);
          await preview.evaluate(() => document.fonts.ready);
          await theme(page, t);
          await still(page);
          await still(preview);
          await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
          await page.mouse.move(4, size.height - 4);
          await preview.mouse.move(4, size.height - 4);
          const where = `${t} ${size.width}×${size.height}`;
          const pv = preview.locator('#ticket-modal');
          const fld = dialog.locator('.fld').first();
          expect
            .soft(await lookBut(fld.locator('label'), 'width'), `a label, ${where}`)
            .toEqual(await lookBut(pv.locator('.fld > label'), 'width'));
          expect
            .soft(await lookBut(dialog.locator('.title'), 'width'), `the title, ${where}`)
            .toEqual(await lookBut(pv.locator('#tk-t'), 'width'));
          // A textarea keeps its own line height and rows: its frame and its text are what the preview's has.
          expect
            .soft(await lookBut(dialog.locator('textarea').first(), 'width', 'height', 'padding'), `the description, ${where}`)
            .toEqual(await lookBut(pv.locator('.inp.area').first(), 'width', 'height', 'padding'));
          const chip = dialog.locator('.atts .att.img');
          const pchip = pv.locator('.atts .att').first();
          expect.soft(await lookBut(chip, 'width'), `an image's chip, ${where}`).toEqual(await lookBut(pchip, 'width'));
          expect
            .soft(await lookBut(chip.locator('.th'), 'background'), `its thumbnail, ${where}`)
            .toEqual(await lookBut(pchip.locator('.th'), 'background'));
          expect
            .soft(await lookBut(dialog.locator('.attach'), 'width'), `« Joindre un fichier », ${where}`)
            .toEqual(await lookBut(pv.locator('.atts > .btn'), 'width'));
          // The helps under the fields, in a window tall enough (a short one leaves them out).
          if (size.height >= 820)
            expect
              .soft(await lookBut(dialog.locator('.help').first(), 'width', 'height'), `its help, ${where}`)
              .toEqual(await lookBut(pv.locator('.fld .help').first(), 'width', 'height'));
          const harness = dialog.getByRole('group', { name: 'Harness' });
          const pharness = pv.locator('.fgrid .fld').first();
          expect
            .soft(await lookBut(harness.locator('.lb'), 'width'), `« Harness », ${where}`)
            .toEqual(await lookBut(pharness.locator('.lb'), 'width'));
          expect
            .soft(await lookBut(harness.locator('.select'), 'width'), `its menu, ${where}`)
            .toEqual(await lookBut(pharness.locator('.select'), 'width'));
          expect
            .soft(await look(harness.locator('.select [data-engine]')), `its brand, ${where}`)
            .toEqual(await look(pharness.locator('.emark')));
          expect
            .soft(await lookBut(harness.locator('.select b'), 'width'), `its value, ${where}`)
            .toEqual(await lookBut(pharness.locator('.select b'), 'width'));
          expect
            .soft(await look(harness.locator('.select .icon').last()), `its chevron, ${where}`)
            .toEqual(await look(pharness.locator('.select svg')));
          // The grid of three, as the preview's (the window's frame is lot 11's): a third of the form, gaps aside.
          const [, , fw] = await place(dialog.locator('.fgrid'));
          expect.soft((await place(harness.locator('.select')))[2], `its width, ${where}`).toBe(Math.round((fw - 24) / 3));
          // Nothing to scroll in the window: the description takes what is left.
          expect
            .soft(await dialog.locator('.body').evaluate((b) => b.scrollHeight - b.clientHeight), `window's scroll, ${where}`)
            .toBeLessThanOrEqual(1);
          if (SHOTS) {
            fs.mkdirSync(SHOTS, { recursive: true });
            const n = `ticket${size.name}${t === 'light' ? '-light' : ''}`;
            await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
            await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
          }
        }
    } finally {
      await browser.close();
    }
  });
});

test('nothing of the board moves with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await newTicket(page, 'Limiter les tentatives de connexion [lent]');
  const card = page.getByRole('region', { name: 'En cours' }).locator('.card');
  await expect(card).toHaveCount(1, { timeout: 60_000 });
  const knob = page.locator('main.board .knob');
  const durations = () =>
    Promise.all([
      card.evaluate((e) => getComputedStyle(e).transitionDuration),
      knob.evaluate((e) => getComputedStyle(e, '::after').transitionDuration),
    ]);
  expect(await durations()).toEqual(['0.15s', '0.25s']);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  expect((await durations()).map((d) => parseFloat(d) < 0.001)).toEqual([true, true]);
  // A transition already under way when the preference changes ends at its own pace (CSS Transitions): the knob's colour
  // was, in the runner's small window. Its end waited for; one that starts now lasts 0.01 ms, and nothing may run after.
  await page.locator('main.board').evaluate((b) =>
    Promise.all(
      b
        .getAnimations({ subtree: true })
        .filter((a) => a.effect?.getTiming().iterations !== Infinity)
        .map((a) => a.finished),
    ),
  );
  // Nothing runs on the board: no animation, endless or not.
  expect(
    await page.locator('main.board').evaluate((b) => b.getAnimations({ subtree: true }).filter((a) => a.playState === 'running').length),
  ).toBe(0);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

test.describe('in English', () => {
  test.use({ language: 'en', appEnv: { ESCOUADE_CODEX: FAKE_CODEX } });

  /** What is cut short, or overflows its box, in `root`: none of it, but what is cut by design (with its tooltip). */
  const cut = (root: Locator, byDesign: string) =>
    root.evaluate(
      (r, byDesign) =>
        [...r.querySelectorAll<HTMLElement>('*')]
          .filter(
            (el) =>
              el.offsetParent &&
              el.children.length === 0 &&
              !(el instanceof HTMLTextAreaElement) &&
              !el.closest(byDesign) &&
              !el.classList.contains('sr') &&
              (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
          )
          .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
      byDesign,
    );

  test('the labels of the board and of the ticket’s form fit in their place, at both sizes', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false, lang: 'en' });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    const add = async (title: string) => {
      await page.getByRole('button', { name: 'New ticket' }).click();
      const dialog = page.getByRole('dialog', { name: 'New ticket' });
      await dialog.getByRole('textbox', { name: 'Ticket title' }).fill(title);
      await dialog.getByRole('button', { name: 'Add', exact: true }).click();
      await expect(dialog).toBeHidden();
    };
    await add('Password reset [ok]');
    const review = page.getByRole('region', { name: 'To review' });
    await expect(review.locator('.card')).toHaveCount(1, { timeout: 60_000 });
    await add('Rate limiting [lent]');
    await expect(page.getByRole('region', { name: 'In progress' }).locator('.card')).toHaveCount(1, { timeout: 60_000 });
    const board = page.locator('main.board');
    for (const size of SIZES) {
      await page.setViewportSize(size);
      // The board's own texts: its titles, its criteria, what an agent does, cut by design (whole in their tooltips).
      expect(await cut(board, '.title, .crit, .grow, .sub, .v, .why, .over'), `board, ${size.width}×${size.height}`).toEqual([]);
      // Its buttons hold on one line.
      for (const b of await board.locator('.card .btn').all())
        expect((await b.boundingBox())!.height, `${await b.textContent()}, ${size.width}×${size.height}`).toBeLessThanOrEqual(29);
      await expect(review.getByRole('button', { name: 'Approve and merge' })).toHaveText('Approve');
      // A card under way with its harness and its attachments beside its key and its loop (the room of two more items
      // on its top line, as wide as « Claude » and « 📎 1 »): nothing leaves the card, the key stays on one line (ESC-84).
      const top = await page
        .getByRole('region', { name: 'In progress' })
        .locator('.card .top')
        .first()
        .evaluate((el) => {
          const key = el.querySelector<HTMLElement>('.key')!;
          const extra = ['Claude', 'C 1'].map((text) => {
            const c = key.cloneNode() as HTMLElement;
            c.classList.add('esc84');
            c.textContent = text;
            key.after(c);
            return c;
          });
          const card = el.closest('.card')!.getBoundingClientRect();
          const out = [...el.children]
            .filter((c) => c.getBoundingClientRect().right > card.right - 11 || c.getBoundingClientRect().left < card.left)
            .map((c) => c.textContent);
          const keyLines = key.getBoundingClientRect().height;
          for (const c of extra) c.remove();
          return { out, oneLine: keyLines < 20 };
        });
      expect(top, `the top line of a full card, ${size.width}×${size.height}`).toEqual({ out: [], oneLine: true });
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `kanban${size.name}-en.app.png`) });
      }
    }
    await page.getByRole('button', { name: 'New ticket' }).click();
    const dialog = page.getByRole('dialog', { name: 'New ticket' });
    // Measured once it has risen (the windows' rise scales them a little, ESC-83).
    await dialog.evaluate((e) => Promise.all(e.getAnimations().map((a) => a.finished)));
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await expect(dialog.getByRole('group', { name: 'Harness' })).toBeVisible();
      // The values of the menus are cut by design, whole in their tooltips.
      expect(await cut(dialog, '.select b, .nm2'), `form, ${size.width}×${size.height}`).toEqual([]);
      for (const name of ['Harness', 'Account', 'Model', 'Effort', 'Branch'])
        expect((await dialog.getByRole('group', { name }).locator('.select').boundingBox())!.height, name).toBe(32);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `ticket${size.name}-en.app.png`) });
    }
  });
});
