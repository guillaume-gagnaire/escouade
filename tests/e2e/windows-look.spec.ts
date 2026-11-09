// The windows in the 2.0 design (ESC-83, lot 11a): the frame every window shares (a raised panel on the scrim, its
// header of 54 px, its buttons under a rule), the assistant « Nouveau projet » (its rail, its start, its launch, its
// summary), « Ajouter un compte », « Ouvrir une PR sur GitHub », the notes of an update, and the agent's menu. Compared to
// the preview (design/app-redesign/index.html, `#wizard-private`, `#wizard`, `#wizard-summary`, `#account-codex`,
// `#pr-open`, `#update`, `#agent-menu`) in both themes at 1440 × 900 and at CI's 1028 × 779; the English labels in their
// room, nothing out of its box; the motion stopped by prefers-reduced-motion. The captures of the app and of the preview
// go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, openNewProject, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

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
const VERSION = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')).version as string;

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
async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
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
  await page.mouse.move(size.width - 4, size.height - 4);
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

/** For each theme and size, the app beside the preview at `route`. */
async function compare(page: Page, route: string, check: (preview: Page, where: string) => Promise<void>, name = route) {
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of THEMES)
      for (const size of SIZES) {
        await both(page, preview, size, t, route);
        await check(preview, `#${route} ${t} ${size.width}×${size.height}`);
        await shots(page, preview, name, size, t);
      }
  } finally {
    await browser.close();
  }
}

/**
 * The frame of a window beside the preview's: its place when `placed` (the same size asked for), its panel, its header
 * and title, its close button, its footer and its buttons.
 */
async function compareFrame(dialog: Locator, pv: Locator, where: string, placed = true) {
  if (placed) expect.soft(await place(dialog), `the window's place, ${where}`).toEqual(await place(pv));
  expect.soft(await lookBut(dialog, 'width', 'height'), `the window, ${where}`).toEqual(await lookBut(pv, 'width', 'height'));
  const head = dialog.locator('> .head');
  const phead = pv.locator('> .m-head');
  expect.soft(await lookBut(head, 'width'), `its header, ${where}`).toEqual(await lookBut(phead, 'width'));
  expect.soft(await lookBut(head.locator('h2'), 'width'), `its title, ${where}`).toEqual(await lookBut(phead.locator('h2'), 'width'));
  const [tx, ty] = await place(head.locator('h2'));
  const [dx, dy] = await place(dialog);
  const [ptx, pty] = await place(phead.locator('h2'));
  const [pdx, pdy] = await place(pv);
  // Within a pixel: the window is centred, its half-pixels rounded either way.
  expect.soft(Math.abs(tx - dx - (ptx - pdx)), `its title's left in it, ${where}`).toBeLessThanOrEqual(1);
  expect.soft(Math.abs(ty - dy - (pty - pdy)), `its title's top in it, ${where}`).toBeLessThanOrEqual(1);
  const close = head.getByRole('button', { name: /^(Fermer|Close)$/ });
  const pclose = phead.locator('.icon-btn');
  if (await pclose.count()) {
    expect.soft(await look(close), `its ×, ${where}`).toEqual(await look(pclose));
    expect.soft(await close.locator('[data-icon="close"]').count(), `its × drawn, ${where}`).toBe(1);
  }
  const foot = dialog.locator('> .foot');
  const pfoot = pv.locator('> .m-foot');
  expect.soft(await lookBut(foot, 'width'), `its footer, ${where}`).toEqual(await lookBut(pfoot, 'width'));
  const primary = foot.locator('.btn.primary');
  if (await primary.count())
    expect
      .soft(await lookBut(primary, 'width'), `its main button, ${where}`)
      .toEqual(await lookBut(pfoot.locator('.btn.primary:not([hidden])').first(), 'width'));
  const other = foot.locator('.btn:not(.primary)').last();
  if (await other.count())
    expect
      .soft(await lookBut(other, 'width'), `another button, ${where}`)
      .toEqual(await lookBut(pfoot.locator('.btn:not(.primary)').last(), 'width'));
  // The last button against the right edge, as the preview's.
  const [bx, , bw] = await place(foot.locator('.btn').last());
  const [fx, , fw] = await place(foot);
  const [pbx, , pbw] = await place(pfoot.locator('.btn:not([hidden])').last());
  const [pfx, , pfw] = await place(pfoot);
  expect.soft(Math.abs(fx + fw - (bx + bw) - (pfx + pfw - (pbx + pbw))), `the last button's margin, ${where}`).toBeLessThanOrEqual(1);
}

