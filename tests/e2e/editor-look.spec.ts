// The project's editor is the VS Code view (ESC-447, tests/e2e/vscode-view.spec.ts), checked against the editor of the
// 2.0 design (ESC-81, lot 9: design/app-redesign/index.html, `#editor`) where the view draws what it drew: since A7
// (ESC-476), the marks of the changes from main between the numbers and the text, the letters of the files changed and
// their folders marked, « 5 lignes modifiées par rapport à main » in the view's bar, on the agent's worktree picked as
// the source (tests/e2e/vscode-changes.spec.ts: the comparisons, the sources, the base that moves); since A4
// (ESC-472), its theme followed at once, its fonts, its language; since A3 (ESC-464), its keys and Ctrl+P. Its tree,
// tabs, search and status line are VS Code's own, not the preview's. In both themes at 1440 × 900 and at CI's
// 1028 × 779; the English labels in their room; the motion stopped by prefers-reduced-motion. The captures of the app
// and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';
import { editorLines, explorerRow, openInExplorer, unfold, vscodeTabs, workbench } from './vscode';

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

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', ...args], { cwd, encoding: 'utf8' }).trim();

const USERS_BEFORE = [
  "import { Router } from 'express';",
  "import { db } from '../db';",
  '',
  'export const router = Router();',
  '',
  "router.get('/users', async (req) => {",
  '  const users = await db.users.all();',
  '  return users;',
  '});',
  '',
  "router.get('/users/:id', async (req) => db.users.find(req.params.id));",
  '',
].join('\n');
const USERS_AFTER = [
  "import { Router } from 'express';",
  "import { db } from '../db';",
  "import { paginate } from '../db/paginate';",
  '',
  'export const router = Router();',
  '',
  '// La liste des utilisateurs, page par page.',
  "router.get('/users', async (req) => {",
  '  const { limit = 50, cursor } = req.query;',
  '  const page = await paginate(db.users, { limit, cursor });',
  '  return { users: page.items, next: page.next };',
  '});',
  '',
  "router.get('/users/:id', async (req) => db.users.find(req.params.id));",
  '',
].join('\n');

/** The preview's project, committed before it is added: the folders and files its tree shows. */
function previewsProject(repo: string) {
  const write = (root: string, file: string, text: string) => {
    fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    fs.writeFileSync(path.join(root, file), text);
  };
  write(repo, 'src/auth/login.ts', 'export const login = () => null;\n');
  write(repo, 'src/db/index.ts', 'export const db = { users: [] };\n');
  write(repo, 'src/routes/invoices.ts', 'export const invoices = [];\n');
  write(repo, 'src/routes/users.ts', USERS_BEFORE);
  write(repo, 'tests/users.test.ts', "import { it } from 'vitest';\n");
  write(repo, 'package.json', '{ "name": "demo-api" }\n');
  write(repo, 'README.md', '# demo-api\n');
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', 'The project of the preview');
  return write;
}

/** The agent's work in its worktree, as the preview's: `users.ts` and its test modified, `paginate.ts` added. */
async function agentAtWork(repo: string, write: ReturnType<typeof previewsProject>) {
  const worktrees = path.join(repo, '.claude', 'worktrees');
  // The first agent shows once its worktree is made: on a loaded machine, that takes longer than an expect's 15 s.
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0), { timeout: 60_000 }).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  await expect.poll(() => fs.existsSync(path.join(wt, 'src', 'routes', 'users.ts')), { timeout: 30_000 }).toBe(true);
  write(wt, 'src/routes/users.ts', USERS_AFTER);
  write(wt, 'src/db/paginate.ts', 'export async function paginate() {}\n');
  write(wt, 'tests/users.test.ts', "import { it } from 'vitest';\nit('pages', () => {});\n");
  return wt;
}

/** The view's words, in each language. */
const VIEW_WORDS = {
  fr: { editor: 'Éditeur ou workspace du projet', editorItem: 'Éditeur', five: '5 lignes modifiées par rapport à main' },
  en: { editor: 'Editor or workspace of the project', editorItem: 'Editor', five: '5 lines changed from main' },
};

/**
 * Opens the VS Code view on the agent's worktree (ESC-476, A7), the project's folder folded and the preview's folders
 * of the worktree open, the preview's three files in tabs with `users.ts` shown, its marks drawn and counted.
 */
