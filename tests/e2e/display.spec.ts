// « Apparence » in Réglages › Général: the zoom of the whole interface (the WebView's), applied at once, kept after a
// restart, changed from the keyboard too, never past what the window holds (the interface is laid out for 1000 × 600
// pixels at least); and the font sizes of the terminals and of the editor (in the page's « Avancé »), set apart.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, SHOW, test, openSettingsPage, settingsEntry } from './fixture';

const STEPS = [80, 90, 100, 110, 125, 150, 175, 200];
/** The steps the settings offer in segments (the keys reach the others). */
const SEGMENTS = [90, 100, 110, 125, 150];
const BRANCH = 'ticket/esc-112-une-branche-au-nom-bien-trop-long-pour-tenir-dans-la-barre-d-etat';
const percent = (n: number) => `${n} %`;

/** The zoom the page is shown at, from its pixel ratio and the one it had at 100 %. */
const zoomOf = async (page: Page, base: number) => Math.round(((await page.evaluate(() => devicePixelRatio)) / base) * 100);

/**
 * The elements shown past the right or the bottom edge of the window: what a zoom must never cut. One below the fold of
 * a pane that scrolls, the pane itself in the window, is reached by scrolling: not cut.
 */
function overflowing(page: Page) {
  return page.evaluate(() => {
    const out: string[] = [];
    /** Whether `el` lies in a box that scrolls on `axis` (the board scrolls sideways under four columns), itself in view. */
    const scrolledTo = (el: Element, axis: 'x' | 'y') => {
      for (let p = el.parentElement; p; p = p.parentElement) {
        const s = getComputedStyle(p);
        const o = axis === 'y' ? s.overflowY : s.overflowX;
        const scrolls = axis === 'y' ? p.scrollHeight > p.clientHeight : p.scrollWidth > p.clientWidth;
        if ((o === 'auto' || o === 'scroll') && scrolls) {
          const b = p.getBoundingClientRect();
          return b.bottom <= innerHeight + 1 && b.right <= innerWidth + 1;
        }
      }
      return false;
    };
    for (const el of document.querySelectorAll('body *')) {
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height || getComputedStyle(el).visibility === 'hidden') continue;
      // The terminals' parking lot is off screen on purpose.
      if (r.left < -1000) continue;
      if ((r.right <= innerWidth + 1 || scrolledTo(el, 'x')) && (r.bottom <= innerHeight + 1 || scrolledTo(el, 'y'))) continue;
      if (r.right > innerWidth + 1 || r.bottom > innerHeight + 1) {
        out.push(`${el.tagName.toLowerCase()}.${[...el.classList].join('.')} (${Math.round(r.right)}, ${Math.round(r.bottom)})`);
      }
    }
    return out.slice(0, 10);
  });
}

/** The settings' page « Général », open. */
async function openDisplay(page: Page) {
  await page.keyboard.press('Control+,');
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  return dialog;
}

const settingsOf = (data: string) => JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8'));