/** What is cut short, or out of its box, in `root`: none of it, but the user's names, folders and commands. */
const cut = (root: Locator) =>
  root.evaluate((r) => {
    const out: string[] = [];
    for (const el of r.querySelectorAll<HTMLElement>('*')) {
      if (!el.offsetParent || el.closest('.xterm, input, textarea, select, .path, .scope, .sr')) continue;
      const name = `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`;
      if (el.children.length === 0 && (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1)) out.push(name);
      const parent = el.parentElement;
      // A choice reaches into the margin by design (the preview's `.radio`); a box of `display: contents` has no edge.
      if (!parent || parent === r.parentElement || el.matches('.w-radio') || getComputedStyle(parent).display === 'contents') continue;
      const [a, b] = [el.getBoundingClientRect(), parent.getBoundingClientRect()];
      const scrolls = getComputedStyle(parent).overflowX !== 'visible';
      if (!scrolls && (a.left < b.left - 1 || a.right > b.right + 1)) out.push(`${name} out of its parent`);
    }
    return out;
  });

/** A git repository with one commit, `billing-service`, in the test's folder: the preview's new project. */
function billingService(root: string): string {
  const dir = path.join(root, 'billing-service');
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'README.md'), '# billing-service\n');
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

/** The new project's assistant on `repo`, its folder read. */
async function openAssistant(page: Page, repo: string, lang: 'fr' | 'en' = 'fr') {
  await openNewProject(page, lang);
  const dialog = page.getByRole('dialog', { name: lang === 'fr' ? 'Nouveau projet' : 'New project' });
  await dialog.getByPlaceholder(lang === 'fr' ? 'C:\\chemin\\vers\\le\\projet' : 'C:\\path\\to\\the\\project').fill(repo);
  await expect(dialog.getByText(lang === 'fr' ? /Dépôt git détecté/ : /Git repository detected/)).toBeVisible();
  return dialog;
}

