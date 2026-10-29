// The settings in the 2.0 design (ESC-152, lot 11b): the window with its tabs in three groups (general, « Par projet »,
// « Système »), the tab's title, the project chosen by pills, the settings in rows under hairlines, the controls in the
// project's colour, and each tab's own blocks (a harness's state, an account's card, the models of the voice, the
// launch commands on a grid, a closed project, the MCP server's activity). Compared to the preview
// (design/app-redesign/index.html, `#settings-<tab>`) in both themes at 1440 × 900 and at CI's 1028 × 779; the English
// labels in their room; nothing that overflows at 1028, 1280 and 1440; the motion stopped by prefers-reduced-motion.
// The captures of the app and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

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
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}${lang ? `&lang=${lang}` : ''}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

/** Both captures, when asked for. */
async function shoot(page: Page, preview: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.screenshot({ path: path.join(SHOTS, `app-${name}.png`) });
  await preview.screenshot({ path: path.join(SHOTS, `preview-${name}.png`) });
}

/** The settings open on `tab` (its name), the pointer and the focus out of the way. */
async function openTab(page: Page, tab: string) {
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  if (!(await dialog.isVisible())) await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await dialog.getByRole('tab', { name: tab }).click();
  await page.mouse.move(2, 400);
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  return dialog;
}

/** The tabs compared, by their name in the app and their route in the preview. */
const TABS = [
  ['Harness', 'settings-claude'],
  ['Application', 'settings-app'],
  ['Comptes', 'settings-accounts'],
  ['Voix', 'settings-voice'],
  ['Notifications', 'settings-notifications'],
  ['Projets', 'settings-projects'],
  ['Kanban', 'settings-board'],
  ['Intégrations', 'settings-integrations'],
  ['Serveur MCP', 'settings-mcp'],
  ['Terminaux', 'settings-terminals'],
  ['Réseau', 'settings-network'],
  ['À propos', 'settings-about'],
] as const;

/**
 * What each tab has of its own, the app's beside the preview's: [what, the app's selector, the preview's, the keys left
 * out (what differs by content)].
 */
const OWN: Record<string, [string, string, string, string[]][]> = {
  Harness: [
    ['a harness’s state', '.hstate', '.hstate', ['width', 'color', 'border']],
    ['a harness’s name', '.hg h2', '.hg h2', ['width']],
    ['« Trouvé : … »', '.lab .found', '.lab .found', ['width', 'height']],
    ['« Claude Code seulement »', '.lab .only', '.lab .only', ['width', 'height']],
  ],
  Comptes: [
    ['an account', '.acct', '.acct', ['width', 'height']],
    ['its sign-in', '.acct .stt', '.acct .stt', ['width', 'color', 'border']],
    ['a key of its card', '.acct .l2 .k', '.acct .l2 .k', ['height']],
    ['« Ajouter un compte… »', '.hg .btn', '.hg .btn', ['width']],
    ['a project chip', '.chip-t', '.chip-t', ['width', 'background', 'color']],
  ],
  Voix: [
    ['a model of Whisper', '.mrow', '.mrow', ['width', 'height']],
    ['its radio', '.mrow .rd:not(:checked)', '.mrow .rd[aria-checked="false"]', []],
    ['its name', '.mrow .mn b', '.mrow .mn b', ['width']],
  ],
  Application: [],
  Notifications: [
    ['a type, ticked', '.check-row', '.check-row', ['width']],
    ['its box', '.check-row .tick-box', '.check-row .cb', []],
  ],
  Projets: [
    ['the project chosen', '.proj-pill[aria-pressed="true"]', '.proj-pill[aria-pressed="true"]', ['width']],
    ['the scope', '.s-scope', '.s-scope', ['height']],
    ['a colour', '.swatch[aria-pressed="false"]', '.swatches button[aria-pressed="false"]', ['background']],
  ],
  Kanban: [
    ['the action chosen', '.w-radio[aria-checked="true"]', '.radio[aria-checked="true"]', ['width', 'height']],
    ['its dot', '.w-radio[aria-checked="true"] .dot', '.radio[aria-checked="true"] .dot', []],
    ['another action', '.w-radio[aria-checked="false"]', '.radio[aria-checked="false"]', ['width', 'height']],
  ],
  Intégrations: [
    ['a service’s letter', '.badge', '.svc .logo-t', ['width']],
    ['« Connecter un hébergeur… »', '.hg .btn:last-child', '#sg-forges .hg .btn', ['width']],
  ],
  'Serveur MCP': [],
  Terminaux: [],
  Réseau: [],
  'À propos': [
    // Its height and its logo's width follow the line of the harnesses found (the preview's, longer, wraps and shrinks it).
    ['who it is', '.about-id', '.about-id', ['width', 'height']],
    ['its logo', '.about-id img', '.about-id svg', ['width']],
    ['its version', '.about-id b', '.about-id b', ['width']],
  ],
};