async function editorAsThePreview(page: Page, lang: 'fr' | 'en' = 'fr') {
  const words = VIEW_WORDS[lang];
  await page.getByRole('button', { name: words.editor }).click();
  await page.getByRole('menuitemradio', { name: words.editorItem }).click();
  const view = page.getByTestId('vscode-view');
  const wb = workbench(page);
  await expect(wb).toBeVisible({ timeout: 60_000 });
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').nth(1).click();
  const explorer = wb.locator('.explorer-folders-view');
  const roots = explorer.locator('.monaco-list-row[aria-level="1"]');
  await expect(roots).toHaveCount(2, { timeout: 30_000 });
  await unfold(roots.nth(0), false);
  await unfold(roots.nth(1), true);
  for (const name of ['src', 'db', 'routes', 'tests']) await unfold(explorerRow(page, name));
  for (const name of ['users.ts', 'paginate.ts', 'users.test.ts']) {
    await explorerRow(page, name).click();
    await expect.poll(() => vscodeTabs(page)).toContain(name);
  }
  await explorerRow(page, 'users.ts').click();
  await expect(editorLines(page)).toContainText('paginate(db.users');
  // The keyboard in the text, as the preview draws it: the row of the file shown is not the explorer's focused one.
  await editorLines(page).click();
  await expect(view.getByTestId('vscode-changes')).toHaveText(words.five, { timeout: 30_000 });
  await expect.poll(() => quickDiffMarks(page), { timeout: 30_000 }).toEqual(['added', 'added', 'modified', 'modified', 'modified']);
  return view;
}

/** The marks of VS Code's quick diff in the editor's margin, by kind, one per line, top to bottom. */
const quickDiffMarks = (page: Page) =>
  workbench(page)
    .locator('.editor-instance .monaco-editor .margin-view-overlays .dirty-diff-glyph')
    .evaluateAll((els) =>
      els
        .map((e) => ({ top: e.getBoundingClientRect().top, kind: /dirty-diff-(added|modified|deleted)/.exec(e.className)?.[1] ?? '' }))
        .sort((a, b) => a.top - b.top)
        .map((m) => m.kind),
    );

/** The letter VS Code draws after a row of its explorer (a badge, in a `::after`), its colour, and the colour of its name. */
const explorerLetter = (row: Locator) =>
  row.evaluate((el) => {
    let letter: string | null = null;
    let color = '';
    for (const n of [el, ...el.querySelectorAll('*')]) {
      const s = getComputedStyle(n, '::after');
      const m = /^"(.+)"$/.exec(s.content);
      if (m && m[1].trim()) {
        letter = m[1].trim();
        color = s.color;
      }
    }
    return { letter, color, name: getComputedStyle(el.querySelector('.label-name') ?? el).color };
  });

/** The preview at `#route`, in theme `t`: from a blank page, for its script to open the route. */
async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

