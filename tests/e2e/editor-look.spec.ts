// ESC-447: the project's editor is now the VS Code view (tests/e2e/vscode-view.spec.ts). These tests of the CodeMirror
// editor of the 2.0 design are fixme until the tickets that bring what they check back in the view: A7 (the marks of
// the changes from the base, the comparison with the disk, the choice of the source), A3 (its keys, its focus,
// Ctrl+P, the links of the conversations); A4 (the theme followed at once, the fonts) has tests of the view, below.
//
// The editor in the 2.0 design (ESC-81, lot 9): its bar (back to the conversation, the source, « Enregistré »,
// « Enregistrer »), the tree with its changed folders marked and its letters, the tabs, the path above the text with
// what changed, the search floating at the top right, the marks of the changes between the numbers and the text, the
// status line. Compared to the preview (design/app-redesign/index.html, `#editor`) in both themes at 1440 × 900 and at
// CI's 1028 × 779; the English labels in their room; the motion stopped by prefers-reduced-motion. The captures of the
// app and of the preview go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';
import { editorLines, openInExplorer, vscodeTabs, workbench } from './vscode';

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

/**
 * Opens the editor on the agent's worktree, the preview's folders open, the preview's three tabs in its order with
 * `users.ts` shown.
 */
async function editorAsThePreview(page: Page, lang: 'fr' | 'en' = 'fr') {
  await page.getByRole('button', { name: lang === 'fr' ? 'Ouvrir l’éditeur du projet' : 'Open the project’s editor' }).click();
  const ed = page.locator('main.editor');
  await ed.locator('.ed-bar .src').click();
  await page.getByRole('menuitemradio').nth(1).click();
  await expect(ed.locator('.status .source')).toContainText('worktree');
  const tree = ed.getByRole('tree', { name: lang === 'fr' ? 'Fichiers' : 'Files' });
  const dir = (name: string) => tree.getByRole('treeitem', { name: new RegExp(`^${name}`) });
  for (const name of ['src', 'db', 'routes', 'tests']) {
    await expect(dir(name)).toBeVisible();
    if ((await dir(name).getAttribute('aria-expanded')) === 'false') await dir(name).click();
    await expect(dir(name)).toHaveAttribute('aria-expanded', 'true');
  }
  // Every tab closed, then the preview's three opened in its order.
  const tabs = ed.getByRole('tablist');
  while ((await tabs.getByRole('tab').count()) > 0) await tabs.getByRole('tab').first().getByRole('button').click();
  // A file's name, then its letter when git has one for it.
  const file = (name: string) => tree.getByRole('treeitem', { name: new RegExp(`^${name.replaceAll('.', '\\.')}( [MAD])?$`) });
  for (const name of ['users.ts', 'paginate.ts', 'users.test.ts']) await file(name).click();
  // A tab is named by its file and its close button: found by its file's name alone.
  await tabs
    .locator('.tab')
    .filter({ has: page.getByText('users.ts', { exact: true }) })
    .click();
  await expect(ed.locator('.cm-content')).toContainText('paginate(db.users');
  await expect(ed.locator('.crumbs .vs')).toHaveText(lang === 'fr' ? '5 lignes modifiées par rapport à main' : '5 lines changed from main');
  return ed;
}

/** The search in the file, as the preview shows it: « paginate », whole word, on its first match. */
async function findAsThePreview(page: Page, ed: Locator) {
  await ed.locator('.cm-content').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.press('Control+F');
  const find = ed.locator('.cm-find');
  await expect(find).toBeVisible();
  await page.keyboard.type('paginate');
  await find.locator('.cm-find-tog').nth(2).click();
  await page.keyboard.press('Enter');
  await expect(find.locator('.cm-find-count')).toHaveText(/^1 (sur|of) 3$/);
  return find;
}

/** The preview at `#route`, in theme `t`: from a blank page, for its script to open the route. */
async function previewAt(preview: Page, route: string, t: 'dark' | 'light') {
  await preview.goto('about:blank');
  await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#${route}`);
  await preview.evaluate(() => document.fonts.ready);
  await still(preview);
}