/** The window, its tabs, its header and its foot, and the first group and row of the tab, beside the preview's. */
async function compareFrame(page: Page, preview: Page, tab: string, where: string) {
  const win = page.locator('.settings[role="dialog"]');
  const pwin = preview.locator('#settings .settings');
  expect.soft(await place(win), `the window's place, ${where}`).toEqual(await place(pwin));
  expect.soft(await look(win), `the window, ${where}`).toEqual(await look(pwin));
  const pairs: [string, Locator, Locator, string[]][] = [
    ['the tabs’ side', win.locator('.s-nav'), pwin.locator('.s-nav'), []],
    ['its title', win.locator('.s-nav-h'), pwin.locator('.s-nav-h'), ['width']],
    ['the tab chosen', win.locator('.s-tab[aria-selected="true"]'), pwin.locator('.s-tab[aria-selected="true"]'), []],
    [
      'another tab',
      win.locator('.s-tab[aria-selected="false"]:not(.changed)').first(),
      pwin.locator('.s-tab[aria-selected="false"]:not(.changed)').first(),
      [],
    ],
    ['a group of tabs', win.locator('.s-group').first(), pwin.locator('.s-group').first(), ['width']],
    ['the version', win.locator('.s-ver'), pwin.locator('.s-ver'), ['width']],
    ['the tab’s title', win.locator('.s-head h1'), pwin.locator('.s-head h1'), ['width']],
    ['what it sets', win.locator('.s-head p'), pwin.locator('.s-head p'), ['width']],
    ['« Fermer »', win.locator('.s-head .close'), pwin.locator('.s-head .icon-btn'), []],
    ['the body', win.locator('.s-body'), pwin.locator('.s-body'), ['height']],
    ['the foot', win.locator('.s-foot'), pwin.locator('.s-foot'), []],
    ['« Annuler »', win.locator('.s-foot .btn').first(), pwin.locator('.s-foot .btn').first(), []],
    ['« Enregistrer »', win.locator('.s-foot .btn.primary'), pwin.locator('.s-foot .btn.primary'), []],
  ];
  for (const [what, a, p, skip] of pairs) {
    expect.soft(await lookBut(a, ...skip), `${what}, ${where}`).toEqual(await lookBut(p, ...skip));
    if (!skip.length) expect.soft(await place(a), `${what}'s place, ${where}`).toEqual(await place(p));
  }
  // The settings: a group's title, a row, its label and what it does, a switch, a field, segments, wherever both have one.
  const body = win.locator('.s-body');
  const pbody = pwin.locator('.s-pane:not([hidden])');
  const common: [string, string, string, string[]][] = [
    ['a group’s title', '.sg h2', '.sg h2', ['width']],
    ['a row', '.sr', '.sr', ['width', 'height']],
    ['a row’s label', '.sr .lab > b', '.sr .lab > b', ['width', 'height']],
    ['what it does', '.sr .lab > .d', '.sr .lab > span:not(.found):not(.only):not(.warn):not(.smark)', ['width', 'height']],
    ['a group’s note', '.sg > .note', '.sg > .note', ['width', 'height']],
    ['a switch', '.ctl .switch[aria-checked="false"]', '.ctl .tgl[aria-checked="false"]', []],
    ['a switch on', '.ctl .switch[aria-checked="true"]', '.ctl .tgl[aria-checked="true"]', ['background']],
    ['a field', '.ctl input.field:not(.num)', '.ctl input.inp:not(.w-num)', ['width']],
    ['segments', '.ctl .segmented', '.ctl .seg.inline', ['width']],
    ['the segment chosen', '.ctl .segmented [aria-pressed="true"]', '.ctl .seg.inline [aria-selected="true"]', ['width']],
    ...OWN[tab],
  ];
  for (const [what, sel, psel, skip] of common) {
    const a = body.locator(sel).first();
    const p = pbody.locator(psel).first();
    if (!(await a.count()) || !(await p.count()) || !(await a.isVisible()) || !(await p.isVisible())) continue;
    expect.soft(await lookBut(a, ...skip), `${what}, ${where}`).toEqual(await lookBut(p, ...skip));
  }
}

test('the settings look as the preview draws them, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(400_000);
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
        await theme(page, t);
        await still(page);
        await page.evaluate(() => document.fonts.ready);
        for (const [tab, route] of TABS) {
          await openTab(page, tab);
          await previewAt(preview, route, t);
          await shoot(page, preview, `${route}${size.name}-${t}`);
          await compareFrame(page, preview, tab, `${tab}, ${t} ${size.width}×${size.height}`);
        }
      }
  } finally {
    await browser.close();
  }
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
        if (tops.size > 1 && (s.whiteSpace === 'nowrap' || el.closest('.s-tab, .btn, .s-head h1, .hg h2, .proj-pill, .segmented')))
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

