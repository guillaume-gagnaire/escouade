// ESC-447: the project's editor is now the VS Code view (tests/e2e/vscode-view.spec.ts). These tests of the CodeMirror
// editor of the 2.0 design are fixme until the tickets that bring what they check back in the view: A7 (the marks of
// the changes from the base, the comparison with the disk, the choice of the source), A3 (its keys, its focus,
// Ctrl+P, the links of the conversations) and A4 (the theme followed at once, the fonts).
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
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  await expect.poll(() => fs.existsSync(path.join(wt, 'src', 'routes', 'users.ts'))).toBe(true);
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

// A7: the editor of the preview, its marks of the changes from main and its source (A4: its theme and fonts).
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

// A4: the theme followed at once (the A7 marks of the preview's project on the way).
test.fixme('the editor follows the theme at once, CodeMirror’s own base theme included', async ({ app }) => {
  const { page } = app;
  const write = previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo, write);
  const ed = await editorAsThePreview(page);
  const cm = ed.locator('.cm-editor');
  const keyword = cm.locator('.cm-line').first().locator('span').first();
  const colorOf = (el: Locator) => el.evaluate((n) => getComputedStyle(n).color);
  const tokenOf = (name: string) =>
    page.evaluate((name) => {
      const probe = document.createElement('span');
      probe.style.color = `var(${name})`;
      document.body.append(probe);
      const c = getComputedStyle(probe).color;
      probe.remove();
      return c;
    }, name);
  const classes: Record<string, string> = {};
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await expect.poll(() => colorOf(keyword), `a keyword in the ${t} theme`).toLookLike(await tokenOf('--syn-k'));
    classes[t] = (await cm.getAttribute('class')) ?? '';
  }
  // CodeMirror's own base theme, light or dark: its scoping class changes with the window's theme.
  expect(classes.light).not.toBe(classes.dark);
});

// A3: the caret, the focus and Ctrl+P in the view; A7: the source's menu on the worktree.
test.fixme('the editor stays still with prefers-reduced-motion', async ({ app }) => {
  const { page } = app;
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const write = previewsProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await agentAtWork(app.repo, write);
  const ed = await editorAsThePreview(page);
  await ed.locator('.cm-content').click();
  // The caret does not blink.
  const blink = await ed.locator('.cm-cursorLayer').evaluate((n) => getComputedStyle(n).animationDuration);
  expect(parseFloat(blink), `its blink lasts ${blink}`).toBeLessThan(0.001);
  const motion = (el: Locator) => el.evaluate((e) => getComputedStyle(e).animationName);
  // The menu of the source, and « Ouvrir un fichier ».
  await ed.locator('.ed-bar .src').click();
  const menu = page.getByRole('menu', { name: 'Parcourir' });
  await expect(menu).toBeVisible();
  expect(await motion(menu)).toBe('none');
  await page.keyboard.press('Escape');
  await ed.locator('.ed-bar .src').click();
  await expect(menu).toBeHidden();
  await page.keyboard.press('Control+P');
  const palette = page.getByRole('dialog', { name: 'Ouvrir un fichier' });
  await expect(palette).toBeVisible();
  expect(await motion(palette)).toBe('none');
  expect(await motion(page.locator('.overlay'))).toBe('none');
  // Its keys and its close button drawn, no character as an icon.
  await expect(palette.getByRole('button', { name: 'Fermer' }).locator('svg[data-icon="close"]')).toBeVisible();
  await expect(palette.locator('.keys svg[data-icon="up"]')).toBeVisible();
  await page.keyboard.press('Escape');
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

    // The editor and the two palettes (Ctrl K, Ctrl P) at 1 028, 1 280 and 1 440 px, the side panel closed, open alone
    // and side by side with the conversation: nothing sticks out, as the folds go by the room of each (@container).
    // A3: Ctrl+P and the keys in the view; A7: the source and the marks of the preview's worktree.
    test.fixme('nothing of the editor or of its searches sticks out, at 1028, 1280 and 1440 px, whatever the side panel', async ({
      app,
    }) => {
      test.setTimeout(300_000);
      const { page } = app;
      const write = previewsProject(app.repo);
      await addProject(page, app.repo, { worktrees: true, lang });
      await agentAtWork(app.repo, write);
      const words =
        lang === 'fr'
          ? {
              split: 'Conversation et fichiers côte à côte',
              classic: 'Disposition classique',
              conv: 'Rechercher dans les conversations',
              open: 'Ouvrir un fichier',
            }
          : { conv: 'Search conversations', open: 'Open a file' };
      await editorAsThePreview(page, lang);
      const back = () => page.locator('main.editor .ed-bar .btn').first().click();
      const sizes = [
        { width: 1028, height: 779 },
        { width: 1280, height: 800 },
        { width: 1440, height: 900 },
      ];
      for (const panel of ['closed', 'alone', 'side by side'] as const) {
        // The panel set at the widest, where its buttons all show.
        await page.setViewportSize({ width: 1440, height: 900 });
        if (await page.locator('main.editor').isVisible()) await back();
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
            await page.keyboard.press(key);
            const palette = page.getByRole('dialog', { name });
            await expect(palette).toBeVisible();
            expect(await sticksOut(palette, ['.search', '.scope', '.foot', '.keys']), `${name}, ${where}`).toEqual([]);
            await page.keyboard.press('Escape');
            await expect(palette).toBeHidden();
          }
          await page.getByRole('button', { name: lang === 'fr' ? 'Ouvrir l’éditeur du projet' : 'Open the project’s editor' }).click();
          const ed = page.locator('main.editor');
          await expect(ed.locator('.cm-content')).toBeVisible();
          await ed.locator('.cm-content').click();
          await page.keyboard.press('Control+F');
          await expect(ed.locator('.cm-find')).toBeVisible();
          await page.mouse.move(size.width / 2, 20);
          expect(
            await sticksOut(ed, ['.ed-bar', '.tree-h', '.th-row', '.crumbs', '.status', '.cm-find', '.cm-find-row']),
            `the editor, ${where}`,
          ).toEqual([]);
          // The search stays within the text.
          const [fx, , fw] = await place(ed.locator('.cm-find'));
          const [px, , pw] = await place(ed.locator('.pane'));
          expect(fx >= px && fx + fw <= px + pw, `the search within the pane, ${where}`).toBe(true);
          if (SHOTS) {
            fs.mkdirSync(SHOTS, { recursive: true });
            await page.screenshot({ path: path.join(SHOTS, `editor-room-${lang}-${panel.replace(/ /g, '-')}-${size.width}.app.png`) });
          }
          await back();
        }
        if (panel !== 'closed') {
          await expect(page.locator('main.conv')).toBeVisible();
          await page.keyboard.press(panel === 'alone' ? 'Control+Shift+B' : 'Control+Shift+L');
          await expect(page.locator('aside.git-side')).toHaveCount(0);
        }
      }
    });
  });