// A7: the editor of the preview, its marks of the changes from main and its source (its theme and fonts are the VS Code
// view's own since A4, below).
test.fixme('the editor looks as the preview draws it, in both themes, at both sizes', async ({ app }) => {
  test.setTimeout(240_000);
  const exe = chromium.executablePath();
  test.skip(
    !fs.existsSync(exe) || !fs.existsSync(PREVIEW_FONT),
    'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)',
  );
  const { page } = app;
  const write = previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo, write);
  const ed = await editorAsThePreview(page);
  const find = await findAsThePreview(page, ed);
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

        // The bar: 52 px, back to the conversation, the source, « Enregistré », « Enregistrer ».
        const bar = ed.locator('header.ed-bar');
        const pbar = pv.locator('.ed-bar');
        expect.soft(await place(bar), `bar's place, ${where}`).toLookLike(await place(pbar));
        expect.soft(await look(bar), `bar, ${where}`).toLookLike(await look(pbar));
        expect
          .soft(await look(bar.locator('.btn').first()), `« Conversation », ${where}`)
          .toLookLike(await look(pbar.locator('.btn').first()));
        const src = bar.locator('.src');
        const psrc = pbar.locator('.agent-pick');
        expect.soft(await lookBut(src, 'width'), `the source, ${where}`).toLookLike(await lookBut(psrc, 'width'));
        expect.soft((await place(src)).slice(0, 2), `the source's place, ${where}`).toLookLike((await place(psrc)).slice(0, 2));
        expect
          .soft(await lookBut(src.locator('.kind'), 'width'), `« worktree », ${where}`)
          .toLookLike(await lookBut(psrc.locator('.wt'), 'width'));
        expect.soft(await look(bar.locator('.saved')), `« Enregistré », ${where}`).toLookLike(await look(pbar.locator('.saved')));
        const save = bar.locator('.btn').last();
        const psave = pbar.locator('.btn').last();
        expect.soft(await look(save), `« Enregistrer », ${where}`).toLookLike(await look(psave));
        expect.soft(await place(save), `« Enregistrer »'s place, ${where}`).toLookLike(await place(psave));

        // The tree: its column, its title and count, its folder, its rows, letters and dots.
        const side = ed.locator('aside.editor-side');
        const ptree = pv.locator('.tree');
        const [sx, sy, sw] = await place(side);
        const [px, py, pw] = await place(ptree);
        expect.soft([sx, sy, sw], `the tree's place, its line included, ${where}`).toEqual([px, py, pw]);
        expect
          .soft(await lookBut(side.locator('.tree-h b'), 'width'), `« Fichiers », ${where}`)
          .toLookLike(await lookBut(ptree.locator('.tree-h b'), 'width'));
        expect
          .soft(await lookBut(side.locator('.tree-h b .n'), 'width'), `its count, ${where}`)
          .toLookLike(await lookBut(ptree.locator('.tree-h b span'), 'width'));
        expect
          .soft(await lookBut(side.locator('.tree-h .caption'), 'width'), `its folder, ${where}`)
          .toLookLike(await lookBut(ptree.locator('.tree-h .caption'), 'width'));
        expect
          .soft(await place(side.locator('.tree-h .caption')), `its folder's place, ${where}`)
          .toLookLike(await place(ptree.locator('.tree-h .caption')));
        const row = side.locator('.row').first();
        const prow = ptree.locator('.node').first();
        expect.soft(await place(row), `the first row's place, ${where}`).toLookLike(await place(prow));
        expect.soft(await look(row), `a folder, ${where}`).toLookLike(await look(prow));
        const on = side.locator('.row.on');
        const pon = ptree.locator('.node[aria-current="true"]');
        expect.soft(await place(on), `the file shown, its place, ${where}`).toLookLike(await place(pon));
        expect.soft(await look(on), `the file shown, ${where}`).toLookLike(await look(pon));
        for (const l of ['m', 'a'])
          expect
            .soft(await lookBut(side.locator(`.letter.${l}`).first(), 'width'), `letter ${l.toUpperCase()}, ${where}`)
            .toLookLike(await lookBut(ptree.locator(`.letter.${l}`).first(), 'width'));
        expect
          .soft(await look(side.locator('.mdot').first()), `a changed folder's dot, ${where}`)
          .toLookLike(await look(ptree.locator('.mdot').first()));
        const [mx] = await place(side.locator('.mdot').first());
        const [pmx] = await place(ptree.locator('.mdot').first());
        expect.soft(mx, `the dot after its folder's name, ${where}`).toBe(pmx);

        // The tabs: 38 px, the one shown lit and underlined.
        const tabs = ed.locator('.tabs');
        const ptabs = pv.locator('.ed-tabs');
        expect.soft(await lookBut(tabs, 'width'), `tabs, ${where}`).toLookLike(await lookBut(ptabs, 'width'));
        const tab = tabs.locator('.tab.on');
        const ptab = ptabs.locator('[aria-selected="true"]');
        expect.soft(await look(tab), `the tab shown, ${where}`).toLookLike(await look(ptab));
        expect.soft(await place(tab), `the tab shown, its place, ${where}`).toLookLike(await place(ptab));
        expect
          .soft(await look(tabs.locator('.tab:not(.on)').first()), `another tab, ${where}`)
          .toLookLike(await look(ptabs.locator('[aria-selected="false"]').first()));
        expect.soft(await look(tab.locator('.close')), `its close button, ${where}`).toLookLike(await look(ptab.locator('.x')));

        // The path above the text, and what changed.
        const crumbs = ed.locator('.crumbs');
        const pcrumbs = pv.locator('.ed-crumbs');
        expect.soft(await place(crumbs), `the path's place, ${where}`).toLookLike(await place(pcrumbs));
        expect.soft(await look(crumbs), `the path, ${where}`).toLookLike(await look(pcrumbs));
        expect
          .soft(await lookBut(crumbs.locator('.last'), 'width'), `the file's name, ${where}`)
          .toLookLike(await lookBut(pcrumbs.locator('span[style]'), 'width'));
        expect
          .soft(await lookBut(crumbs.locator('.vs'), 'width'), `what changed, ${where}`)
          .toLookLike(await lookBut(pcrumbs.locator('.vs'), 'width'));
        expect
          .soft(await lookBut(crumbs.locator('.vs .add'), 'width'), `« 5 lignes », ${where}`)
          .toLookLike(await lookBut(pcrumbs.locator('.vs .add'), 'width'));

        // The search, floating at the top right.
        const pfind = pv.locator('.find');
        expect.soft(await place(find), `the search's place, ${where}`).toLookLike(await place(pfind));
        expect.soft(await look(find), `the search, ${where}`).toLookLike(await look(pfind));
        const field = find.locator('.cm-find-field').first();
        expect.soft(await lookBut(field, 'width'), `its field, ${where}`).toLookLike(await lookBut(pfind.locator('.field.focus'), 'width'));
        expect.soft(await place(field), `its field's place, ${where}`).toLookLike(await place(pfind.locator('.field.focus')));
        expect
          .soft(await place(find.locator('.cm-find-field').last()), `the replacement's place, ${where}`)
          .toLookLike(await place(pfind.locator('.field:not(.focus)')));
        expect
          .soft(await lookBut(find.locator('.cm-find-field').last(), 'width'), `the replacement, ${where}`)
          .toLookLike(await lookBut(pfind.locator('.field:not(.focus)'), 'width'));
        expect
          .soft(await lookBut(find.locator('.cm-find-count'), 'width'), `« 1 sur 3 », ${where}`)
          .toLookLike(await lookBut(pfind.locator('.count'), 'width'));
        for (const pressed of ['true', 'false'])
          expect
            .soft(await look(find.locator(`.cm-find-tog[aria-pressed="${pressed}"]`).first()), `a toggle ${pressed}, ${where}`)
            .toLookLike(await look(pfind.locator(`.tog[aria-pressed="${pressed}"]`).first()));
        expect
          .soft(await look(find.locator('.cm-find-btn').first()), `an icon button, ${where}`)
          .toLookLike(await look(pfind.locator('.icon-btn').first()));
        expect
          .soft(await look(find.locator('.cm-find-mini').first()), `« Remplacer », ${where}`)
          .toLookLike(await look(pfind.locator('.mini-btn').first()));

        // The text: its font, its numbers, the marks of the changes, where it starts; the matches.
        const code = pv.locator('.code-ed');
        const content = ed.locator('.cm-content');
        const fontOf = (el: Locator) =>
          el.evaluate((n) => {
            const c = getComputedStyle(n);
            return [c.fontFamily, c.fontSize, c.lineHeight, c.color];
          });
        expect.soft(await fontOf(content), `the text's font, ${where}`).toLookLike(await fontOf(code));
        const nb = ed.locator('.cm-lineNumbers .cm-gutterElement').nth(1);
        const pnb = code.locator('.l .nb').first();
        expect.soft(await lookBut(nb, 'width'), `a line's number, ${where}`).toLookLike(await lookBut(pnb, 'width'));
        const [nbx, nby, nbw] = await place(nb);
        const [pnbx, pnby, pnbw] = await place(pnb);
        expect.soft([nbx + nbw, nby], `the numbers' right edge and first line, ${where}`).toEqual([pnbx + pnbw, pnby]);
        const line = ed.locator('.cm-line').first();
        const startOf = (el: Locator) =>
          el.evaluate((n) => Math.round(n.getBoundingClientRect().x + parseFloat(getComputedStyle(n).paddingLeft)));
        expect
          .soft(await startOf(line), `where the text starts, ${where}`)
          .toBe(await startOf(code.locator('.l').first().locator('> span').nth(2)));
        const mark = (el: Locator) =>
          el.evaluate((n) => {
            const s = getComputedStyle(n, '::before');
            const r = n.getBoundingClientRect();
            return { x: Math.round(r.x), width: Math.round(r.width), background: s.backgroundColor, radius: s.borderRadius };
          });
        const pmark = (el: Locator) =>
          el.evaluate((n) => {
            const s = getComputedStyle(n);
            const r = n.getBoundingClientRect();
            return { x: Math.round(r.x), width: Math.round(r.width), background: s.backgroundColor, radius: s.borderRadius };
          });
        expect
          .soft(await mark(ed.locator('.cm-change.changed:not(.modified)').first()), `a line added, ${where}`)
          .toLookLike(await pmark(code.locator('.l.ad .gm').first()));
        expect
          .soft(await mark(ed.locator('.cm-change.modified').first()), `a line modified, ${where}`)
          .toLookLike(await pmark(code.locator('.l.md .gm').first()));
        const bg = (el: Locator) => el.evaluate((n) => [getComputedStyle(n).backgroundColor, getComputedStyle(n).color]);
        expect
          .soft(await bg(ed.locator('.cm-searchMatch-selected')), `the match the cursor is on, ${where}`)
          .toLookLike(await bg(code.locator('.hit.cur-hit')));
        expect
          .soft((await bg(ed.locator('.cm-searchMatch:not(.cm-searchMatch-selected)').first()))[0], `a match, ${where}`)
          .toLookLike((await bg(code.locator('.hit:not(.cur-hit)').first()))[0]);

        // The status line.
        const status = ed.locator('.status');
        const pstatus = pv.locator('.ed-status');
        expect.soft(await place(status), `the status line's place, ${where}`).toLookLike(await place(pstatus));
        expect.soft(await look(status), `the status line, ${where}`).toLookLike(await look(pstatus));
        expect
          .soft(await lookBut(status.locator('.source .mono'), 'width'), `its worktree, ${where}`)
          .toLookLike(await lookBut(pstatus.locator('.end .mono'), 'width'));

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
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
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
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
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
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
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
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
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
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
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

  // A7: the bar, the source and the marks of the changes of the editor of the preview.
  test.fixme('the labels of the editor fit in their place, at both sizes', async ({ app }) => {
    const { page } = app;
    const write = previewsProject(app.repo);
    await addProject(page, app.repo, { worktrees: true, lang: 'en' });
    await agentAtWork(app.repo, write);
    const ed = await editorAsThePreview(page, 'en');
    const find = await findAsThePreview(page, ed);
    for (const size of SIZES) {
      await page.setViewportSize(size);
      await page.mouse.move(size.width / 2, 20);
      // What is cut short, or overflows its box: none of it, but the names of the user's files and folders, cut by design.
      const cut = await ed.evaluate((root) =>
        [...root.querySelectorAll<HTMLElement>('.ed-bar *, .tree-h *, .tabs *, .crumbs *, .status *, .cm-find *')]
          .filter(
            (el) =>
              el.offsetParent &&
              el.children.length === 0 &&
              !el.closest('.caption, .src .nm, .path, .source') &&
              (el.scrollWidth > el.clientWidth + 1 || el.scrollHeight > el.clientHeight + 1),
          )
          .map((el) => `${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 40)} »`),
      );
      expect(cut, `${size.width}×${size.height}`).toEqual([]);
      await expect(ed.getByRole('button', { name: 'Conversation' })).toBeVisible();
      await expect(ed.getByText('Saved', { exact: true })).toBeVisible();
      await expect(find.getByRole('button', { name: 'Replace all' })).toHaveText('All');
      await expect(find.locator('.cm-find-count')).toHaveText('1 of 3');
      await expect(ed.locator('.status')).toContainText('Spaces: 2');
      // The status line, the bar and the search hold on one line.
      for (const sel of ['.status', '.ed-bar', '.crumbs'])
        expect((await ed.locator(sel).boundingBox())!.height, sel).toBeLessThanOrEqual(52);
      if (SHOTS) {
        fs.mkdirSync(SHOTS, { recursive: true });
        await page.screenshot({ path: path.join(SHOTS, `editor${size.name}-en.app.png`) });
      }
    }
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
          ? { conv: 'Rechercher dans les conversations', open: 'Ouvrir un fichier', editor: 'Ouvrir l’éditeur du projet' }
          : { conv: 'Search conversations', open: 'Open a file', editor: 'Open the project’s editor' };
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

    // A7: the source and the marks of the changes of the preview's worktree in the view, in their room.
    test.fixme('the marks of the changes of the worktree in the view fit in their room', async ({ app }) => {
      const { page } = app;
      const write = previewsProject(app.repo);
      await addProject(page, app.repo, { worktrees: true, lang });
      await agentAtWork(app.repo, write);
      const ed = await editorAsThePreview(page, lang);
      expect(await sticksOut(ed, ['.ed-bar', '.tree-h', '.th-row', '.crumbs', '.status'])).toEqual([]);
    });
  });