/** Every tab, read to its end, at 1028, 1280 and 1440 px: what overflows or overlaps. */
async function measure(page: Page, tabs: readonly string[], lang: string) {
  const win = page.locator('.settings[role="dialog"]');
  const problems: string[] = [];
  for (const size of [
    { width: 1028, height: 779 },
    { width: 1280, height: 800 },
    { width: 1440, height: 900 },
  ]) {
    await page.setViewportSize(size);
    for (const tab of tabs) {
      await win.getByRole('tab', { name: tab, exact: true }).click();
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
      for (const p of await overflows(win, [
        '.sr',
        '.l1',
        '.l2',
        '.cmd-row',
        '.step',
        '.s-head',
        '.s-scope',
        '.s-foot',
        '.hg',
        '.mrow',
        '.item',
      ]))
        problems.push(`${lang} ${size.width}: ${tab}: ${p}`);
    }
  }
  return problems;
}

test('nothing of the settings overflows nor overlaps, at 1028, 1280 and 1440 px, in French', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await addProject(page, app.repo);
  await openTab(page, 'Harness');
  // A private project shows its files and its accounts.
  await openTab(page, 'Projets');
  await page.getByRole('dialog', { name: 'Réglages' }).getByRole('switch', { name: 'Projet privé' }).click();
  expect(
    await measure(
      page,
      TABS.map(([tab]) => tab),
      'fr',
    ),
  ).toEqual([]);
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the labels of the settings fit in their place, at 1028, 1280 and 1440 px', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    await addProject(page, app.repo, { lang: 'en' });
    await page.getByRole('button', { name: 'Settings (Ctrl+,)' }).click();
    const win = page.getByRole('dialog', { name: 'Settings' });
    await expect(win.locator('.s-group')).toHaveText(['Per project', 'System']);
    await win.getByRole('tab', { name: 'Projects' }).click();
    await win.getByRole('switch', { name: 'Private project' }).click();
    const tabs = [
      'Application',
      'Harness',
      'Accounts',
      'Voice',
      'Notifications',
      'Projects',
      'Kanban',
      'Integrations',
      'MCP server',
      'Terminals',
      'Network',
      'About',
    ];
    expect(await measure(page, tabs, 'en')).toEqual([]);
    if (SHOTS)
      for (const tab of ['Harness', 'Accounts', 'Voice', 'Projects']) {
        await page.setViewportSize({ width: 1028, height: 779 });
        await win.getByRole('tab', { name: tab, exact: true }).click();
        await page.screenshot({ path: path.join(SHOTS, `app-settings-${tab.toLowerCase()}-small-en.png`) });
      }
  });
});

test('the settings open still with prefers-reduced-motion, their icons drawn', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const motion = (el: Locator) => el.evaluate((e) => getComputedStyle(e).animationName);
  await openTab(page, 'Harness');
  const win = page.locator('.settings[role="dialog"]');
  expect(await motion(win)).toMatch(/rise/);
  await page.keyboard.press('Escape');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openTab(page, 'Harness');
  expect(await motion(win)).toBe('none');
  expect(await motion(page.locator('.overlay:has(.settings)'))).toBe('none');
  // A switch that does not glide to its place.
  const knob = await win
    .locator('.switch')
    .first()
    .evaluate((e) => getComputedStyle(e, '::after').transitionDuration);
  expect(Number.parseFloat(knob)).toBeLessThan(0.01);
  // Each tab its icon, and « Fermer » too.
  await expect(win.locator('.s-tab svg[data-icon]')).toHaveCount(13);
  await expect(win.getByRole('button', { name: 'Fermer' }).locator('svg[data-icon="close"]')).toBeVisible();
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

/** The blocks of the lot 0 bis (ESC-150) at their address, the app's beside the preview's, in both themes, at both sizes. */
test('the private project, the voice on line, the cleaned worktrees and the quotas look as the preview draws them', async ({ app }) => {
  test.setTimeout(300_000);
  const exe = chromium.executablePath();
  test.skip(!fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT), 'the preview needs Playwright’s Chromium and the site’s packages');
  const { page } = app;
  await addProject(page, app.repo);
  const dialog = await openTab(page, 'Projets');
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
        await still(page);
        const where = `${t} ${size.width}×${size.height}`;
        await openTab(page, 'Projets');
        // #private, #private-accounts: its files harness by harness, its accounts.
        await previewAt(preview, 'private', t);
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
        await page.locator('.settings').getByRole('button', { name: 'Nettoyer' }).click();
        await previewAt(preview, 'worktree-cleanup', t);
        await compare([['what was cleaned', '.leftovers', '#leftovers', ['width', 'height']]], `#worktree-cleanup, ${where}`);
        // #accounts-quotas: « Changer de compte à » in segments.
        await openTab(page, 'Comptes');
        await previewAt(preview, 'accounts-quotas', t);
        await compare(
          [
            [
              '« Changer de compte à »',
              '.segmented[aria-label="Changer de compte à"]',
              '.seg.inline[aria-label="Changer de compte à"]',
              ['width'],
            ],
            ['its title', '[data-anchor="limit"] h2', '#sg-quotas h2', ['width']],
            ['its note', '[data-anchor="limit"] .note', '#sg-quotas .note', ['width', 'height']],
          ],
          `#accounts-quotas, ${where}`,
        );
        await shoot(page, preview, `accounts-quotas${size.name}-${t}`);
      }
  } finally {
    await browser.close();
  }
});