test('the assistant « Nouveau projet » reads as the preview: its start, its launch, its summary', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  // A project to copy: the start offers it, as the preview's.
  await addProject(page, app.repo, { firstAgent: false });
  const other = billingService(app.root);
  const dialog = await openAssistant(page, other);
  await dialog.getByRole('radio', { name: /^Copier la configuration/ }).click();
  await expect(dialog.getByRole('group', { name: 'Projet copié' }).locator('[aria-pressed="true"]')).toHaveCount(1);
  // The click brought the radio into view: the step is compared from its top, as it opens (its tile's fields, ESC-167
  // and 168, make it taller than the preview's).
  await dialog.locator('.page').evaluate((p) => p.scrollTo(0, 0));
  // The step and the folder beside the title; the rail drawn, numbers and no glyph.
  await expect(dialog.locator('.head .chip-sm')).toHaveText([/billing-service$/, 'Étape 1 sur 8']);
  await compare(page, 'wizard-private', async (preview, where) => {
    const pv = preview.locator('#wiz-modal .modal');
    await compareFrame(dialog, pv, where);
    expect
      .soft(await lookBut(dialog.locator('[data-step-of]'), 'width'), `the step, ${where}`)
      .toEqual(await lookBut(pv.locator('#wiz-step'), 'width'));
    expect.soft(await place(dialog.locator('.rail')), `the rail, ${where}`).toEqual(await place(pv.locator('.rail')));
    const step = dialog.locator('.rail [aria-current="step"]');
    const pstep = pv.locator('.rail li[aria-current="step"]');
    expect.soft(await lookBut(step, 'width'), `the step on, ${where}`).toEqual(await lookBut(pstep, 'width'));
    expect.soft(await place(step), `its place, ${where}`).toEqual(await place(pstep));
    expect.soft(await look(step.locator('.k')), `its number, ${where}`).toEqual(await look(pstep.locator('.k')));
    const next = dialog.locator('.rail button').nth(1);
    const pnext = pv.locator('.rail li').nth(1);
    expect.soft(await lookBut(next, 'width'), `a step to come, ${where}`).toEqual(await lookBut(pnext, 'width'));
    expect.soft(await place(next), `its place, ${where}`).toEqual(await place(pnext));
    const h = dialog.locator('.page h3');
    const ph = pv.locator('#wiz-start h3');
    expect.soft(await lookBut(h, 'width'), `the step's title, ${where}`).toEqual(await lookBut(ph, 'width'));
    expect.soft((await place(h)).slice(0, 2), `its place, ${where}`).toEqual((await place(ph)).slice(0, 2));
    expect
      .soft(await lookBut(dialog.locator('.page .lead'), 'width', 'height'), `what it is for, ${where}`)
      .toEqual(await lookBut(pv.locator('#wiz-start .wiz-lead'), 'width', 'height'));
    const radio = dialog.getByRole('radio', { name: /^Copier la configuration/ });
    const pradio = pv.locator('#wiz-start .radio[aria-checked="true"]');
    expect.soft(await lookBut(radio, 'width', 'height'), `the start chosen, ${where}`).toEqual(await lookBut(pradio, 'width', 'height'));
    expect.soft(await look(radio.locator('.dot')), `its dot, ${where}`).toEqual(await look(pradio.locator('.dot')));
    expect.soft(await lookBut(radio.locator('b'), 'width'), `its name, ${where}`).toEqual(await lookBut(pradio.locator('b'), 'width'));
    expect
      .soft(await lookBut(radio.locator('.d'), 'width', 'height'), `what it does, ${where}`)
      .toEqual(await lookBut(pradio.locator('span span'), 'width', 'height'));
    const unchecked = dialog.getByRole('radio', { name: /^Partir de zéro/ });
    const punchecked = pv.locator('#wiz-start .radio[aria-checked="false"]').last();
    expect.soft(await look(unchecked.locator('.dot')), `a start not chosen, ${where}`).toEqual(await look(punchecked.locator('.dot')));
    expect
      .soft(await lookBut(dialog.locator('.w-chip[aria-pressed="true"]'), 'width'), `the project copied, ${where}`)
      .toEqual(await lookBut(pv.locator('#wiz-start .chip-t[aria-pressed="true"]'), 'width'));
  });

  // Its launch, a command in a row as the preview's table.
  await dialog.getByRole('radio', { name: /^Partir de zéro/ }).click();
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
  await expect(dialog.getByRole('heading', { name: 'Lancement' })).toBeVisible();
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const command = dialog.getByRole('group', { name: 'Commande 1' });
  await command.getByRole('textbox', { name: 'Nom' }).fill('API');
  await command.getByRole('textbox', { name: 'Commande' }).fill('npm run dev:api');
  await expect(dialog.locator('.rail .done [data-icon="check"]')).toHaveCount(2);
  await expect(dialog.getByRole('button', { name: 'Remplir automatiquement' }).locator('[data-icon="sparkle"]')).toHaveCount(1);
  await compare(page, 'wizard', async (preview, where) => {
    const pv = preview.locator('#wiz-modal .modal');
    await compareFrame(dialog, pv, where);
    const done = dialog.locator('.rail .done').first();
    const pdone = pv.locator('.rail li.ok').first();
    expect.soft(await lookBut(done, 'width'), `a step done, ${where}`).toEqual(await lookBut(pdone, 'width'));
    expect.soft(await look(done.locator('.k')), `its tick, ${where}`).toEqual(await look(pdone.locator('.k')));
    const head = dialog.locator('.cmd.head');
    const phead = pv.locator('.cmd-head');
    expect.soft(await lookBut(head, 'width', 'height'), `the columns' names, ${where}`).toEqual(await lookBut(phead, 'width', 'height'));
    const [x, , w] = await place(head);
    const [px, , pw] = await place(phead);
    expect.soft([x, w], `their place, ${where}`).toEqual([px, pw]);
    for (const [i, name] of ['Nom', 'Commande', 'Sous-dossier'].entries()) {
      const field = command.getByRole('textbox', { name: new RegExp(`^${name}`) });
      const pfield = pv.locator('.launch-row:not(.cmd-head)').first().locator('.inp').nth(i);
      expect.soft(await lookBut(field, 'height'), `« ${name} », ${where}`).toEqual(await lookBut(pfield, 'height'));
      expect.soft((await place(field))[0], `its column, ${where}`).toBe((await place(pfield))[0]);
    }
    const add = dialog.getByRole('button', { name: 'Ajouter une commande', exact: true });
    expect
      .soft(await lookBut(add, 'width'), `« Ajouter une commande », ${where}`)
      .toEqual(await lookBut(pv.locator('#wiz-launch .btn').first(), 'width'));
  });

  // Its summary: the check that passes, then what will be set.
  await dialog.getByRole('button', { name: /Récapitulatif/ }).click();
  await expect(dialog.locator('.ck.ok [data-icon="check-c"]')).toHaveCount(1);
  await compare(page, 'wizard-summary', async (preview, where) => {
    const pv = preview.locator('#wiz-modal .modal');
    await compareFrame(dialog, pv, where);
    const ok = dialog.locator('.ck.ok');
    const pok = pv.locator('.ck.ok');
    expect.soft(await lookBut(ok, 'width'), `the check that passes, ${where}`).toEqual(await lookBut(pok, 'width'));
    expect.soft((await place(ok)).slice(0, 1), `its place, ${where}`).toEqual((await place(pok)).slice(0, 1));
    expect.soft(await look(ok.locator('svg')), `its icon, ${where}`).toEqual(await look(pok.locator('svg')));
    expect
      .soft(await lookBut(dialog.locator('.sum'), 'width', 'height'), `what will be set, ${where}`)
      .toEqual(await lookBut(pv.locator('.sum'), 'width', 'height'));
    for (const cell of ['dt', 'dd'])
      expect
        .soft(await lookBut(dialog.locator(`.sum ${cell}`).first(), 'width'), `a ${cell}, ${where}`)
        .toEqual(await lookBut(pv.locator(`.sum ${cell}`).first(), 'width'));
  });
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
});

