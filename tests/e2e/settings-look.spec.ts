// The settings window of direction A (ESC-379, R2 of the 2.2 redesign): the bar of the categories on the left (the
// search, Escouade's pages, the projects with the one chosen unfolded on its pages, « Tous les projets », « À propos » and
// the version at its foot), the page's title, its settings in rows under hairlines, « Avancé » folded at its foot, and
// the window tinted by what it sets (Escouade's teal, or the project's colour). Compared to the previews
// (design/settings-2-2/index.html, `#a-<screen>`) in both themes at 1440 × 900 and at CI's 1028 × 779, where the
// window takes the room there is; the English labels in their room; nothing that overflows at 1028, 1280 and 1440; the
// motion stopped by prefers-reduced-motion. The blocks of the 2.0 tabs still shown (a private project's files, the
// quotas) are compared to design/app-redesign/index.html until their lot redraws them. The captures of the app and of
// the preview go to ESCOUADE_E2E_SHOTS when it is set.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, settingsEntry, test } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
/** The previews of direction A; they load Inter from the root's packages. */
const PREVIEW = path.join(ROOT, 'design', 'settings-2-2', 'index.html');
/** The preview of the 2.0 design, for the blocks of the old tabs; it loads Inter from the site's packages. */
const PREVIEW_20 = path.join(ROOT, 'design', 'app-redesign', 'index.html');
const PREVIEW_20_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1028, height: 779, name: '-small' },
];
/** Escouade's teal (src/lib/theme.ts) and the first project's colour, as the browser computes them. */
const TEAL = 'oklch(0.74 0.115 190)';
const FIRST = 'oklch(0.72 0.12 48)';

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
      border: `${s.borderTopWidth} ${s.borderTopStyle} ${s.borderTopColor} / ${s.borderRightWidth} ${s.borderBottomWidth}`,
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

/** Nothing glides nor rises: the tint is read where it ends. */
const freeze = (page: Page) =>
  page.evaluate(() => {
    if (document.getElementById('still')) return;
    const style = document.createElement('style');
    style.id = 'still';
    style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
    document.head.append(style);
  });

/** The colour the window is tinted with, as computed. */
const tint = (page: Page) => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--p').trim());