// A7 (ESC-476): the marks of the changes from main in the VS Code view, as the preview draws those of its editor: the
// count of the lines changed in the view's bar (the preview's « 5 lignes modifiées par rapport à main »), the marks of
// the lines added and changed between the numbers and the text, the letters of the files and the mark of their folders.
// The tree, the tabs, the search and the status line are VS Code's own (their theme and fonts are A4's, below).
test('the marks of the changes in the view look as the preview draws them, in both themes, at both sizes', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  const write = previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo, write);
  const view = await editorAsThePreview(page);
  const wb = workbench(page);
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
        await previewAt(preview, 'editor', t);
        const where = `${t} ${size.width}×${size.height}`;
        const pv = preview.locator('#view-editor');

        // « 5 lignes modifiées par rapport à main »: its text, its colour, its font, « 5 lignes » in green.
        const vs = view.getByTestId('vscode-changes');
        const pvs = pv.locator('.ed-crumbs .vs');
        await expect(vs).toHaveText((await pvs.textContent())!.trim());
        const text = (l: Locator) => lookBut(l, 'width', 'height', 'padding', 'border', 'background', 'shadow', 'radius');
        expect.soft(await text(vs), `the count, ${where}`).toLookLike(await text(pvs));
        expect.soft(await text(vs.locator('.add')), `« 5 lignes », ${where}`).toLookLike(await text(pvs.locator('.add')));
        await expect(view.getByTestId('vscode-compare')).toBeVisible();

        // The marks: the lines added and changed, in the preview's colours.
        await expect
          .poll(() => quickDiffMarks(page), { message: `the marks, ${where}` })
          .toEqual(['added', 'added', 'modified', 'modified', 'modified']);
        const glyph = (kind: string) =>
          wb.locator(`.editor-instance .monaco-editor .margin-view-overlays .dirty-diff-glyph.dirty-diff-${kind}`).first();
        const markColor = (l: Locator) => l.evaluate((n) => getComputedStyle(n).borderLeftColor);
        const pmark = (cls: string) =>
          pv
            .locator(`.code-ed .l.${cls} .gm`)
            .first()
            .evaluate((n) => getComputedStyle(n).backgroundColor);
        const added = await pmark('ad');
        const changed = await pmark('md');
        await expect.poll(() => markColor(glyph('added')), { message: `an added line's mark, ${where}` }).toLookLike(added);
        await expect.poll(() => markColor(glyph('modified')), { message: `a changed line's mark, ${where}` }).toLookLike(changed);

        // The letters of the files changed, and their folders marked, in the preview's colours.
        for (const [name, cls, letter] of [
          ['users.ts', 'm', 'M'],
          ['paginate.ts', 'a', 'A'],
          ['users.test.ts', 'm', 'M'],
        ] as const) {
          const got = await explorerLetter(explorerRow(page, name));
          expect.soft(got.letter, `${name}'s letter, ${where}`).toBe(letter);
          const want = await pv
            .locator(`.node .letter.${cls}`)
            .first()
            .evaluate((n) => getComputedStyle(n).color);
          expect.soft(got.color, `${name}'s letter's colour, ${where}`).toLookLike(want);
          expect.soft(got.name, `${name}'s name in its colour, ${where}`).toLookLike(want);
        }
        const dot = await pv
          .locator('.node .mdot')
          .first()
          .evaluate((n) => getComputedStyle(n).backgroundColor);
        for (const name of ['src', 'routes']) {
          const got = await explorerLetter(explorerRow(page, name));
          expect.soft(got.letter, `${name} marked, ${where}`).not.toBeNull();
          expect.soft(got.color, `${name}'s mark in the colour of the preview's dot, ${where}`).toLookLike(dot);
        }

        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          const n = `editor${size.name}${t === 'light' ? '-light' : ''}`;
          await page.screenshot({ path: path.join(SHOTS, `${n}.app.png`) });
          await preview.screenshot({ path: path.join(SHOTS, `${n}.preview.png`) });
        }
      }
  } finally {
    await browser.close();
  }
});

// A4 (ESC-472): the VS Code view takes Escouade's theme at once, from the tokens of app.css (the window's theme, the
// project's tint, a plugin's), its fonts and its size, and its language; never loaded again for its colours, its tabs
// kept. The colours are compared as the browser computes them (`toLookLike`, srgb.ts).

/** The colour a token of the app has now, in the root (where the project's tint is). */
const tokenOf = (page: Page, name: string) =>
  page.evaluate((name) => {
    const probe = document.createElement('span');
    probe.style.color = `var(${name})`;
    document.documentElement.append(probe);
    const c = getComputedStyle(probe).color;
    probe.remove();
    return c;
  }, name);
const bgOf = (el: Locator) => el.evaluate((n) => getComputedStyle(n).backgroundColor);
const colorOf = (el: Locator) => el.evaluate((n) => getComputedStyle(n).color);

/** The parts of the workbench the theme is checked on. */
function parts(page: Page) {
  const wb = workbench(page);
  const word = (text: string) =>
    editorLines(page)
      .locator('span span')
      .filter({ hasText: new RegExp(`^${text}$`) })
      .first();
  return {
    editor: wb.locator('.editor-instance .monaco-editor-background').first(),
    side: wb.locator('.part.sidebar').first(),
    tabs: wb.locator('.editor-group-container > .title').first(),
    tab: wb.locator('.tabs-container .tab.active').first(),
    status: wb.locator('.part.statusbar').first(),
    find: wb.locator('.editor-instance .find-widget').first(),
    keyword: word('import'),
    text: word("'express'"),
  };
}