test('« Ajouter un compte » reads as the preview, then its sign-in in a well', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Comptes et harness');
  await settings.getByRole('button', { name: 'Ajouter un compte Claude Code…' }).click();
  const dialog = page.getByRole('dialog', { name: /^Ajouter un compte/ });
  await expect(dialog.getByRole('radio', { name: /Un nouveau compte/ })).toBeChecked();
  await compare(page, 'account-codex', async (preview, where) => {
    const pv = preview.locator('#acct-modal .modal');
    await compareFrame(dialog, pv, where, false);
    const [, , w] = await place(dialog);
    expect.soft(w, `its width, ${where}`).toBe((await place(pv))[2]);
    const radio = dialog.getByRole('radio', { name: /Un nouveau compte/ });
    const pradio = pv.locator('#acct-r-new');
    expect
      .soft(await lookBut(radio, 'width', 'height'), `a new account, chosen, ${where}`)
      .toEqual(await lookBut(pradio, 'width', 'height'));
    expect.soft(await look(radio.locator('.dot')), `its dot, ${where}`).toEqual(await look(pradio.locator('.dot')));
    const machine = dialog.getByRole('radio', { name: /Le compte de la machine/ });
    expect.soft(await look(machine.locator('.dot')), `the machine's, ${where}`).toEqual(await look(pv.locator('#acct-r-machine .dot')));
    expect
      .soft(await lookBut(dialog.locator('.w-fld .lab').first(), 'width'), `a field's name, ${where}`)
      .toEqual(await lookBut(pv.locator('.fld .lb').first(), 'width'));
    const name = dialog.getByRole('textbox', { name: 'Nom' });
    expect
      .soft(await lookBut(name, 'width', 'height', 'font'), `the name, ${where}`)
      .toEqual(await lookBut(pv.locator('#acct-name'), 'width', 'height', 'font'));
    // A file shared, when the machine's account has one (the test's may have none).
    const chip = dialog.locator('.w-chip.pick').first();
    if (await chip.count())
      expect
        .soft(await lookBut(chip, 'width', 'font'), `a file shared, ${where}`)
        .toEqual(await lookBut(pv.locator('.chip-t[aria-pressed="true"]').first(), 'width', 'font'));
    expect
      .soft(await lookBut(dialog.locator('.help').last(), 'width', 'height'), `what is never shared, ${where}`)
      .toEqual(await lookBut(pv.locator('.acct-new .help').first(), 'width', 'height'));
  });

  // Its sign-in: the program in a well, what is awaited beside a spinner, which prefers-reduced-motion stops.
  await dialog.getByRole('textbox', { name: 'Nom' }).fill('Équipe');
  await dialog.getByRole('button', { name: 'Créer et se connecter' }).click();
  const signin = page.getByRole('dialog', { name: 'Se connecter au compte « Équipe »' });
  await expect(signin.locator('.term .xterm')).toBeVisible({ timeout: 20_000 });
  const spin = signin.locator('.said .spin');
  await expect(spin).toHaveCount(1);
  await compare(
    page,
    'account-codex',
    async (preview, where) => {
      const pv = preview.locator('#acct-modal .modal');
      expect
        .soft(
          await lookBut(signin.locator('.term'), 'width', 'height', 'padding', 'color', 'border', 'font'),
          `the terminal's well, ${where}`,
        )
        .toEqual(await lookBut(pv.locator('.mini-term'), 'width', 'height', 'padding', 'color', 'border', 'font'));
      expect
        .soft(await lookBut(signin.locator('.said'), 'width', 'height'), `what is awaited, ${where}`)
        .toEqual(await lookBut(pv.locator('.waitline'), 'width', 'height'));
    },
    'account-signin',
  );
  await page.evaluate(() => document.getElementById('still')?.remove());
  const motion = () => spin.evaluate((e) => getComputedStyle(e).animationName);
  expect(await motion()).toMatch(/spin/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect.poll(motion).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await signin.getByRole('button', { name: 'Fermer' }).click();
  await expect(signin).toBeHidden();
});