/** The theme of the app, set as « Thème » would (its effect without its dialog). */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** A preview at `#route`, in theme `t`, tinted as the app is (its teal is a hex of the same colour, the app's is exact). */
async function previewAt(preview: Page, file: string, route: string, t: 'dark' | 'light', p?: string) {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(file).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await freeze(preview);
  if (p) await preview.evaluate((p) => document.documentElement.style.setProperty('--p', p), p);
}

/** Both captures, when asked for. */
async function shoot(page: Page, preview: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.screenshot({ path: path.join(SHOTS, `app-${name}.png`) });
  await preview.screenshot({ path: path.join(SHOTS, `preview-${name}.png`) });
}

/** The settings open on a page, the pointer and the focus out of the way. */
async function openPage(page: Page, name: string) {
  const dialog = await openSettingsPage(page, name);
  await page.mouse.move(2, 400);
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  return dialog;
}

/** The pages compared, by their name in the bar, their preview, and the colour they tint the window with. */
const PAGES = [
  ['Général', 'a-general', TEAL],
  ['Comptes et harness', 'a-comptes', TEAL],
  ['Plugins', 'a-plugins', TEAL],
  ['Projet', 'a-projet', FIRST],
] as const;

/** The window, its bar and its header (no foot: each setting is saved as it changes), and what its page shares with the preview's. */
async function compareFrame(page: Page, preview: Page, route: string, small: boolean, where: string) {
  const win = page.locator('.settings[role="dialog"]');
  const pwin = preview.locator('.win.a');
  // At 1028 × 779 the preview keeps its 1120 × 780; the app takes the room there is (below).
  const size = small ? ['width', 'height'] : [];
  if (!small) expect.soft(await place(win), `the window's place, ${where}`).toEqual(await place(pwin));
  expect.soft(await lookBut(win, ...size), `the window, ${where}`).toEqual(await lookBut(pwin, ...size));
  const project = route === 'a-projet';
  const pairs: [string, Locator, Locator, string[]][] = [
    ['the bar', win.locator('.s-nav'), pwin.locator('.a-nav'), small ? ['height'] : []],
    ['the search', win.locator('.s-nav .search'), pwin.locator('.a-nav .field.search'), []],
    ['its shortcut', win.locator('.s-nav kbd'), pwin.locator('.a-nav kbd'), ['width']],
    ['a heading of the bar', win.locator('.s-h').first(), pwin.locator('.a-h').first(), ['width']],
    ['the page chosen', win.locator('.s-i[aria-current="page"]'), pwin.locator('.a-i.on'), []],
    [
      'another page',
      win.locator('.s-i:not([aria-current]):not(.sub):not([aria-expanded])').first(),
      pwin.locator('.a-i:not(.on):not(.sub)').first(),
      [],
    ],
    ['the foot of the bar', win.locator('.s-nav .bot'), pwin.locator('.a-bot'), []],
    ['the version', win.locator('.s-nav .ver'), pwin.locator('.a-bot .ver'), ['width']],
    ['the header', win.locator('.s-head'), pwin.locator('.a-main .head'), size],
    ['the page’s title', win.locator('.s-head h1'), pwin.locator('.a-main .head h1'), ['width']],
    ['what it sets', win.locator('.s-head p'), pwin.locator('.a-main .head p'), ['width', 'height']],
    ['« Fermer »', win.locator('.s-head .close'), pwin.locator('.a-main .head .close'), []],
    ['the body', win.locator('.s-body'), pwin.locator('.a-body'), ['height', ...size]],
    ['its column', win.locator('.s-body .inner'), pwin.locator('.a-body > .inner'), ['height', ...size]],
  ];
  if (project)
    pairs.push(
      ['a page of the project', win.locator('.s-i.sub:not([aria-current])').first(), pwin.locator('.a-i.sub:not(.on)').first(), []],
      ['its tile', win.locator('.s-head .tile'), pwin.locator('.a-main .head .tile'), []],
    );
  for (const [what, a, p, skip] of pairs) {
    expect.soft(await lookBut(a, ...skip), `${what}, ${where}`).toEqual(await lookBut(p, ...skip));
    if (!skip.length && !small) expect.soft(await place(a), `${what}'s place, ${where}`).toEqual(await place(p));
  }
  // The settings: a group's title, a row, its label and what it does, a switch, segments, « Avancé » folded, wherever
  // both have one.
  const body = win.locator('.s-body');
  const pbody = pwin.locator('.a-body');
  const common: [string, string, string, string[]][] = [
    ['a group’s title', '.sg h2', '.sg h2', ['width']],
    ['a row', '.sr', '.sr', ['width', 'height']],
    ['a row’s label', '.sr .lab > b', '.sr .lab > b', ['width', 'height']],
    ['what it does', '.sr .lab > .d', '.sr .lab > .d', ['width', 'height']],
    ['a group’s note', '.sg > .note', '.sg > .note', ['width', 'height']],
    ['a switch', '.ctl .switch[aria-checked="false"]', '.ctl .switch[aria-checked="false"]', []],
    ['a switch on', '.ctl .switch[aria-checked="true"]', '.ctl .switch[aria-checked="true"]', ['background']],
    ['segments', '.ctl .segmented', '.ctl .segmented', ['width']],
    ['the segment chosen', '.ctl .segmented [aria-pressed="true"]', '.ctl .segmented [aria-pressed="true"]', ['width']],
    ['« Avancé », folded', '.fold', '.fold', ['width']],
    ['what it hides', '.fold .names', '.fold .names', ['width']],
  ];
  for (const [what, sel, psel, skip] of common) {
    const a = body.locator(sel).first();
    const p = pbody.locator(psel).first();
    if (!(await a.count()) || !(await p.count())) continue;
    expect.soft(await lookBut(a, ...skip), `${what}, ${where}`).toEqual(await lookBut(p, ...skip));
  }
}

test('the settings look as the previews of direction A draw them, tinted by what they set, in both themes, at both sizes', async ({
  app,
}) => {
  test.setTimeout(400_000);
  test.skip(!fs.existsSync(chromium.executablePath()), 'the previews need Playwright’s Chromium');
  const { page } = app;
  await addProject(page, app.repo);
  await freeze(page);
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await theme(page, t);
        await page.evaluate(() => document.fonts.ready);
        const small = size.width < 1440;
        for (const [name, route, color] of PAGES) {
          const dialog = await openPage(page, name);
          const where = `${name}, ${t} ${size.width}×${size.height}`;
          // The tint follows what the page sets.
          expect.soft(await tint(page), `the tint, ${where}`).toBe(color);
          await expect(dialog).toHaveAttribute('data-scope', color === TEAL ? 'app' : 'project');
          // Each harness's state, read a moment after the page opens: shot once Claude Code's is drawn.
          if (route === 'a-comptes') await expect(dialog.locator('[data-engine="claude"] .hstate')).toBeVisible();
          await previewAt(preview, PREVIEW, route, t, await tint(page));
          await shoot(page, preview, `${route}${size.name}-${t}`);
          await compareFrame(page, preview, route, small, where);
          // In CI's window, the room there is: 16 px around it.
          if (small) expect.soft(await place(page.locator('.settings[role="dialog"]')), `the window, ${where}`).toEqual([16, 16, 996, 747]);
        }
      }
  } finally {
    await browser.close();
  }
  // Closed, the window has the app's own tint back: the project's.
  await page.keyboard.press('Escape');
  await expect.poll(() => tint(page)).toBe(FIRST);
  await openPage(page, 'Général');
  await page.keyboard.press('Escape');
  await expect.poll(() => tint(page)).toBe(FIRST);
});

