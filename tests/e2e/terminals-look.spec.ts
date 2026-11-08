// Terminals and launches in the 2.0 design (ESC-80, lot 8): the log of a launch command (name, state, « depuis 14:02 »,
// command, folder and shell, Effacer, Relancer, Stopper), a terminal, the notification of a command that crashed with
// « Voir le log », the window of « Tester » (who wrote the commands, then each step; then the launch, step by step), the
// window « « Front » tourne déjà », and the bands above a harness's terminal (Copilot CLI's question about the trust of
// the folder, OpenCode's « Escouade garde le fil »). Compared to the preview (design/app-redesign/index.html, `#run`,
// `#terminal`, `#test`, `#test-run`, `#run-conflict`, `#copilot-view`, `#opencode-view`) in both themes at 1440 × 900
// and at CI's 1028 × 779; xterm's theme follows the theme at once; the English labels in their room; the motion stopped
// by prefers-reduced-motion. The captures of the app and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Browser, type Locator, type Page } from '@playwright/test';
import { addProject, expect, FAKE_COPILOT, FAKE_OPENCODE, test, unfoldSections, closeSettings } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
/** The preview loads Inter from the site's packages: without them, its text is not the app's. */
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];
const THEMES = ['dark', 'light'] as const;
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

/** The preview at `#route`, in theme `t`: from a blank page, for its script to open the route. */
async function previewAt(preview: Page, route: string, t: 'dark' | 'light', lang = '') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}${lang}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

/** The app and the preview, at `size` in theme `t`, still and the pointer out of the way. */
async function both(page: Page, preview: Page, size: (typeof SIZES)[number], t: 'dark' | 'light', route: string) {
  await page.setViewportSize(size);
  await preview.setViewportSize(size);
  await theme(page, t);
  await still(page);
  await page.evaluate(() => document.fonts.ready);
  await page.mouse.move(size.width / 2, 12);
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await previewAt(preview, route, t);
}

/** Saves the captures of the app and of the preview, named after the preview's (`<écran>[-small][-light]`). */
async function shots(page: Page, preview: Page, screen: string, size: (typeof SIZES)[number], t: 'dark' | 'light') {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const n = `${screen}${size.name}${t === 'light' ? '-light' : ''}`;
  await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
  await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
}

/** The launch commands of the project, as Réglages › Projets › Lancement writes them. */
async function commands(page: Page, list: { name: string; command: string; dir?: string }[]) {
  // The header's « Commandes de lancement… »: in a short window (the runner's), the section folds and « Configurer » with it.
  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  for (const [i, c] of list.entries()) {
    await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
    const cfg = dialog.getByRole('group', { name: `Commande ${i + 1}` });
    await cfg.getByLabel('Nom').fill(c.name);
    await cfg.getByLabel('Commande').fill(c.command);
    if (c.dir) await cfg.getByLabel(/Sous-dossier/).fill(c.dir);
  }
  await closeSettings(dialog);
  await unfoldSections(page);
}

/** A colour as `page` paints it, in 8-bit sRGB (an oklch() of the preview as an rgb() of xterm.js). */
const rgbIn = (page: Page, css: string) =>
  page.evaluate((css) => {
    const c = document.createElement('canvas');
    c.width = c.height = 1;
    const ctx = c.getContext('2d', { willReadFrequently: true })!;
    // A token (var(--well)) is resolved by the page first.
    const probe = document.createElement('div');
    probe.style.color = css;
    document.body.append(probe);
    ctx.fillStyle = getComputedStyle(probe).color;
    probe.remove();
    ctx.fillRect(0, 0, 1, 1);
    return [...ctx.getImageData(0, 0, 1, 1).data.slice(0, 3)];
  }, css);

/** The colour xterm.js paints its terminal with (set on its viewport from its theme), in 8-bit sRGB. */
const xtermBackground = async (el: Locator) =>
  rgbIn(el.page(), await el.locator('.xterm-viewport').evaluate((v) => (v as HTMLElement).style.backgroundColor));