test.describe('« Ouvrir une PR sur GitHub »', () => {
  let github: { url: string; close: () => Promise<void> };
  test.beforeAll(async () => {
    github = await startFakeTrackers(0);
    process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
    process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
  });
  test.afterAll(async () => {
    await github?.close();
    delete process.env.ESCOUADE_GITHUB_API;
    delete process.env.ESCOUADE_PR_READ_EVERY_MS;
  });

  test('reads as the preview: what is proposed, the target, the draft, the files left out', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
    const bare = path.join(app.root, 'origin.git');
    git(app.root, 'init', '-q', '--bare', bare);
    const url = 'https://github.com/acme/demo.git';
    git(app.repo, 'remote', 'add', 'origin', url);
    git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
    git(app.repo, 'push', '-q', 'origin', 'main');
    await addProject(page, app.repo, { worktrees: true, firstAgent: false });
    await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
    await expect(page.locator('.side .card')).toHaveCount(1, { timeout: 60_000 });
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Services connectés');
    await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
    const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
    await form.getByLabel(/Jeton/).fill('ghp_jeton');
    await form.getByRole('button', { name: 'Connecter' }).click();
    await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
    await closeSettings(settings);
    await expect(settings).toHaveCount(0);
    await page.getByRole('button', { name: /^Kanban/ }).click();
    await page.getByRole('button', { name: 'Agents', exact: true }).click();
    await page.locator('.side .card').first().click();
    const worktrees = () =>
      git(app.repo, 'worktree', 'list', '--porcelain')
        .split('\n')
        .filter((l) => l.startsWith('worktree '))
        .map((l) => l.slice('worktree '.length))
        .slice(1);
    await expect.poll(() => worktrees().length).toBe(1);
    const wt = worktrees()[0];
    fs.writeFileSync(path.join(wt, 'login.ts'), 'export const login = 1;\n');
    git(wt, 'add', '-A');
    git(wt, 'commit', '-qm', 'feat: connexion par jeton');
    // Two files not committed: said, as the preview says them.
    fs.writeFileSync(path.join(wt, 'a.ts'), '1\n');
    fs.writeFileSync(path.join(wt, 'b.ts'), '2\n');
    await page.getByRole('button', { name: 'Ouvrir une PR' }).click();
    const dialog = page.getByRole('dialog', { name: 'Ouvrir une PR sur GitHub' });
    await expect(dialog.getByRole('textbox', { name: 'Titre' })).toHaveValue(/.+/);
    await expect(dialog.locator('.w-notice')).toContainText('2 fichiers non commités du worktree ne seront pas dans la PR.');
    await expect(dialog.getByRole('button', { name: 'Pousser et ouvrir' }).locator('[data-icon="push"]')).toHaveCount(1);
    await expect(dialog.getByRole('button', { name: 'Proposer à nouveau' }).locator('[data-icon="sparkle"]')).toHaveCount(1);
    await compare(page, 'pr-open', async (preview, where) => {
      const pv = preview.locator('#pr-modal .modal');
      await compareFrame(dialog, pv, where, false);
      expect.soft((await place(dialog))[2], `its width, ${where}`).toBe((await place(pv))[2]);
      expect
        .soft(await lookBut(dialog.locator('.head .lead'), 'width', 'height', 'font'), `its icon, ${where}`)
        .toEqual(await lookBut(pv.locator('.m-head > svg'), 'width', 'height', 'font'));
      expect
        .soft(await lookBut(dialog.locator('.scope'), 'width', 'height'), `the agent and its branches, ${where}`)
        .toEqual(await lookBut(pv.locator('.scope'), 'width', 'height'));
      expect
        .soft(await lookBut(dialog.locator('.w-fld .lab').first(), 'width'), `a field's name, ${where}`)
        .toEqual(await lookBut(pv.locator('.fld label').first(), 'width'));
      expect
        .soft(await lookBut(dialog.getByRole('textbox', { name: 'Titre' }), 'width'), `the title, ${where}`)
        .toEqual(await lookBut(pv.locator('#pr-t'), 'width'));
      expect
        .soft(await lookBut(dialog.getByRole('textbox', { name: 'Description' }), 'width', 'height'), `the description, ${where}`)
        .toEqual(await lookBut(pv.locator('.ta'), 'width', 'height'));
      expect
        .soft(await look(dialog.locator('.w-tick .tick-box')), `the draft's box, ${where}`)
        .toEqual(await look(pv.locator('.cbline .cb')));
      const notice = dialog.locator('.w-notice');
      const pnotice = pv.locator('.notice');
      expect
        .soft(await lookBut(notice, 'width', 'height'), `the files left out, ${where}`)
        .toEqual(await lookBut(pnotice, 'width', 'height'));
      expect.soft(await look(notice.locator('svg')), `its icon, ${where}`).toEqual(await look(pnotice.locator('svg')));
    });
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
  });
});