test('the bar goes from page to page by the keyboard, and « Avancé » unfolds in place, saying what it hides', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const dialog = await openSettingsPage(page, 'Général');
  const nav = dialog.getByRole('navigation');
  await settingsEntry(dialog, 'Général').focus();
  await page.keyboard.press('ArrowDown');
  await expect(settingsEntry(dialog, 'Agents')).toHaveAttribute('aria-current', 'page');
  await expect(settingsEntry(dialog, 'Agents')).toBeFocused();
  await page.keyboard.press('End');
  await expect(settingsEntry(dialog, 'À propos')).toHaveAttribute('aria-current', 'page');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  // Gone through, the project opens on its page « Projet ».
  await expect(nav.locator('button[aria-expanded="true"]')).toBeFocused();
  await expect(settingsEntry(dialog, 'Projet')).toHaveAttribute('aria-current', 'page');
  await expect(dialog.getByRole('heading', { level: 1 })).toHaveText('demo-api');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await expect(settingsEntry(dialog, 'Git et worktrees')).toHaveAttribute('aria-current', 'page');
  await expect(dialog.getByRole('heading', { level: 1 })).toHaveText('Git et worktrees');
  await page.keyboard.press('Home');
  await expect(settingsEntry(dialog, 'Général')).toBeFocused();
  // « Avancé », folded at the foot of « Projet », says what it hides; unfolded, « Fermer le projet… » is there.
  await openSettingsPage(dialog, 'Projet');
  const fold = dialog.locator('#settings-panel').getByRole('button', { name: /^Avancé/ });
  await expect(fold).toHaveAttribute('aria-expanded', 'false');
  await expect(fold).toContainText('Fermer le projet');
  await expect(dialog.getByRole('button', { name: 'Fermer le projet…' })).toHaveCount(0);
  await fold.click();
  await expect(dialog.getByRole('button', { name: 'Fermer le projet…' })).toBeVisible();
  // A change is saved as it is made (ESC-380): no dot of a change waiting, here or on the project folded.
  await openSettingsPage(dialog, 'Projet');
  await dialog.getByRole('textbox', { name: 'Nom du projet' }).fill('demo-api-2');
  await openSettingsPage(dialog, 'Général');
  await expect(dialog).toHaveAttribute('data-saving', 'false');
  await expect(nav.locator('button[aria-expanded="false"]')).toHaveText(/demo-api-2/);
  await expect(dialog.locator('.s-nav .chg')).toHaveCount(0);
  await expect(page.getByRole('navigation', { name: 'Projets' }).getByRole('tab', { name: /demo-api-2/ })).toBeVisible();
});

/**
 * What overflows in `root`: a box that clips its content (but an ellipsis, which is meant), a box out of `root`, two of
 * the boxes of `rows` (the children of each) that overlap, a line meant to hold on one that went on two.
 */