/** The view, in the colours of the tokens now: its editor, its tree, its tabs, its bars, its syntax, its search. */
async function followsTheTokens(page: Page, where: string) {
  const p = parts(page);
  const want: [Locator, 'bg' | 'color', string, string][] = [
    [p.editor, 'bg', '--panel', 'the editor'],
    [p.side, 'bg', '--frame', 'the tree'],
    [p.tabs, 'bg', '--frame', 'the tabs’ row'],
    [p.tab, 'bg', '--panel', 'the tab shown'],
    [p.status, 'bg', '--frame', 'the status bar'],
    [p.find, 'bg', '--raise', 'the search'],
    [p.keyword, 'color', '--syn-k', 'a keyword'],
    [p.text, 'color', '--syn-s', 'a string'],
  ];
  for (const [el, kind, token, label] of want)
    await expect
      .poll(() => (kind === 'bg' ? bgOf(el) : colorOf(el)), { message: `${label}, ${where}`, timeout: 10_000 })
      .toLookLike(await tokenOf(page, token));
}

/** The view on the preview's project, users.ts shown after index.ts, its search open. */
async function vscodeOnThePreview(page: Page) {
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  await openInExplorer(page, 'src/db/index.ts');
  await openInExplorer(page, 'src/routes/users.ts');
  await expect(editorLines(page)).toContainText("import { Router } from 'express';");
  await editorLines(page).click();
  await page.keyboard.press('Control+F');
  await expect(parts(page).find).toBeVisible();
  await page.keyboard.type('users');
}

test('the VS Code view follows the theme at once, its fonts the app’s, its tabs kept, at both sizes', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  const { page } = app;
  previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await vscodeOnThePreview(page);
  const wb = workbench(page);
  const loads: string[] = [];
  page.on('framenavigated', (f) => {
    if (f === page.mainFrame()) loads.push(f.url());
  });
  for (const t of ['dark', 'light'] as const)
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await theme(page, t);
      const where = `${t} ${size.width}×${size.height}`;
      await followsTheTokens(page, where);
      // The fonts: the app's for its interface, its code font for the text, ligatures as elsewhere.
      const font = await editorLines(page).evaluate((n) => {
        const c = getComputedStyle(n);
        return [c.fontFamily, c.fontFeatureSettings];
      });
      expect(font[0], `the code's font, ${where}`).toMatch(/^"?JetBrains Mono/);
      expect(font[1], `its ligatures, ${where}`).toMatch(/"calt"/);
      expect(font[1], `its ligatures, ${where}`).not.toMatch(/off/);
      const ui = await wb
        .locator('.part.sidebar')
        .first()
        .evaluate((n) => getComputedStyle(n).fontFamily);
      expect(ui, `the tree's font, ${where}`).toMatch(/^"?Inter Variable/);
      expect(await vscodeTabs(page), `its tabs, ${where}`).toEqual(['index.ts', 'users.ts']);
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `vscode${size.name}${t === 'light' ? '-light' : ''}.png`) });
      }
    }
  // Never loaded again, the search still open.
  expect(loads).toEqual([]);
  await expect(parts(page).find).toBeVisible();
});

