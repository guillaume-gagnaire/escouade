// The choices 50 to 58 of design/app-redesign/UX-NOTES.md (ESC-166, ESC-81, ESC-152), which wait for the user's word, in
// the app at 1028 × 779, where they bite, in both themes (ESC-84): what each shows is checked, and its capture goes to
// ESCOUADE_E2E_SHOTS when it is set (`choice-<n>-<what>[-light].png`), for the user to judge them.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_CODEX, send, test, openAdvanced, openSettingsPage } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SMALL = { width: 1028, height: 779 };
const THEMES = ['dark', 'light'] as const;

/** The theme of the app, set as « Thème » would (its effect without its dialog), nothing gliding. */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
    if (!document.getElementById('still')) {
      const style = document.createElement('style');
      style.id = 'still';
      style.textContent = '*, *::before, *::after { transition: none !important; animation: none !important; }';
      document.head.append(style);
    }
  }, t);

async function shot(page: Page, name: string, t: 'dark' | 'light') {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  await page.screenshot({ path: path.join(SHOTS, `${name}${t === 'light' ? '-light' : ''}.png`) });
}

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, stdio: 'ignore' });

test('50 to 52: the header of a narrow conversation folds, its path gives up the project first, « … » takes the rest', async ({ app }) => {
  const { page } = app;
  await page.setViewportSize(SMALL);
  await addProject(page, app.repo, { worktrees: true });
  await send(page, 'edit');
  const head = page.locator('main.conv header.head');
  await expect(head.locator('.btn.test')).toBeVisible();
  // Side by side, the conversation is at its narrowest.
  await page.keyboard.press('Control+Shift+L');
  await expect(page.locator('aside.git-side.docked')).toBeVisible();
  for (const t of THEMES) {
    await theme(page, t);
    await page.mouse.move(SMALL.width / 2, 400);
    // 50: the name, the state and the main button stay; the durations, files and tokens go.
    await expect(head.locator('.name')).toBeVisible();
    await expect(head.locator('.state')).toBeVisible();
    await expect(head.locator('.fig.files')).toBeHidden();
    // 51: the branch kept.
    await expect(head.locator('.crumbs .branch')).toBeVisible();
    await shot(page, 'choice-50-51-header', t);
    // 52: the layout and the files in « … ».
    await head.locator('.actions .more').click();
    const menu = page.getByRole('menu');
    await expect(menu.getByRole('menuitem', { name: /Disposition classique/ })).toBeVisible();
    await shot(page, 'choice-52-more', t);
    await page.keyboard.press('Escape');
    await expect(menu).toBeHidden();
  }
});

test('53 to 56: the editor’s tree, its actions on hover, no file icons, its folds, and what the search finds', async ({ app }) => {
  const { page } = app;
  await page.setViewportSize(SMALL);
  for (const [file, text] of [
    ['src/routes/users.ts', 'export const users = [];\nexport const paginate = () => users;\n// paginate\n'],
    ['src/db/index.ts', 'export const db = {};\n'],
    ['README.md', '# demo-api\n'],
  ]) {
    fs.mkdirSync(path.dirname(path.join(app.repo, file)), { recursive: true });
    fs.writeFileSync(path.join(app.repo, file), text);
  }
  git(app.repo, 'add', '-A');
  git(app.repo, 'commit', '-qm', 'files');
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  const ed = page.locator('main.editor');
  const tree = ed.getByRole('tree', { name: 'Fichiers' });
  for (const name of ['src', 'routes']) {
    const dir = tree.getByRole('treeitem', { name: new RegExp(`^${name}`) });
    if ((await dir.getAttribute('aria-expanded')) === 'false') await dir.click();
  }
  await tree.getByRole('treeitem', { name: /^users\.ts/ }).click();
  await expect(ed.locator('.cm-content')).toContainText('paginate');
  // 55: the tree is 200 px in an editor of 904 px or less.
  expect(Math.round((await ed.locator('.tree').boundingBox())!.width)).toBeLessThanOrEqual(201);
  // 54: no icon of a file's type in the tree.
  await expect(tree.locator('.tree-row .ficon, .tree-row .ext-icon')).toHaveCount(0);
  for (const t of THEMES) {
    await theme(page, t);
    // 53: the tree's actions over its count while it is hovered.
    await tree.hover();
    await expect(ed.locator('.tree-h .actions')).toHaveCSS('opacity', '1');
    // The title stays whole: in a narrow column, the actions go over the folder's line under it.
    const title = ed.locator('.tree-h b');
    const [titleRight, actions] = await Promise.all([
      title.evaluate((n) => n.getBoundingClientRect().left + n.scrollWidth),
      ed.locator('.tree-h .actions').evaluate((n) => n.getBoundingClientRect()),
    ]);
    const titleBox = (await title.boundingBox())!;
    expect(actions.top >= titleBox.y + titleBox.height - 1 || actions.left >= titleRight, 'the title under the actions').toBe(true);
    expect(await title.evaluate((n) => n.scrollWidth <= n.clientWidth + 1), 'the title whole').toBe(true);
    await shot(page, 'choice-53-54-55-tree', t);
    // 56: « 1 sur 3 » on a match, then a regular expression that is not one, in red.
    await ed.locator('.cm-content').click();
    await page.keyboard.press('Control+Home');
    await page.keyboard.press('Control+F');
    const find = ed.locator('.cm-find');
    await expect(find).toBeVisible();
    const field = find.locator('.cm-find-field input').first();
    await field.fill('paginate');
    await page.keyboard.press('Enter');
    await expect(find.locator('.cm-find-count')).toHaveText(/^\d+ sur \d+$/);
    await shot(page, 'choice-56-count', t);
    await find.locator('.cm-find-tog').nth(1).click();
    await field.fill('pagin(');
    await expect(find).toHaveClass(/cm-find-invalid/);
    await shot(page, 'choice-56-invalid', t);
    await find.locator('.cm-find-tog').nth(1).click();
    await page.keyboard.press('Escape');
  }
});

test.describe('57 and 58', () => {
  test.use({ appEnv: { ESCOUADE_CODEX: FAKE_CODEX } });

  test('a setting’s row keeps its name’s room, and the harnesses are ordered by chips with arrows', async ({ app }) => {
    const { page } = app;
    await page.setViewportSize(SMALL);
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    for (const t of THEMES) {
      await theme(page, t);
      // 57: « Langue parlée » is a menu, its name on its own column.
      await openSettingsPage(dialog, 'Voix');
      const language = dialog.getByRole('combobox', { name: 'Langue parlée' });
      await language.scrollIntoViewIfNeeded();
      await expect(language).toBeVisible();
      await page.mouse.move(2, 400);
      await shot(page, 'choice-57-row', t);
      // 58: « Ordre des harness », each chip with its number and its arrows.
      await openSettingsPage(dialog, 'Comptes et harness');
      await openAdvanced(dialog);
      const order = dialog.getByRole('list', { name: 'Ordre des harness' });
      await order.scrollIntoViewIfNeeded();
      await expect(order.getByRole('button', { name: /Codex/ }).first()).toBeVisible();
      await page.mouse.move(2, 400);
      await shot(page, 'choice-58-order', t);
    }
  });
});