function overflows(root: Locator, rows: string[]) {
  return root.evaluate((top, rows) => {
    const out: string[] = [];
    const name = (el: Element) =>
      `${el.tagName.toLowerCase()}${
        typeof el.className === 'string' && el.className
          ? '.' +
            el.className
              .trim()
              .split(/\s+/)
              .filter((c) => !c.startsWith('svelte-'))
              .join('.')
          : ''
      }${el.textContent?.trim() ? ` « ${el.textContent.trim().slice(0, 40)} »` : ''}`;
    const shown = (el: Element) => {
      const r = el.getBoundingClientRect();
      return r.width > 1 && r.height > 1 && getComputedStyle(el).visibility !== 'hidden';
    };
    const box = top.getBoundingClientRect();
    for (const el of [top, ...top.querySelectorAll('*')]) {
      if (!shown(el) || (el.closest('svg') !== null && el.tagName !== 'svg') || el.closest('option')) continue;
      const s = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      const clips = s.overflowX === 'hidden' || s.overflowX === 'clip';
      // A field holds what the user typed (a path): it scrolls by design.
      const field = el.matches('input, textarea, select');
      if (!field && el.scrollWidth > el.clientWidth + 1 && clips && s.textOverflow !== 'ellipsis')
        out.push(`${name(el)} cut: ${el.scrollWidth} > ${el.clientWidth}`);
      if (el !== top && !el.closest('.s-body') && (r.left < box.left - 1 || r.right > box.right + 1))
        out.push(`${name(el)} out of the window`);
      // In the body, out of its width (it scrolls only up and down).
      const scroller = el.closest('.s-body');
      if (scroller && el !== scroller) {
        const b = scroller.getBoundingClientRect();
        if (r.left < b.left - 1 || r.right > b.right + 1) out.push(`${name(el)} out of the body`);
      }
      for (const node of el.childNodes) {
        if (node.nodeType !== Node.TEXT_NODE || !node.textContent?.trim()) continue;
        const range = document.createRange();
        range.selectNodeContents(node);
        const tops = new Set([...range.getClientRects()].filter((q) => q.width > 0).map((q) => Math.round(q.top)));
        if (tops.size > 1 && (s.whiteSpace === 'nowrap' || el.closest('.s-i, .btn, .s-head h1, .hg h2, .fold, .segmented')))
          out.push(`${name(el)} on ${tops.size} lines`);
      }
    }
    for (const sel of rows)
      for (const row of top.querySelectorAll(sel)) {
        const kids = [...row.children].filter(shown).map((el) => [el, el.getBoundingClientRect()] as const);
        for (let i = 0; i < kids.length; i++)
          for (let j = i + 1; j < kids.length; j++) {
            const [a, ra] = kids[i];
            const [b, rb] = kids[j];
            const x = Math.min(ra.right, rb.right) - Math.max(ra.left, rb.left);
            const y = Math.min(ra.bottom, rb.bottom) - Math.max(ra.top, rb.top);
            if (x > 1 && y > 1) out.push(`${name(a)} over ${name(b)}`);
          }
      }
    // A row's label keeps its room beside its control.
    for (const lab of top.querySelectorAll('.sr .lab')) {
      const w = lab.getBoundingClientRect().width;
      if (shown(lab) && w < 140 && lab.closest('.sr:not(.wide)')) out.push(`${name(lab)} squeezed to ${Math.round(w)} px`);
    }
    return out;
  }, rows);
}

/** Every page of the bar, the project's unfolded, read to its end at 1028, 1280 and 1440 px: what overflows or overlaps. */
async function measure(page: Page, dialog: Locator, lang: string) {
  const problems: string[] = [];
  const pages = (await dialog.getByRole('navigation').locator('button:not([aria-expanded]) .lbl').allTextContents()).map((s) => s.trim());
  expect(pages.length).toBeGreaterThan(14);
  for (const size of [
    { width: 1028, height: 779 },
    { width: 1280, height: 800 },
    { width: 1440, height: 900 },
  ]) {
    await page.setViewportSize(size);
    for (const name of pages) {
      await openSettingsPage(dialog, name);
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
      for (const p of await overflows(dialog, ['.sr', '.l1', '.l2', '.cmd-row', '.step', '.s-head', '.s-i', '.hg', '.mrow', '.item']))
        problems.push(`${lang} ${size.width}: ${name}: ${p}`);
    }
  }
  return problems;
}

test('nothing of the settings overflows nor overlaps, at 1028, 1280 and 1440 px, in French', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  await addProject(page, app.repo);
  // A private project shows its files and its accounts, and its lock in the bar.
  const dialog = await openSettingsPage(page, 'Confidentialité');
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  expect(await measure(page, dialog, 'fr')).toEqual([]);
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the settings fit in their place, at 1028, 1280 and 1440 px', async ({ app }) => {
    test.setTimeout(300_000);
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    const dialog = await openSettingsPage(page, 'Privacy');
    await expect(dialog.getByRole('navigation').locator('.s-h')).toHaveText(['Escouade', 'Projects']);
    await dialog.getByRole('switch', { name: 'Private project' }).click();
    expect(await measure(page, dialog, 'en')).toEqual([]);
    if (SHOTS)
      for (const name of ['Accounts and harnesses', 'Voice', 'Project']) {
        await page.setViewportSize({ width: 1028, height: 779 });
        await openSettingsPage(dialog, name);
        await page.screenshot({ path: path.join(SHOTS, `app-settings-${name.toLowerCase().replace(/\W+/g, '-')}-small-en.png`) });
      }
  });
});

