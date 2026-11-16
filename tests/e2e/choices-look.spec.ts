// The choices 50 to 58 of design/app-redesign/UX-NOTES.md (ESC-166, ESC-81, ESC-152), which wait for the user's word, in
// the app at 1028 × 779, where they bite, in both themes (ESC-84): what each shows is checked, and its capture goes to
// ESCOUADE_E2E_SHOTS when it is set (`choice-<n>-<what>[-light].png`), for the user to judge them.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_CODEX, send, test, openAdvanced, openSettingsPage } from './fixture';
import { editorLines, explorerRow, openInExplorer, workbench } from './vscode';

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

// ESC-447: the tree and the search of the CodeMirror editor are gone with it (the project's editor is the VS Code view).
// ESC-476 (A7) keeps of 53 to 56 what has a meaning in the view: its tree, with the letters of the files changed and
// their folders marked, and what its search finds (VS Code's, in Escouade's language). Retired with the old tree: its
// actions over its count (53), no file icons (54: VS Code draws those of its icon theme) and its 200 px in a narrow
// editor (55: VS Code's side bar keeps the width the user gives it).
test('53 and 56: the view’s tree with its files changed marked, and what its search finds', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(240_000);
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
  git(app.repo, '-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'files');
  await addProject(page, app.repo);
  // Changed since HEAD: marked in the tree.
  fs.writeFileSync(
    path.join(app.repo, 'src', 'routes', 'users.ts'),
    'export const users = [];\nexport const paginate = () => users.slice(0, 50);\n// paginate\n',
  );
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  await openInExplorer(page, 'src/routes/users.ts');
  await expect(editorLines(page)).toContainText('paginate');
  const wb = workbench(page);
  const badge = (name: string) =>
    explorerRow(page, name).evaluate(
      (el) => [el, ...el.querySelectorAll('*')].map((n) => getComputedStyle(n, '::after').content).find((c) => /^"\S+"$/.test(c)) ?? null,
    );
  for (const t of THEMES) {
    await theme(page, t);
    await page.mouse.move(SMALL.width / 2, 400);
    // 53: the file changed has its letter, its folders a mark.
    await expect.poll(() => badge('users.ts'), { timeout: 30_000 }).toBe('"M"');
    expect(await badge('routes')).not.toBeNull();
    expect(await badge('README.md')).toBeNull();
    await shot(page, 'choice-53-tree', t);
    // 56: « 1 sur 2 » on a match, then a regular expression that is not one, in error.
    await editorLines(page).click();
    await page.keyboard.press('Control+Home');
    await page.keyboard.press('Control+F');
    const find = wb.locator('.editor-instance .find-widget.visible');
    await expect(find).toBeVisible();
    await page.keyboard.press('Control+A');
    await page.keyboard.type('paginate');
    await page.keyboard.press('Enter');
    await expect(find.locator('.matchesCount')).toHaveText(/^\d+ sur 2$/);
    await shot(page, 'choice-56-count', t);
    await page.keyboard.press('Alt+R');
    await page.keyboard.press('Control+A');
    await page.keyboard.type('pagin(');
    await expect(find.locator('.monaco-findInput .monaco-inputbox.error')).toBeVisible();
    await shot(page, 'choice-56-invalid', t);
    await page.keyboard.press('Alt+R');
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