/** The background of `el`, in 8-bit sRGB. */
const backgroundOf = async (el: Locator) => rgbIn(el.page(), (await look(el)).background);

async function withPreview(run: (preview: Page, browser: Browser) => Promise<void>) {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    await run(preview, browser);
  } finally {
    await browser.close();
  }
}

/** Where an element is, its top taken from the bottom of `head` (a header whose height may differ from the preview's). */
const under = async (el: Locator, head: Locator) => {
  const [x, y, w, h] = await place(el);
  const [, hy, , hh] = await place(head);
  return [x, y - (hy + hh), w, h];
};

/** The header of a log or a terminal, beside the preview's: its place, its title, its crumbs, its buttons. */
async function compareHead(head: Locator, phead: Locator, where: string) {
  expect.soft(await place(head), `header's place, ${where}`).toEqual(await place(phead));
  expect.soft(await lookBut(head, 'width'), `header, ${where}`).toEqual(await lookBut(phead, 'width'));
  expect.soft(await lookBut(head.locator('h1'), 'width'), `its title, ${where}`).toEqual(await lookBut(phead.locator('h1'), 'width'));
  const [hx, hy] = await place(head.locator('h1'));
  const [phx, phy] = await place(phead.locator('h1'));
  expect.soft([hx, hy], `its title's place, ${where}`).toEqual([phx, phy]);
  expect
    .soft(await lookBut(head.locator('.crumbs'), 'width'), `its crumbs, ${where}`)
    .toEqual(await lookBut(phead.locator('.crumbs'), 'width'));
  const [cx, cy] = await place(head.locator('.crumbs'));
  const [pcx, pcy] = await place(phead.locator('.crumbs'));
  expect.soft([cx, cy], `its crumbs' place, ${where}`).toEqual([pcx, pcy]);
  const btns = head.locator('.actions .btn');
  const pbtns = phead.locator('.actions .btn');
  const n = await pbtns.count();
  expect.soft(await btns.count(), `its buttons, ${where}`).toBe(n);
  for (let i = 0; i < n; i++) {
    const [b, pb] = [btns.nth(i), pbtns.nth(i)];
    expect.soft(await lookBut(b, 'width'), `button ${i + 1}, ${where}`).toEqual(await lookBut(pb, 'width'));
    expect.soft(await b.locator('svg').count(), `button ${i + 1}'s icon, ${where}`).toBe(await pb.locator('svg').count());
  }
  const [lx, , lw] = await place(btns.last());
  const [plx, , plw] = await place(pbtns.last());
  expect.soft(lx + lw, `the last button's right edge, ${where}`).toBe(plx + plw);
}

/** The terminal under the header, beside the preview's well: its place, its colour, where its text starts. */
async function compareTerm(box: Locator, pterm: Locator, where: string) {
  const [x, y, w] = await place(box);
  const [px, py, pw] = await place(pterm);
  expect.soft([x, y, w], `the terminal's place, ${where}`).toEqual([px, py, pw]);
  expect.soft((await look(box)).background, `its well, ${where}`).toBe((await look(pterm)).background);
  // xterm.js paints it in the same colour, and starts its text where the preview's does.
  const well = await backgroundOf(pterm);
  await expect.poll(() => xtermBackground(box), { message: `xterm's background, ${where}` }).toEqual(well);
  const [sx, sy] = await place(box.locator('.xterm-screen'));
  const pad = await pterm.evaluate((e) => [parseFloat(getComputedStyle(e).paddingLeft), parseFloat(getComputedStyle(e).paddingTop)]);
  expect.soft([sx, sy], `where its text starts, ${where}`).toEqual([px + pad[0], py + pad[1]]);
}