test('the settings open still with prefers-reduced-motion, their icons drawn', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const motion = (el: Locator) => el.evaluate((e) => getComputedStyle(e).animationName);
  await openPage(page, 'Comptes et harness');
  const win = page.locator('.settings[role="dialog"]');
  expect(await motion(win)).toMatch(/rise/);
  await page.keyboard.press('Escape');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openPage(page, 'Comptes et harness');
  expect(await motion(win)).toBe('none');
  expect(await motion(page.locator('.overlay:has(.settings)'))).toBe('none');
  // A switch that does not glide to its place.
  const knob = await win
    .locator('.switch')
    .first()
    .evaluate((e) => getComputedStyle(e, '::after').transitionDuration);
  expect(Number.parseFloat(knob)).toBeLessThan(0.01);
  // Each page of Escouade its icon, « Tous les projets » and « À propos » too, and « Fermer ».
  await expect(win.locator('.s-i svg[data-icon]')).toHaveCount(10);
  await expect(win.getByRole('button', { name: 'Fermer' }).locator('svg[data-icon="close"]')).toBeVisible();
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

/** The blocks of the lot 0 bis (ESC-150) at their address, the app's beside the 2.0 preview's, in both themes, at both sizes. */
test('the private project, the cleaned worktrees and the quotas look as the 2.0 preview draws them', async ({ app }) => {
  test.setTimeout(300_000);
  const exe = chromium.executablePath();
  test.skip(!fs.existsSync(exe) || !fs.existsSync(PREVIEW_20_FONT), 'the preview needs Playwright’s Chromium and the site’s packages');
  const { page } = app;
  await addProject(page, app.repo);
  const dialog = await openPage(page, 'Confidentialité');
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  const browser = await chromium.launch();
  const preview = await browser.newPage();
  const body = page.locator('.settings .s-body');
  const compare = async (pairs: [string, string, string, string[]][], where: string) => {
    for (const [what, sel, psel, skip] of pairs) {
      const a = body.locator(sel).first();
      const p = preview.locator('#settings .s-pane:not([hidden])').locator(psel).first();
      await expect(a, `${what}, ${where}`).toBeVisible();
      expect.soft(await lookBut(a, ...skip), `${what}, ${where}`).toEqual(await lookBut(p, ...skip));
    }
  };
  try {
    for (const t of ['dark', 'light'] as const)
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await preview.setViewportSize(size);
        await theme(page, t);
        await freeze(page);
        const where = `${t} ${size.width}×${size.height}`;
        await openPage(page, 'Confidentialité');
        // #private, #private-accounts: its files harness by harness, its accounts.
        await previewAt(preview, PREVIEW_20, 'private', t, await tint(page));
        await compare(
          [
            ['a title in the group', '.sub-h b', '.sub-h b', ['width']],
            ['what it says', '.sub-h span', '.sub-h span', ['width', 'height']],
            ['a harness of « Ses fichiers »', '.fence li .engine', '.fence b', ['width']],
            ['« Garde-fou »', '.fence .smark:not(.barrier)', '.fence .smark:not(.barrier)', ['width']],
            ['« Barrière »', '.fence .smark.barrier', '.fence .smark.barrier', ['width']],
            ['a shared account', '.acct-note.warn', '.acct-note.warn', ['width', 'height']],
          ],
          `#private, ${where}`,
        );
        await shoot(page, preview, `private${size.name}-${t}`);
        // #worktree-cleanup: what « Nettoyer » removed and kept.
        await openPage(page, 'Git et worktrees');
        await page.locator('.settings').getByRole('button', { name: 'Nettoyer' }).click();
        await previewAt(preview, PREVIEW_20, 'worktree-cleanup', t, await tint(page));
        await compare([['what was cleaned', '.leftovers', '#leftovers', ['width', 'height']]], `#worktree-cleanup, ${where}`);
        // #accounts-quotas: « Changer de compte à » in segments.
        await openPage(page, 'Comptes et harness');
        await previewAt(preview, PREVIEW_20, 'accounts-quotas', t, await tint(page));
        await compare(
          [
            [
              '« Changer de compte à »',
              '.segmented[aria-label="Changer de compte à"]',
              '.seg.inline[aria-label="Changer de compte à"]',
              ['width'],
            ],
            ['its title', '[data-anchor="limit"] h2', '#sg-quotas h2', ['width']],
            // No note under it any more (R4, design/settings-2-2/index.html#a-comptes): each row says what it does.
          ],
          `#accounts-quotas, ${where}`,
        );
        await shoot(page, preview, `accounts-quotas${size.name}-${t}`);
      }
  } finally {
    await browser.close();
  }
});