test('the zoom of Réglages › Général grows the whole interface at once, nothing cut, and stays after a restart', async ({ app }) => {
  let { page } = app;
  await addProject(page, app.repo);
  const base = await page.evaluate(() => devicePixelRatio);
  // The biggest step this window holds, as its size at 100 % says.
  const room = await page.evaluate(() => ({ width: innerWidth, height: innerHeight }));
  const fit = STEPS.filter((z) => z <= 100 || ((room.width * 100) / z >= 999.5 && (room.height * 100) / z >= 599.5)).at(-1)!;
  // The step set: the biggest the window holds, or 90 % in a window too small for more than 100 % (a 1024 × 768 screen,
  // as CI's).
  const set = fit > 100 ? fit : 90;

  let dialog = await openDisplay(page);
  const zoom = dialog.getByRole('group', { name: 'Zoom de l’interface' });
  await expect(zoom.getByRole('button', { name: percent(100) })).toHaveAttribute('aria-pressed', 'true');
  for (const z of SEGMENTS) {
    if (z > fit) await expect(zoom.getByRole('button', { name: percent(z) })).toBeDisabled();
    else await expect(zoom.getByRole('button', { name: percent(z) })).toBeEnabled();
  }
  // At once, the dialog grows (or shrinks) with the rest, nothing to save.
  await zoom.getByRole('button', { name: percent(set) }).click();
  await expect.poll(() => zoomOf(page, base)).toBe(set);
  await expect(zoom.getByRole('button', { name: percent(set) })).toHaveAttribute('aria-pressed', 'true');
  await expect(settingsEntry(dialog, 'Général')).not.toHaveClass(/changed/);
  await expect.poll(() => settingsOf(app.data).uiZoom).toBe(set);
  expect(await overflowing(page)).toEqual([]);
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();

  // The interface is all there and works: the sidebar, the project's tab, the conversation, the board.
  expect(await overflowing(page)).toEqual([]);
  await expect(page.getByRole('button', { name: /Nouvel agent/ })).toBeInViewport();
  await expect(page.getByRole('button', { name: 'Envoyer' })).toBeInViewport();
  const box = page.getByRole('textbox').last();
  await box.fill('Bonjour');
  await expect(box).toHaveValue('Bonjour');
  await page.getByRole('button', { name: 'Kanban' }).first().click();
  await expect(page.getByText('À faire', { exact: true }).first()).toBeInViewport();
  expect(await overflowing(page)).toEqual([]);

  // Kept after a restart, shown at once.
  page = await app.restart();
  await expect(page.getByRole('button', { name: /Nouvel agent|Ajouter un projet/ }).first()).toBeVisible();
  await expect.poll(() => zoomOf(page, base)).toBe(set);
  expect(await overflowing(page)).toEqual([]);

  // From the keyboard: Ctrl+= a step up but no further than the window holds, Ctrl+- a step down, Ctrl+0 back to 100 %.
  await page.locator('body').click({ position: { x: 5, y: 300 } });
  if (set < fit) {
    await page.keyboard.press('Control+=');
    await expect.poll(() => zoomOf(page, base)).toBe(STEPS[STEPS.indexOf(set) + 1]);
  }
  await page.keyboard.press('Control+=');
  await page.waitForTimeout(300);
  expect(await zoomOf(page, base)).toBe(fit);
  await page.keyboard.press('Control+-');
  await expect.poll(() => zoomOf(page, base)).toBe(STEPS[STEPS.indexOf(fit) - 1]);
  await page.keyboard.press('Control+0');
  await expect.poll(() => zoomOf(page, base)).toBe(100);
  await page.keyboard.press('Control+-');
  await page.keyboard.press('Control+-');
  await expect.poll(() => zoomOf(page, base)).toBe(80);
  expect(await overflowing(page)).toEqual([]);
  await expect.poll(() => settingsOf(app.data).uiZoom).toBe(80);
  dialog = await openDisplay(page);
  await expect(dialog.getByRole('group', { name: 'Zoom de l’interface' }).getByRole('button', { name: percent(80) })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
});

test('the terminals and the editor have their own font size, set at once and fitted to their place', async ({ app }) => {
  const { page } = app;
  // The sizes the terminals' pseudo-consoles are told: the one they start at, then each new one (a terminal started at the size
  // of its place is told none until it changes). Seen as they go (a route has missed some).
  const resizes: { cols: number; rows: number }[] = [];
  page.on('request', (r) => {
    if (!/ipc[.]localhost[/]term_(spawn|resize)/.test(r.url())) return;
    const body = JSON.parse(r.postData() ?? '{}');
    resizes.push({ cols: body.cols, rows: body.rows });
  });
  await addProject(page, app.repo);
  // A long branch name, which the status bar has to make room for.
  execFileSync('git', ['checkout', '-q', '-b', BRANCH], { cwd: app.repo, stdio: 'pipe' });
  await page.keyboard.press('Control+t');
  const term = page.locator('main .xterm').first();
  await expect(term).toBeVisible();
  const measure = () =>
    page.evaluate(() => {
      const xterm = document.querySelector('main .xterm') as HTMLElement;
      const screen = xterm.querySelector('.xterm-screen') as HTMLElement;
      const box = xterm.parentElement!.parentElement!;
      // xterm.js gives its textarea the size of a cell.
      const cell = xterm.querySelector('.xterm-helper-textarea') as HTMLElement;
      return {
        cell: cell.getBoundingClientRect().width,
        screen: screen.getBoundingClientRect().width,
        room: box.getBoundingClientRect().width,
      };
    });
  /** Sets the terminals' font size in the settings, then types in the terminal: xterm.js sizes its textarea as it moves the cursor. */
  const setSize = async (typed: string, kept: string) => {
    const dialog = await openDisplay(page);
    const field = dialog.getByRole('spinbutton', { name: 'Taille du texte des terminaux' });
    await field.fill(typed);
    await field.press('Tab');
    await expect(field).toHaveValue(kept);
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await term.click();
    await page.keyboard.type('a');
  };
  await term.click();
  await page.keyboard.type('a');
  await expect.poll(async () => (await measure()).cell).toBeGreaterThan(0);
  await expect.poll(() => resizes.length).toBeGreaterThan(0);
  const before = { ...(await measure()), cols: resizes.at(-1)!.cols };

  await setSize('20', '20');
  // Cells as wide as the text: 20 / 12.5 times what they were.
  await expect.poll(async () => (await measure()).cell / before.cell).toBeGreaterThan(1.45);
  expect((await measure()).cell / before.cell).toBeLessThan(1.8);
  // Fitted again to its box, the new size told to its program: fewer columns, as many as the box holds.
  await expect.poll(() => resizes.at(-1)!.cols).toBeLessThan(before.cols * 0.7);
  const after = await measure();
  expect(after.screen).toBeLessThanOrEqual(after.room + 1);
  expect(after.screen).toBeGreaterThan(after.room * 0.8);
  expect(Math.abs(resizes.at(-1)!.cols - Math.floor(after.screen / after.cell))).toBeLessThanOrEqual(1);
  // Out of range: held to the biggest.
  await setSize('40', '24');
  await expect.poll(async () => (await measure()).cell / before.cell).toBeGreaterThan(1.75);

  const dialog = await openDisplay(page);
  // The editor's, in « Avancé » at the foot of the page, folded.
  await dialog
    .locator('#settings-panel')
    .getByRole('button', { name: /^Avancé/ })
    .click();
  const editor = dialog.getByRole('spinbutton', { name: 'Taille du texte de l’éditeur' });
  await editor.fill('16');
  await editor.press('Tab');
  await expect.poll(() => settingsOf(app.data)).toMatchObject({ terminalFontSize: 24, editorFontSize: 16 });
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();

  await page.getByRole('button', { name: 'Parcourir' }).click();
  await page.getByRole('treeitem', { name: /src/ }).click();
  await page.getByRole('treeitem', { name: /app\.ts/ }).click();
  await expect(page.locator('.cm-content')).toContainText('const a = 1;');
  const line = await page
    .locator('.cm-line')
    .first()
    .evaluate((el) => {
      const style = getComputedStyle(el);
      return { font: style.fontSize, height: el.getBoundingClientRect().height };
    });
  expect(line.font).toBe('16px');
  // The lines grow with the text: 1.75 times its size, as the 2.0 design draws them (ESC-81).
  expect(Math.round(line.height)).toBe(Math.round(16 * 1.75));
  // The gutter's line numbers sit on their lines.
  const gutter = await page
    .locator('.cm-lineNumbers .cm-gutterElement')
    .nth(1)
    .evaluate((el) => el.getBoundingClientRect().height);
  expect(Math.round(gutter)).toBe(Math.round(line.height));
  expect(await overflowing(page)).toEqual([]);

  // In the smallest window the interface is laid out for, nothing is cut either: the status bar keeps its buttons whole,
  // what it tells gives way, the branch's name cut short; the settings are on the projects' rail (ESC-158).
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 1000, height: 600, deviceScaleFactor: 0, mobile: false });
  await expect.poll(() => page.evaluate(() => [innerWidth, innerHeight])).toEqual([1000, 600]);
  expect(await overflowing(page)).toEqual([]);
  const bar = page.locator('footer.bar');
  await expect(bar.getByRole('button', { name: 'Son des notifications' })).toBeInViewport({ ratio: 1 });
  await expect(page.getByRole('button', { name: 'Réglages (Ctrl+,)' })).toBeInViewport({ ratio: 1 });
  await expect(bar.getByRole('button', { name: new RegExp(BRANCH) })).toBeInViewport({ ratio: 1 });
  const branch = await bar
    .locator('.branch .name')
    .evaluate((el) => ({ shown: el.clientWidth, whole: el.scrollWidth, text: el.textContent }));
  expect(branch.text).toContain(BRANCH);
  expect(branch.shown).toBeLessThan(branch.whole);
  await cdp.send('Emulation.clearDeviceMetricsOverride');
});