test('the VS Code view takes the tint of the project shown, and a plugin’s theme, at once', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  const { page } = app;
  previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  // A second project, of the next colour.
  const other = path.join(app.root, 'studio-web');
  fs.mkdirSync(other);
  git(other, 'init', '-q', '-b', 'main');
  fs.writeFileSync(path.join(other, 'README.md'), '# studio-web\n');
  git(other, 'add', '-A');
  git(other, 'commit', '-qm', 'init');
  await addProject(page, other, { firstAgent: false });
  const tile = (name: string) => page.getByRole('tab', { name: new RegExp(`^${name}`) });
  await tile('demo-api').click();
  await expect(tile('demo-api')).toHaveAttribute('aria-selected', 'true');
  await vscodeOnThePreview(page);
  const side = parts(page).side;
  await followsTheTokens(page, 'demo-api');
  const first = await bgOf(side);
  // The other project, its editor: the view in its tint once `--p` has glided, the first project's tabs kept.
  await tile('studio-web').click();
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  await expect(workbench(page)).toBeVisible();
  await expect.poll(() => bgOf(side), { timeout: 10_000 }).not.toLookLike(first);
  await followsTheTokens(page, 'studio-web');
  await tile('demo-api').click();
  await expect.poll(() => bgOf(side), { timeout: 10_000 }).toLookLike(first);
  expect(await vscodeTabs(page)).toEqual(['index.ts', 'users.ts']);

  // A plugin's theme, as « Couleurs » lays it (lib/plugin-theme.svelte.ts): its sheet over app.css, its attribute.
  await page.evaluate(() => {
    const sheet = document.createElement('style');
    sheet.id = 'escouade-plugin-theme';
    sheet.textContent =
      ":root:root:not([data-theme='light']) { --frame: #000000; --panel: #050505; --raise: #141414; --syn-k: #ffd400; --syn-s: #4dff88; }" +
      ":root:root[data-theme='light'] { --frame: #ffffff; --panel: #ffffff; --raise: #f2f2f2; --syn-k: #0040c0; --syn-s: #006b2e; }";
    document.head.append(sheet);
    document.documentElement.dataset.pluginTheme = 'dev.escouade.high-contrast/high-contrast';
  });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await expect.poll(() => bgOf(parts(page).editor), { timeout: 10_000 }).toLookLike(t === 'dark' ? 'rgb(5, 5, 5)' : 'rgb(255, 255, 255)');
    await followsTheTokens(page, `plugin, ${t}`);
  }
  // Escouade's colours back.
  await page.evaluate(() => {
    document.getElementById('escouade-plugin-theme')?.remove();
    delete document.documentElement.dataset.pluginTheme;
  });
  await followsTheTokens(page, 'Escouade, light');
});

test('the VS Code view takes « Taille du texte de l’éditeur » at once', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(240_000);
  const { page } = app;
  previewsProject(app.repo);
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  await openInExplorer(page, 'src/routes/users.ts');
  const size = () => editorLines(page).evaluate((n) => getComputedStyle(n).fontSize);
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  await dialog
    .locator('#settings-panel')
    .getByRole('button', { name: /^Avancé/ })
    .click();
  const field = dialog.getByRole('spinbutton', { name: 'Taille du texte de l’éditeur' });
  await expect.poll(size).toBe(`${await field.inputValue()}px`);
  await field.fill('17');
  await field.press('Tab');
  await expect.poll(size).toBe('17px');
  await closeSettings(dialog);
  expect(await vscodeTabs(page)).toEqual(['users.ts']);
});

test('the VS Code view speaks Escouade’s language, and takes a new one when the window reloads, nothing lost', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  const { page } = app;
  previewsProject(app.repo);
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  await openInExplorer(page, 'src/db/index.ts');
  await openInExplorer(page, 'src/routes/users.ts');
  const title = () => workbench(page).locator('.part.sidebar .title-label').first();
  // In French, as the app.
  await expect(title()).toHaveText(/^explorateur$/i);
  // A change not saved.
  await editorLines(page).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('// pas encore enregistré');
  await expect.poll(() => vscodeTabs(page)).toContain('users.ts*');

  // English: the app at once; the view says it changes when the window reloads.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const reglages = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(reglages, 'Général');
  await reglages.getByRole('group', { name: 'Langue de l’interface' }).getByRole('button', { name: 'English', exact: true }).click();
  await closeSettings(page.getByRole('dialog', { name: 'Settings' }));
  const view = page.getByTestId('vscode-view');
  await expect(view.getByTestId('vscode-lang-note')).toHaveText('VS Code changes language when the window reloads.');
  await expect(title()).toHaveText(/^explorateur$/i);
  // « Reload »: the file modified is asked about first; « Cancel » keeps everything.
  await view.getByTestId('vscode-reload').click();
  const ask = page.getByRole('dialog', { name: 'Save the changes to “users.ts”?' });
  await expect(ask).toBeVisible();
  await ask.getByRole('button', { name: 'Cancel' }).click();
  await expect.poll(() => vscodeTabs(page)).toContain('users.ts*');
  // « Save »: written, then the window reloads, the view in English, its tabs open again, the one shown last.
  await view.getByTestId('vscode-reload').click();
  await ask.getByRole('button', { name: 'Save', exact: true }).click();
  await expect.poll(() => fs.readFileSync(path.join(app.repo, 'src/routes/users.ts'), 'utf8')).toContain('// pas encore enregistré');
  await expect(title()).toHaveText(/^explorer$/i, { timeout: 60_000 });
  await expect(page.getByTestId('vscode-lang-note')).toHaveCount(0);
  await expect.poll(() => vscodeTabs(page), { timeout: 30_000 }).toEqual(['index.ts', 'users.ts']);
  await expect(workbench(page).locator('.tabs-container .tab.active .label-name')).toHaveText('users.ts');
  await expect(editorLines(page)).toContainText('// pas encore enregistré');
});