test.describe('the notes of an update installed', () => {
  test.use({
    preStart: {
      run: (root: string) => {
        fs.writeFileSync(
          path.join(root, 'data', 'update.json'),
          JSON.stringify({
            version: VERSION,
            notes:
              '- Codex, deuxième harness : passation d’un agent à l’autre\n- Conversation vocale avec un agent\n- Sous-agents et tâches de fond dans la barre latérale',
          }),
        );
        return {};
      },
    },
  });

  test('read as the preview’s window of the update, its icon before the title', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    await page.locator('.toast').getByRole('button', { name: 'Voir les nouveautés' }).click();
    const dialog = page.getByRole('dialog', { name: `Nouveautés d’Escouade ${VERSION}` });
    await expect(dialog.locator('.head [data-icon="download"]')).toHaveCount(1);
    await expect(dialog.locator('.md li')).toHaveCount(3);
    await compare(page, 'update', async (preview, where) => {
      const pv = preview.locator('#upd-modal .modal');
      await compareFrame(dialog, pv, where, false);
      expect.soft((await place(dialog))[2], `its width, ${where}`).toBe((await place(pv))[2]);
      expect
        .soft(await lookBut(dialog.locator('.head .lead'), 'width', 'height', 'font'), `its icon, ${where}`)
        .toEqual(await lookBut(pv.locator('.m-head > svg'), 'width', 'height', 'font'));
      expect
        .soft(await lookBut(dialog.locator('.md li').first(), 'width'), `a change, ${where}`)
        .toEqual(await lookBut(pv.locator('.m-body li').first(), 'width'));
    });
    // Its rise stopped by prefers-reduced-motion.
    await page.evaluate(() => document.getElementById('still')?.remove());
    expect(await dialog.evaluate((e) => getComputedStyle(e).animationName)).toMatch(/rise/);
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await expect.poll(() => dialog.evaluate((e) => getComputedStyle(e).animationName)).toBe('none');
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await dialog.getByRole('button', { name: 'Fermer' }).last().click();
  });
});