test('the window kept out of sight draws, and the page has the focus as in front, at its usual size', async ({ app }) => {
  const { page } = app;
  // The tests run with the window off every screen and never focused (fixture.ts), unless ESCOUADE_E2E_SHOW asks to see it.
  const state = await page.evaluate(
    () =>
      new Promise<Record<string, unknown>>((resolve) => {
        const now = (frame: boolean) => ({
          frame,
          focus: document.hasFocus(),
          visibility: document.visibilityState,
          width: innerWidth,
          height: innerHeight,
          // Where the window is, right of the left edge of the screen it is nearest to.
          offScreen: screenX >= (screen as Screen & { availLeft: number }).availLeft + screen.width,
        });
        const t = setTimeout(() => resolve(now(false)), 2000);
        requestAnimationFrame(() => (clearTimeout(t), resolve(now(true))));
      }),
  );
  expect(state).toMatchObject({ frame: true, focus: true, visibility: 'visible', offScreen: !SHOW });
  // The size of tauri.conf.json, as big as the screen allows: the interface's minimum at least.
  expect(state.width as number).toBeGreaterThanOrEqual(1000);
  expect(state.height as number).toBeGreaterThanOrEqual(600);
  // A field focused in the page has the focus, as in a window in front.
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .focus();
  await expect(page.getByRole('button', { name: /Ajouter un projet/ }).first()).toBeFocused();
});