test('the log of a launch command, a crash and a terminal look as the preview draws them, in both themes, at both sizes', async ({
  app,
}) => {
  test.setTimeout(300_000);
  const { page } = app;
  fs.mkdirSync(path.join(app.repo, 'web'));
  await addProject(page, app.repo);
  await commands(page, [
    { name: 'Front', command: "Write-Output '  VITE v6.4.3  ready in 412 ms'; Start-Sleep 600", dir: 'web' },
    { name: 'Worker', command: 'exit 1' },
  ]);
  await page.getByRole('button', { name: 'Tout lancer', exact: true }).click();
  const front = page.locator('.run').filter({ has: page.locator('.name', { hasText: /^Front$/ }) });
  await expect(front).toContainText('en cours');
  // The crash is told with « Voir le log », which opens the log of the command.
  const toast = page.locator('.toasts .toast.error', { hasText: "« Worker » s'est arrêté en erreur (code 1)" });
  await expect(toast).toBeVisible({ timeout: 30_000 });
  await withPreview(async (preview) => {
    await theme(page, 'dark');
    await previewAt(preview, 'run', 'dark');
    expect
      .soft(await lookBut(toast.locator('.act'), 'width'), 'the button of the notification')
      .toEqual(await lookBut(preview.locator('#toast .link'), 'width'));
  });
  await toast.getByRole('button', { name: 'Voir le log' }).click();
  const view = page.locator('main.rv');
  await expect(view.locator('h1')).toHaveText('Worker');
  await expect(view.locator('.state.err')).toHaveText('planté (code 1)');
  await expect(view.getByRole('button', { name: 'Relancer' }).locator('[data-icon="restart"]')).toBeVisible();

  await front.click();
  await expect(view.locator('h1')).toHaveText('Front');
  await expect(view.locator('.since')).toHaveText(/^depuis \d\d:\d\d$/);
  await expect(view.locator('.xterm')).toBeVisible();
  await withPreview(async (preview) => {
    for (const t of THEMES)
      for (const size of SIZES) {
        await both(page, preview, size, t, 'run');
        const where = `#run ${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#view-run');
        await compareHead(view.locator('> .head'), pv.locator('.head'), where);
        expect
          .soft(await lookBut(view.locator('.state'), 'width'), `its state, ${where}`)
          .toEqual(await lookBut(pv.locator('.pill'), 'width'));
        expect
          .soft(await lookBut(view.locator('.since'), 'width'), `since when, ${where}`)
          .toEqual(await lookBut(pv.locator('.since'), 'width'));
        expect
          .soft(await lookBut(view.locator('.crumbs .cmdline'), 'width'), `its command, ${where}`)
          .toEqual(await lookBut(pv.locator('.crumbs .mono').first(), 'width'));
        await compareTerm(view.locator('.box'), pv.locator('pre.term'), where);
        await shots(page, preview, 'run', size, t);
      }
  });

  // A terminal, with its shell (`#terminal`).
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  const tv = page.locator('main.tv');
  await expect(tv.locator('.xterm')).toBeVisible();
  await withPreview(async (preview) => {
    for (const t of THEMES)
      for (const size of SIZES) {
        await both(page, preview, size, t, 'terminal');
        const where = `#terminal ${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#view-term');
        await compareHead(tv.locator('> .head'), pv.locator('.head'), where);
        expect
          .soft(await lookBut(tv.locator('.chip'), 'width'), `its shell, ${where}`)
          .toEqual(await lookBut(pv.locator('.chip-sm'), 'width'));
        await compareTerm(tv.locator('.box'), pv.locator('pre.term'), where);
        await shots(page, preview, 'terminal', size, t);
      }
  });
});

test('xterm takes the theme at once, the terminal kept open, with colours read in the light theme', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  const box = page.locator('main.tv .box');
  await expect(box.locator('.xterm')).toBeVisible();
  for (const t of ['light', 'dark', 'light'] as const) {
    await theme(page, t);
    const well = await rgbIn(page, 'var(--well)');
    await expect.poll(() => xtermBackground(box), { message: t }).toEqual(well);
  }
  // The same terminal throughout: nothing opened again.
  await expect(page.locator('main.tv .xterm')).toHaveCount(1);
  await theme(page, 'dark');
});

test.describe('the window of « Tester »', () => {
  test('reads as the preview, the recipe and then its steps, in both themes, at both sizes', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    fs.writeFileSync(path.join(app.repo, 'serveur.js'), 'setInterval(() => {}, 1000);\n');
    execFileSync('git', ['add', '-A'], { cwd: app.repo });
    execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Page [ok] [recette]');
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    const review = page.getByRole('region', { name: 'À tester' });
    await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
    await review.getByRole('button', { name: 'Tester' }).click();
    const dialog = page.getByRole('dialog', { name: 'Tester DEM-1' });
    await expect(dialog.locator('.notice')).toContainText('Ces commandes ont été écrites par');
    await withPreview(async (preview) => {
      for (const t of THEMES)
        for (const size of SIZES) {
          await both(page, preview, size, t, 'test');
          const where = `#test ${t} ${size.width}×${size.height}`;
          const pv = preview.locator('#test-modal');
          expect
            .soft(await lookBut(dialog.locator('.phase'), 'width'), `« À relire », ${where}`)
            .toEqual(await lookBut(pv.locator('#test-state'), 'width'));
          // The window's frame is lot 11's: what is inside it is this lot's.
          const notice = dialog.locator('.notice');
          const pnotice = pv.locator('.notice');
          expect
            .soft(await lookBut(notice, 'width', 'height'), `who wrote them, ${where}`)
            .toEqual(await lookBut(pnotice, 'width', 'height'));
          expect
            .soft(await lookBut(notice.locator('b'), 'width'), `the agent, in bold, ${where}`)
            .toEqual(await lookBut(pnotice.locator('b'), 'width'));
          expect.soft(await look(notice.locator('svg')), `its shield, ${where}`).toEqual(await look(pnotice.locator('svg')));
          expect
            .soft(await lookBut(dialog.locator('h3').first(), 'width'), `a section's title, ${where}`)
            .toEqual(await lookBut(pv.locator('.m-sec h3').first(), 'width'));
          expect
            .soft(await lookBut(dialog.locator('ul.steps'), 'width', 'height'), `the processes, ${where}`)
            .toEqual(await lookBut(pv.locator('ul.steps'), 'width', 'height'));
          const step = dialog.locator('ul.steps li').first();
          const pstep = pv.locator('ul.steps li').first();
          expect.soft(await lookBut(step, 'width', 'height'), `a process, ${where}`).toEqual(await lookBut(pstep, 'width', 'height'));
          expect
            .soft(await lookBut(step.locator('.name'), 'width'), `its name, ${where}`)
            .toEqual(await lookBut(pstep.locator('.sname'), 'width'));
          expect
            .soft(await lookBut(step.locator('.cmd'), 'width'), `its command, ${where}`)
            .toEqual(await lookBut(pstep.locator('.cmd'), 'width'));
          expect
            .soft(await lookBut(step.locator('dt').first(), 'width'), `a fact, ${where}`)
            .toEqual(await lookBut(pstep.locator('dt').first(), 'width'));
          expect
            .soft(await lookBut(step.locator('dd').first(), 'width'), `its value, ${where}`)
            .toEqual(await lookBut(pstep.locator('dd').first(), 'width'));
          const launch = dialog.getByRole('button', { name: 'Lancer' });
          expect.soft(await launch.locator('[data-icon="play"]').count(), `« Lancer »'s icon, ${where}`).toBe(1);
          await shots(page, preview, 'test', size, t);
        }
    });

    // Launched: the steps, each with its time; then where the browser went.
    await dialog.getByRole('button', { name: 'Lancer' }).click();
    await expect(dialog.getByText(/en attente de localhost:\d+…/)).toBeVisible({ timeout: 30_000 });
    await expect(dialog.locator('.phase')).toHaveText('En cours');
    await expect(dialog.locator('.lines li .spin')).toHaveCount(1);
    await withPreview(async (preview) => {
      for (const t of THEMES)
        for (const size of SIZES) {
          await both(page, preview, size, t, 'test-run');
          const where = `#test-run ${t} ${size.width}×${size.height}`;
          const pv = preview.locator('#test-modal');
          const line = dialog.locator('.lines li').first();
          const pline = pv.locator('.flow li').first();
          expect.soft(await lookBut(line, 'width'), `a step, ${where}`).toEqual(await lookBut(pline, 'width'));
          expect
            .soft(await lookBut(line.locator('.label'), 'width'), `its name, ${where}`)
            .toEqual(await lookBut(pline.locator('.lbl'), 'width'));
          expect
            .soft(await lookBut(line.locator('.detail'), 'width', 'color', 'border'), `its state, ${where}`)
            .toEqual(await lookBut(pline.locator('.det'), 'width', 'color', 'border'));
          // Its mark in the first column, its name in the second, as the preview's.
          const [mx] = await place(line.locator('.mark'));
          const [lx] = await place(line.locator('.label'));
          const [pmx] = await place(pline.locator('.mk'));
          const [plx] = await place(pline.locator('.lbl'));
          expect.soft(lx - mx, `its name after its mark, ${where}`).toBe(plx - pmx);
          await shots(page, preview, 'test-run', size, t);
        }
    });
    await dialog.getByRole('button', { name: 'Tout arrêter' }).click();
  });
});

test('a launch command that runs in a worktree already asks first, « Arrêter l’autre et lancer » foremost', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true });
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const agentName = (await page.locator('.card .name').first().textContent())!.trim();
  await commands(page, [{ name: 'Front', command: 'Start-Sleep 600' }]);
  const front = page.locator('.run').filter({ has: page.locator('.name', { hasText: /^Front$/ }) });
  await front.hover();
  await front.getByRole('button', { name: 'Lancer' }).click();
  await page.getByRole('menuitem', { name: `Dans le worktree de ${agentName}` }).click();
  // Its log under its own name, in the worktree of its agent (choice 30).
  const inWt = page.locator('.run', { hasText: 'dans son worktree' });
  await expect(inWt).toContainText('en cours');
  await inWt.click();
  const view = page.locator('main.rv');
  await expect(view.locator('h1')).toHaveText('Front');
  await expect(view.locator('.crumbs .owner')).toHaveText(`dans le worktree de ${agentName}`);

  await front.hover();
  await front.getByRole('button', { name: 'Lancer' }).click();
  await page.getByRole('menuitem', { name: 'Dans le projet' }).click();
  const confirm = page.getByRole('dialog', { name: '« Front » tourne déjà' });
  await expect(confirm.locator('.head [data-icon="play"]')).toBeVisible();
  await expect(confirm.locator('.foot button')).toHaveText(['Annuler', 'Lancer quand même', "Arrêter l'autre et lancer"]);
  await expect(confirm.locator('.foot .btn.primary')).toHaveText("Arrêter l'autre et lancer");
  await withPreview(async (preview) => {
    for (const t of THEMES)
      for (const size of SIZES) {
        await both(page, preview, size, t, 'run-conflict');
        const where = `#run-conflict ${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#run-modal');
        expect
          .soft(await lookBut(confirm.locator('.head .lead'), 'width', 'height', 'font'), `its icon, ${where}`)
          .toEqual(await lookBut(pv.locator('.m-head svg'), 'width', 'height', 'font'));
        await shots(page, preview, 'run-conflict', size, t);
      }
  });
  await confirm.getByRole('button', { name: "Arrêter l'autre et lancer" }).click();
  await expect(front).toContainText('en cours');
  await expect(inWt).toContainText('arrêté', { timeout: 15_000 });
  // Lancement folds on its title in a short window: its buttons are in the large one.
  await page.setViewportSize(SIZES[0]);
  await page.getByRole('button', { name: 'Tout arrêter', exact: true }).click();
});

test.describe('the bands above a harness’s terminal', () => {
  test.use({ appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT, ESCOUADE_OPENCODE: FAKE_OPENCODE } });

  test('Copilot CLI’s question about the folder, then OpenCode’s thread kept, as the preview draws them', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    // Copilot CLI: its interface opens on its question about the agent's folder, which the band says until it is answered.
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitemradio', { name: 'Copilot CLI' }).click();
    await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à Copilot…' })).toBeVisible();
    await page.getByRole('button', { name: 'Vue Copilot CLI' }).click();
    const tui = page.locator('main .tui');
    await expect(tui.locator('.xterm')).toBeVisible({ timeout: 20_000 });
    const band = tui.locator('.band.trust');
    await expect(band).toBeVisible({ timeout: 20_000 });
    await expect(band).toContainText('Copilot CLI demande s’il peut faire confiance au dossier de l’agent.');
    await withPreview(async (preview) => {
      for (const t of THEMES)
        for (const size of SIZES) {
          await both(page, preview, size, t, 'copilot-view');
          const where = `#copilot-view ${t} ${size.width}×${size.height}`;
          const pband = preview.locator('#tt-band');
          const head = page.locator('main.conv header.head').first();
          const phead = preview.locator('#view-tterm .head');
          expect.soft(await under(band, head), `the band's place, ${where}`).toEqual(await under(pband, phead));
          expect.soft(await look(band), `the band, ${where}`).toEqual(await look(pband));
          expect
            .soft(await lookBut(band.locator('b'), 'width'), `its question, ${where}`)
            .toEqual(await lookBut(pband.locator('b'), 'width'));
          expect.soft(await look(band.locator('svg')), `its shield, ${where}`).toEqual(await look(pband.locator('svg')));
          const [, ty] = await under(tui.locator('.box'), head);
          const [, pty] = await under(preview.locator('#tt-cp'), phead);
          expect.soft(ty, `the terminal under it, ${where}`).toBe(pty);
          await shots(page, preview, 'copilot-view', size, t);
        }
    });
    // « Yes, and remember this folder »: the band goes.
    await tui.locator('.xterm').click();
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await expect(band).toBeHidden({ timeout: 20_000 });

    // OpenCode: the green band, always.
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitemradio', { name: 'OpenCode' }).click();
    await expect(page.getByRole('textbox', { name: 'Décris la tâche à confier à OpenCode…' })).toBeVisible();
    await page.getByRole('button', { name: 'Vue OpenCode' }).click();
    const keeps = tui.locator('.band.keeps');
    await expect(keeps).toBeVisible({ timeout: 20_000 });
    await withPreview(async (preview) => {
      for (const t of THEMES)
        for (const size of SIZES) {
          await both(page, preview, size, t, 'opencode-view');
          const where = `#opencode-view ${t} ${size.width}×${size.height}`;
          const pkeep = preview.locator('#tt-keep');
          const head = page.locator('main.conv header.head').first();
          const phead = preview.locator('#view-tterm .head');
          expect.soft(await under(keeps, head), `the band's place, ${where}`).toEqual(await under(pkeep, phead));
          expect.soft(await look(keeps), `the band, ${where}`).toEqual(await look(pkeep));
          expect
            .soft(await lookBut(keeps.locator('b'), 'width'), `« Escouade garde le fil », ${where}`)
            .toEqual(await lookBut(pkeep.locator('b'), 'width'));
          expect.soft(await look(keeps.locator('svg')), `its mark, ${where}`).toEqual(await look(pkeep.locator('svg')));
          await shots(page, preview, 'opencode-view', size, t);
        }
    });
  });
});