test('the agent’s menu reads as the preview’s', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.locator('main.conv header.head').getByRole('button', { name: 'Actions de l’agent' }).click();
  const menu = page.getByRole('menu');
  await expect(menu).toBeVisible();
  await compare(page, 'agent-menu', async (preview, where) => {
    const pv = preview.locator('#agent-menu');
    expect.soft(await lookBut(menu, 'width', 'height'), `the menu, ${where}`).toEqual(await lookBut(pv, 'width', 'height'));
    const item = menu.getByRole('menuitem', { name: /^Renommer/ });
    const pitem = pv.locator('.mi').first();
    expect.soft(await lookBut(item, 'width'), `an entry, ${where}`).toEqual(await lookBut(pitem, 'width'));
    expect.soft(await lookBut(item.locator('.l'), 'width'), `its name, ${where}`).toEqual(await lookBut(pitem.locator('b'), 'width'));
  });
  await page.keyboard.press('Escape');
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the assistant, « Add an account » and their labels fit in their place, at both sizes', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false, lang: 'en' });
    const other = billingService(app.root);
    const dialog = await openAssistant(page, other, 'en');
    const fits = async (what: string) => {
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await page.mouse.move(size.width - 4, size.height - 4);
        expect.soft(await cut(dialog), `${what}, ${size.width}×${size.height}`).toEqual([]);
        const [x, y, w, h] = await place(dialog);
        expect.soft(x >= 0 && y >= 0 && x + w <= size.width && y + h <= size.height, `${what} in the window, ${size.width}`).toBe(true);
        if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `${what}${size.name}-en.app.png`) });
      }
    };
    await dialog.getByRole('radio', { name: /^Copy a project/ }).click();
    await fits('wizard-private');
    await dialog.getByRole('radio', { name: /^Start from scratch/ }).click();
    await dialog.getByRole('button', { name: 'Next', exact: true }).click();
    await fits('wizard-worktrees');
    await dialog.getByRole('button', { name: 'Next', exact: true }).click();
    await dialog.getByRole('button', { name: 'Add a command', exact: true }).click();
    await fits('wizard');
    await dialog.getByRole('button', { name: 'Next', exact: true }).click();
    await fits('wizard-kanban');
    await dialog.getByRole('button', { name: /Summary/ }).click();
    await fits('wizard-summary');
    await page.keyboard.press('Escape');

    await page.getByRole('button', { name: 'Settings (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Settings' });
    await openSettingsPage(settings, 'Accounts and harnesses');
    await settings.getByRole('button', { name: 'Add a Claude Code account…' }).click();
    const account = page.getByRole('dialog', { name: /^Add a/ });
    // Captured once risen (a window half-way up shows what is under it).
    await page.evaluate(() =>
      Promise.all(
        document
          .getAnimations()
          .filter((a) => a.effect?.getComputedTiming().iterations !== Infinity)
          .map((a) => a.finished.catch(() => {})),
      ),
    );
    for (const size of SIZES) {
      await page.setViewportSize(size);
      expect.soft(await cut(account), `« Add an account », ${size.width}×${size.height}`).toEqual([]);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `account-codex${size.name}-en.app.png`) });
    }
  });
});