// The VS Code view (ESC-464, A3): its caret, the menu of its source on the worktree, Ctrl+P pressed in it.
test('the editor stays still with prefers-reduced-motion', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(240_000);
  const { page } = app;
  const write = previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo, write);
  // Before the view's first opening: its workbench reads it as it starts.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  const view = page.getByTestId('vscode-view');
  await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
  const motion = (el: Locator) => el.evaluate((e) => getComputedStyle(e).animationName);
  // The menu of the source, the worktree picked in it.
  await view.locator('.vscode-bar .src').click();
  const menu = page.getByRole('menu', { name: 'Parcourir' });
  await expect(menu).toBeVisible();
  expect(await motion(menu)).toBe('none');
  await page.getByRole('menuitemradio').nth(1).click();
  await expect(menu).toBeHidden();
  // « Ouvrir un fichier », from the view: still, its keys and its close button drawn, no character as an icon.
  await workbench(page).click({ position: { x: 300, y: 200 } });
  await page.keyboard.press('Control+P');
  const palette = page.getByRole('dialog', { name: 'Ouvrir un fichier' });
  await expect(palette).toBeVisible();
  expect(await motion(palette)).toBe('none');
  expect(await motion(page.locator('.overlay'))).toBe('none');
  await expect(palette.getByRole('button', { name: 'Fermer' }).locator('svg[data-icon="close"]')).toBeVisible();
  await expect(palette.locator('.keys svg[data-icon="up"]')).toBeVisible();
  // On the worktree: its users.ts, the agent's.
  await page.keyboard.type('users.ts');
  await palette
    .getByRole('option', { name: /users\.ts/ })
    .first()
    .click();
  await expect(palette).toBeHidden();
  await expect(editorLines(page)).toContainText('paginate(db.users');
  // The caret does not blink.
  await expect(workbench(page).locator('.editor-instance .cursors-layer').first()).toHaveClass(/cursor-solid/);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
});

test('the search through the conversations draws its icons, not characters', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.keyboard.press('Control+K');
  const palette = page.getByRole('dialog', { name: 'Rechercher dans les conversations' });
  await expect(palette).toBeVisible();
  await expect(palette.locator('.search svg[data-icon="search"]')).toBeVisible();
  await expect(palette.getByRole('button', { name: 'Fermer' }).locator('svg[data-icon="close"]')).toBeVisible();
  expect(await palette.textContent()).not.toMatch(/[⌕×↑↓]/);
  await page.keyboard.press('Escape');
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  // A7 (ESC-476): the view's bar on the agent's worktree, its count of the lines changed and « Compare », in English;
  // VS Code's commands of the comparison too.
  test('the labels of the view’s bar fit in their place, at both sizes', async ({ app }) => {
    test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
    test.setTimeout(240_000);
    const { page } = app;
    const write = previewsProject(app.repo);
    await addProject(page, app.repo, { worktrees: true, lang: 'en' });
    await agentAtWork(app.repo, write);
    const view = await editorAsThePreview(page, 'en');
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await page.mouse.move(size.width / 2, 20);
      // What is cut short, or overflows its box: none of it, but the name of the source, cut by design.
      const cut = await view
        .locator('.vscode-bar')
        .evaluate((root) =>
          [...root.querySelectorAll<HTMLElement>('*')]
            .filter(
              (el) =>
                el.offsetParent &&
                el.children.length === 0 &&
                !el.closest('.src .nm, .src') &&
                (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
            )
            .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
        );
      expect(cut, `${size.width}×${size.height}`).toEqual([]);
      await expect(view.getByRole('button', { name: 'Conversation' })).toBeVisible();
      await expect(view.getByTestId('vscode-changes')).toHaveText('5 lines changed from main');
      await expect(view.getByTestId('vscode-compare')).toHaveText('Compare');
      // The bar holds on one line.
      expect((await view.locator('.vscode-bar').boundingBox())!.height).toBeLessThanOrEqual(37);
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `editor${size.name}-en.app.png`) });
      }
    }
    // VS Code's menu of the tab, in English: « Compare with Base », « Compare with Saved » once modified.
    const wb = workbench(page);
    await editorLines(page).click();
    await page.keyboard.type('x');
    await wb.locator('.tabs-container .tab', { hasText: /^users\.ts/ }).click({ button: 'right' });
    await expect(wb.locator('.context-view .action-item', { hasText: 'Compare with Base' })).toBeVisible();
    await expect(wb.locator('.context-view .action-item', { hasText: 'Compare with Saved' })).toBeVisible();
    await page.keyboard.press('Escape');
  });
});