test('the spinner of a step stops with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  fs.writeFileSync(path.join(app.repo, 'serveur.js'), 'setInterval(() => {}, 1000);\n');
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Page [ok] [recette]');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  await review.getByRole('button', { name: 'Tester' }).click();
  const dialog = page.getByRole('dialog', { name: 'Tester DEM-1' });
  await dialog.getByRole('button', { name: 'Lancer' }).click();
  const spin = dialog.locator('.lines li .spin');
  await expect(spin).toHaveCount(1, { timeout: 30_000 });
  const motion = () => spin.evaluate((e) => getComputedStyle(e).animationName);
  expect(await motion()).toMatch(/spin/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect.poll(motion).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await dialog.getByRole('button', { name: 'Tout arrêter' }).click();
});

test.describe('in English', () => {
  test.use({ language: 'en', appEnv: { ESCOUADE_OPENCODE: FAKE_OPENCODE } });

  /** What is cut short, or overflows its box, in `root`: none of it, but the user's names, folders and commands. */
  const cut = (root: Locator) =>
    root.evaluate((r) =>
      [...r.querySelectorAll<HTMLElement>('*')]
        .filter(
          (el) =>
            el.offsetParent &&
            el.children.length === 0 &&
            !el.closest('.xterm, .crumbs .where, .crumbs .cmdline') &&
            (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
        )
        .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
    );

  test('the labels of a log, a terminal and OpenCode’s band fit in their place, at both sizes', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    fs.mkdirSync(path.join(app.repo, 'web'));
    await addProject(page, app.repo, { lang: 'en' });
    // The header's « Launch commands… »: in a short window (the runner's), the section folds and « Configure » with it.
    await page.getByRole('button', { name: 'Launch commands…', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Settings' });
    await dialog.getByRole('button', { name: 'Add a command', exact: true }).click();
    const cfg = dialog.getByRole('group', { name: 'Command 1' });
    await cfg.getByLabel('Name').fill('Front');
    await cfg.getByLabel('Command').fill('Start-Sleep 600');
    await cfg.getByLabel(/Subfolder/).fill('web');
    await closeSettings(dialog);
    await unfoldSections(page);
    const front = page.locator('.run').filter({ has: page.locator('.name', { hasText: /^Front$/ }) });
    await page.getByRole('button', { name: 'Run all', exact: true }).click();
    await expect(front).toContainText('running');
    await front.click();
    const view = page.locator('main.rv');
    await expect(view.locator('.since')).toHaveText(/^since /);
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await page.mouse.move(size.width / 2, 12);
      expect(await cut(view.locator('> .head')), `the log's header, ${size.width}×${size.height}`).toEqual([]);
      for (const name of ['Clear', 'Restart', 'Stop']) await expect(view.getByRole('button', { name, exact: true })).toBeVisible();
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `run${size.name}-en.app.png`) });
    }
    await page.getByRole('button', { name: 'New terminal' }).click();
    await page.getByRole('menuitem', { name: /PowerShell/ }).click();
    const tv = page.locator('main.tv');
    await expect(tv.locator('.xterm')).toBeVisible();
    for (const size of SIZES) {
      await page.setViewportSize(size);
      expect(await cut(tv.locator('> .head')), `the terminal's header, ${size.width}×${size.height}`).toEqual([]);
      await expect(tv.getByRole('button', { name: 'Close terminal' })).toBeVisible();
    }
    await page.getByRole('button', { name: 'Other ways to create an agent' }).click();
    await page.getByRole('menuitemradio', { name: 'OpenCode' }).click();
    await page.getByRole('button', { name: 'OpenCode view' }).click();
    const keeps = page.locator('main .tui .band.keeps');
    await expect(keeps).toContainText('Escouade keeps the thread');
    for (const size of SIZES) {
      await page.setViewportSize(size);
      expect(await cut(keeps), `OpenCode's band, ${size.width}×${size.height}`).toEqual([]);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `opencode-view${size.name}-en.app.png`) });
    }
    await page.setViewportSize(SIZES[0]);
    await page.getByRole('button', { name: 'Stop all', exact: true }).click();
  });
});