/**
 * What sticks out of its box in `root`: the leaves wider (or higher) than their box, and the boxes given, but the names
 * of the user's files, folders, branches and agents, and what they wrote, cut short by design.
 */
const sticksOut = (root: Locator, boxes: string[]) =>
  root.evaluate((el, boxes) => {
    const cutByDesign = '.caption, .src .nm, .path, .source, .name, .dir, .snip, .where, .who, .status[role="status"]';
    const over = (e: Element) => e.scrollWidth > e.clientWidth + 1 || e.scrollHeight > e.clientHeight + 1;
    const leaves = [...el.querySelectorAll<HTMLElement>('*')].filter(
      (e) => e.offsetParent && e.children.length === 0 && !e.closest(cutByDesign) && !e.closest('.cm-scroller, .splitter') && over(e),
    );
    const wide = boxes.flatMap((s) => [...el.querySelectorAll(s)]).filter((e) => e.scrollWidth > e.clientWidth + 1);
    return [...leaves, ...wide].map((e) => `${e.tagName.toLowerCase()}.${e.className} « ${e.textContent?.trim().slice(0, 40)} »`);
  }, boxes);

for (const lang of ['fr', 'en'] as const)
  test.describe(`its room, ${lang}`, () => {
    test.use({ language: lang });

    // The view and the two palettes (Ctrl K, Ctrl P) at 1 028, 1 280 and 1 440 px, the side panel closed, open alone
    // and side by side with the conversation: nothing sticks out, as the folds go by the room of each (@container).
    // In the VS Code view (ESC-464, A3), Ctrl+P is pressed in it and opens on the core's index; its search (Ctrl+F) is
    // VS Code's, within the view. The marks of the preview's worktree are A7's (below).
    test('nothing of the editor or of its searches sticks out, at 1028, 1280 and 1440 px, whatever the side panel', async ({ app }) => {
      test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
      test.setTimeout(400_000);
      const { page } = app;
      const write = previewsProject(app.repo);
      await addProject(page, app.repo, { worktrees: true, lang });
      await agentAtWork(app.repo, write);
      const words =
        lang === 'fr'
          ? {
              conv: 'Rechercher dans les conversations',
              open: 'Ouvrir un fichier',
              editor: 'Éditeur ou workspace du projet',
              editorItem: 'Éditeur',
            }
          : { conv: 'Search conversations', open: 'Open a file', editor: 'Editor or workspace of the project', editorItem: 'Editor' };
      const view = page.getByTestId('vscode-view');
      const back = () => page.getByTestId('vscode-back').click();
      const sizes = [
        { width: 1028, height: 779 },
        { width: 1280, height: 800 },
        { width: 1440, height: 900 },
      ];
      /** The palette `name`, opened by `key`: nothing of it sticks out; closed again. */
      const palette = async (key: string, name: string, where: string) => {
        await page.keyboard.press(key);
        const box = page.getByRole('dialog', { name });
        await expect(box).toBeVisible();
        expect(await sticksOut(box, ['.search', '.scope', '.foot', '.keys']), `${name}, ${where}`).toEqual([]);
        return box;
      };
      /** `inner` within `outer`, to a pixel. */
      const within = async (inner: Locator, outer: Locator) => {
        const [ix, iy, iw, ih] = await place(inner);
        const [ox, oy, ow, oh] = await place(outer);
        return ix >= ox - 1 && iy >= oy - 1 && ix + iw <= ox + ow + 1 && iy + ih <= oy + oh + 1;
      };
      for (const panel of ['closed', 'alone', 'side by side'] as const) {
        // The panel set at the widest, where its buttons all show.
        await page.setViewportSize({ width: 1440, height: 900 });
        if (await view.isVisible()) await back();
        await expect(page.locator('main.conv')).toBeVisible();
        // By their shortcuts: the header folds its buttons into « … » once the panel takes room (ESC-166).
        if (panel === 'alone') await page.keyboard.press('Control+Shift+B');
        if (panel === 'side by side') await page.keyboard.press('Control+Shift+L');
        await expect(page.locator('aside.git-side')).toHaveCount(panel === 'closed' ? 0 : 1);
        for (const size of sizes) {
          await page.setViewportSize(size);
          const where = `${panel}, ${size.width}×${size.height}`;
          for (const [key, name] of [
            ['Control+K', words.conv],
            ['Control+P', words.open],
          ]) {
            const box = await palette(key, name, where);
            await page.keyboard.press('Escape');
            await expect(box).toBeHidden();
          }
          await page.getByRole('button', { name: words.editor }).click();
          await page.getByRole('menuitemradio', { name: words.editorItem }).click();
          await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
          // Ctrl+P in the view: the same palette, on the core's index; the file opens in a tab of the view.
          await workbench(page).click({ position: { x: 400, y: 300 } });
          const box = await palette('Control+P', words.open, `${where}, in the view`);
          await page.keyboard.type('users.ts');
          await box
            .getByRole('option', { name: /users\.ts/ })
            .first()
            .click();
          await expect(box).toBeHidden();
          await expect(editorLines(page)).toContainText('router');
          // VS Code's search in the file, within the view.
          await editorLines(page).click();
          await page.keyboard.press('Control+F');
          const find = workbench(page).locator('.find-widget.visible');
          await expect(find).toBeVisible();
          await page.mouse.move(size.width / 2, 20);
          expect(await sticksOut(view, ['.vscode-bar']), `the view's bar, ${where}`).toEqual([]);
          expect(await within(workbench(page), view.locator('.vscode-place')), `the workbench within the view, ${where}`).toBe(true);
          expect(await within(find, view.locator('.vscode-place')), `the search within the view, ${where}`).toBe(true);
          if (SHOTS) {
            fs.mkdirSync(SHOTS, { recursive: true });
            await page.screenshot({ path: path.join(SHOTS, `editor-room-${lang}-${panel.replace(/ /g, '-')}-${size.width}.app.png`) });
          }
          await page.keyboard.press('Escape');
          await back();
        }
        if (panel !== 'closed') {
          await expect(page.locator('main.conv')).toBeVisible();
          await page.keyboard.press(panel === 'alone' ? 'Control+Shift+B' : 'Control+Shift+L');
          await expect(page.locator('aside.git-side')).toHaveCount(0);
        }
      }
    });

    // A7 (ESC-476): the view on the preview's worktree, its count of the lines changed and « Comparer » in its bar, in
    // their room at both sizes.
    test('the marks of the changes of the worktree in the view fit in their room', async ({ app }) => {
      test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
      test.setTimeout(240_000);
      const { page } = app;
      const write = previewsProject(app.repo);
      await addProject(page, app.repo, { worktrees: true, lang });
      await agentAtWork(app.repo, write);
      const view = await editorAsThePreview(page, lang);
      for (const size of SIZES) {
        await page.setViewportSize(size);
        await page.mouse.move(size.width / 2, 20);
        expect(await sticksOut(view, ['.vscode-bar']), `${size.width}×${size.height}`).toEqual([]);
        await expect(view.getByTestId('vscode-compare')).toBeVisible();
        if (SHOTS) {
          fs.mkdirSync(SHOTS, { recursive: true });
          await page.screenshot({ path: path.join(SHOTS, `vscode-changes-room-${lang}${size.name}.app.png`) });
        }
      }
    });
  });
