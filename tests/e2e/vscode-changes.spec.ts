// The changes from the base in the VS Code view (ESC-476, A7): the marks of VS Code's quick diff between the numbers and
// the text, after an agent's change and as it goes on, and « n lignes modifiées par rapport à main » in the view's bar;
// the comparison with the base (the bar, the tab's menu, the palette) and with the disk (« Comparer avec le fichier
// enregistré », the conflict of a save); the letters of the explorer, its folders marked, and « Changements » under it,
// a deleted file there opened against its base; the source changed, each with its base; the base that moves; no view of
// VS Code's source control. ESCOUADE_E2E_SHOTS=<folder>: captures.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { editorLines, explorerRow, unfold, vscodeTabs, workbench } from './vscode';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', ...args], { cwd, encoding: 'utf8' }).trim();

function writeProject(repo: string) {
  const write = (file: string, data: string) => {
    fs.mkdirSync(path.dirname(path.join(repo, file)), { recursive: true });
    fs.writeFileSync(path.join(repo, file), data);
  };
  write('src/app.ts', 'export const answer = 42;\nexport const name = "demo";\n');
  write('src/gone.ts', 'export const gone = true;\n');
  write('README.md', '# demo\n');
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', 'Projet de test');
}

/**
 * The letter of a row of the explorer, as VS Code draws it (a badge in a `::after`), and the colour of its name: the
 * letter null when it has none, `•` for a folder marked by the files under it.
 */
const letterOf = (row: Locator) =>
  row.evaluate((el) => {
    let letter: string | null = null;
    for (const n of [el, ...el.querySelectorAll('*')]) {
      const c = getComputedStyle(n, '::after').content;
      const m = /^"(.+)"$/.exec(c);
      if (m && m[1].trim()) letter = m[1].trim();
    }
    const name = el.querySelector('.label-name') ?? el;
    return { letter, color: getComputedStyle(name).color };
  });

/** The marks of the quick diff in the editor's margin, by their kind, top to bottom. */
const marks = (page: Page) =>
  workbench(page)
    .locator('.editor-instance .monaco-editor .margin-view-overlays .dirty-diff-glyph')
    .evaluateAll((els) =>
      els
        .map((e) => ({
          top: e.parentElement?.getBoundingClientRect().top ?? 0,
          kind: /dirty-diff-(added|modified|deleted)/.exec(e.className)?.[1],
        }))
        .sort((a, b) => a.top - b.top)
        .map((m) => m.kind),
    );

/** The text of each side of the diff editor shown: its base (left), the file (right). */
const diffSides = (page: Page) => {
  const diff = workbench(page).locator('.monaco-diff-editor').first();
  return {
    diff,
    left: diff.locator('.editor.original .view-lines:not(.line-delete)').first(),
    right: diff.locator('.editor.modified .view-lines:not(.line-delete)').first(),
  };
};

/** The command `title` run from VS Code's palette (F1), by its name. */
async function palette(page: Page, title: string) {
  await page.keyboard.press('F1');
  const box = workbench(page).locator('.quick-input-widget');
  await expect(box).toBeVisible();
  await page.keyboard.type(title);
  await box.locator('.monaco-list-row', { hasText: title }).first().click();
}

test('the changes from the base in the VS Code view: marks, comparisons, letters, sources', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(600_000);
  const { page } = app;
  writeProject(app.repo);
  page.on('pageerror', (e) => console.log(`pageerror: ${e.stack ?? e}`.slice(0, 1500)));
  await addProject(page, app.repo, { worktrees: true });
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0), { timeout: 60_000 }).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  await expect.poll(() => fs.existsSync(path.join(wt, 'src', 'app.ts')), { timeout: 30_000 }).toBe(true);

  // 1. The agent's work in its worktree: a line changed and one added, a file deleted, a file added.
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'export const answer = 43;\nexport const name = "demo";\nexport const more = 1;\n');
  fs.rmSync(path.join(wt, 'src', 'gone.ts'));
  fs.writeFileSync(path.join(wt, 'src', 'new.ts'), 'export const fresh = 1;\n');

  // 2. The view on the agent's worktree: its folder alone shown, the project's folded.
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  const view = page.getByTestId('vscode-view');
  const wb = workbench(page);
  await expect(wb).toBeVisible({ timeout: 60_000 });
  const explorer = wb.locator('.explorer-folders-view');
  const rootRows = explorer.locator('.monaco-list-row[aria-level="1"]');
  await expect(rootRows).toHaveCount(1, { timeout: 30_000 });
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').nth(1).click();
  await expect(view.locator('.vscode-bar .src')).toContainText('agent-1');
  await expect(rootRows).toHaveCount(2, { timeout: 30_000 });
  await unfold(rootRows.nth(0), false);
  await unfold(rootRows.nth(1), true);
  const src = explorer.locator('.monaco-list-row[aria-level="2"]', { hasText: /^src$/ });
  await unfold(src, true);
  const row = (name: string) =>
    explorer.locator('.monaco-list-row[aria-level="3"]', { hasText: new RegExp(`^${name.replace('.', '\\.')}$`) });

  // 3. The letters of the explorer: M for the file changed, A for the one added, its folders marked.
  await expect.poll(async () => (await letterOf(row('app.ts'))).letter, { timeout: 30_000 }).toBe('M');
  expect((await letterOf(row('new.ts'))).letter).toBe('A');
  const tokens = await page.evaluate(() => {
    const s = getComputedStyle(document.querySelector('.app, #app') ?? document.documentElement);
    return { m: s.getPropertyValue('--git-m').trim(), a: s.getPropertyValue('--add').trim() };
  });
  expect((await letterOf(row('app.ts'))).color, 'the colour of M, git-m').toLookLike(tokens.m);
  expect((await letterOf(row('new.ts'))).color, 'the colour of A, add').toLookLike(tokens.a);
  expect((await letterOf(src)).letter, 'src marked by its files').not.toBeNull();
  expect((await letterOf(src)).color, 'src in the colour of its changes').not.toBe(
    (await letterOf(explorer.locator('.monaco-list-row', { hasText: /^README\.md$/ }).last())).color,
  );
  // The file deleted is not in the explorer: it shows in « Changements » under it.
  await expect(row('gone.ts')).toHaveCount(0);

  // 4. The marks of the file: line 1 modified, line 3 added; « 2 lignes modifiées par rapport à main ».
  await row('app.ts').click();
  await expect.poll(() => vscodeTabs(page)).toContain('app.ts');
  await expect(editorLines(page)).toContainText('answer = 43');
  await expect.poll(() => marks(page), { timeout: 30_000 }).toEqual(['modified', 'added']);
  const bar = view.getByTestId('vscode-changes');
  /** The tab of app.ts, not a comparison's. */
  const fileTab = () =>
    wb
      .locator('.tabs-container .tab')
      .filter({ hasText: /^app\.ts/ })
      .filter({ hasNotText: '↔' });
  /** An item of the menu of app.ts's tab, once the menu shows it. */
  const tabMenu = async (item: string) => {
    await fileTab().click({ button: 'right' });
    const entry = wb.locator('.context-view .action-item', { hasText: item });
    await expect(entry).toBeVisible();
    await page.waitForTimeout(300);
    await entry.click();
  };
  /** The tab shown closed, by VS Code's Ctrl+W. */
  const closeActive = async () => {
    await wb.locator('.tabs-container .tab.active').click();
    await page.keyboard.press('Control+W');
  };
  await expect(bar).toHaveText('2 lignes modifiées par rapport à main');
  if (SHOTS) {
    fs.mkdirSync(SHOTS, { recursive: true });
    await page.screenshot({ path: path.join(SHOTS, 'vscode-changes-marks.png') });
  }

  // 5. The agent goes on: its change on disk is read again, the marks and the count follow.
  fs.writeFileSync(
    path.join(wt, 'src', 'app.ts'),
    'export const answer = 42;\nexport const name = "demo";\nexport const more = 1;\nexport const most = 2;\n',
  );
  await expect(editorLines(page)).toContainText('most = 2', { timeout: 15_000 });
  await expect.poll(() => marks(page), { timeout: 30_000 }).toEqual(['added', 'added']);
  await expect(bar).toHaveText('2 lignes modifiées par rapport à main');
  // Typed in the view: at once, before it is saved.
  await editorLines(page).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('export const typed = 3;');
  // The empty last line replaced: VS Code marks the three lines changed.
  await expect.poll(() => marks(page).then((m) => m.length), { timeout: 15_000 }).toBe(3);
  await expect(bar).toHaveText('3 lignes modifiées par rapport à main');
  // A click on a mark: VS Code's peek of the change.
  await wb
    .locator('.editor-instance .monaco-editor .margin-view-overlays .dirty-diff-glyph')
    .first()
    .click({ position: { x: 2, y: 4 } });
  await expect(wb.locator('.dirty-diff.peekview-widget, .peekview-widget').first()).toBeVisible({ timeout: 15_000 });
  await page.keyboard.press('Escape');

  // 6. Compared with its base, from the view's bar: VS Code's diff editor, main on the left.
  await view.getByTestId('vscode-compare').click();
  const sides = diffSides(page);
  await expect(sides.diff).toBeVisible({ timeout: 15_000 });
  await expect(sides.left).toContainText('answer = 42');
  await expect(sides.left).not.toContainText('more = 1');
  await expect(sides.right).toContainText('typed = 3');
  await expect.poll(() => vscodeTabs(page)).toContain('app.ts (main) ↔ app.ts*');
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-changes-compare.png') });
  await closeActive();

  // From the tab's menu, and from the palette.
  await tabMenu('Comparer avec la base');
  await expect(sides.diff).toBeVisible({ timeout: 15_000 });
  await closeActive();
  await expect(sides.diff).toHaveCount(0);
  await editorLines(page).click();
  await palette(page, 'Comparer avec la base');
  await expect(sides.diff).toBeVisible({ timeout: 15_000 });
  await closeActive();

  // 7. Compared with the file saved: VS Code's own, from the tab's menu; the disk's version on the left.
  await tabMenu('Comparer avec le fichier enregistré');
  await expect(sides.diff).toBeVisible({ timeout: 15_000 });
  await expect(sides.left).not.toContainText('typed = 3');
  await expect(sides.right).toContainText('typed = 3');
  await closeActive();
  await expect(sides.diff).toHaveCount(0);
  // The conflict of a save (A2) leads there too: « Comparer », the disk's version against mine.
  await fileTab().click();
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'export const byTheAgent = true;\n');
  await page.waitForTimeout(1500);
  await editorLines(page).click();
  await page.keyboard.press('Control+s');
  const toast = wb.locator('.notifications-toasts .notification-toast', { hasText: 'le contenu du fichier est plus récent' });
  await expect(toast).toBeVisible({ timeout: 15_000 });
  await toast.getByRole('button', { name: 'Comparer' }).click();
  await expect(sides.diff).toBeVisible({ timeout: 15_000 });
  await expect(sides.left).toContainText('byTheAgent');
  await expect(sides.right).toContainText('typed = 3');
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-changes-conflict.png') });
  await closeActive();
  // Mine given up: the file is the agent's again.
  await fileTab().click();
  await palette(page, 'Rétablir le fichier');
  await expect(editorLines(page)).toContainText('byTheAgent', { timeout: 15_000 });

  // 8. « Changements » under the explorer: the files of the source changed, the deleted one opened against its base.
  const changes = wb.locator('.pane').filter({ has: page.locator('.pane-header', { hasText: 'Changements' }) });
  const header = changes.locator('.pane-header');
  await expect(header).toBeVisible({ timeout: 20_000 });
  if ((await header.getAttribute('aria-expanded')) !== 'true') await header.click();
  const list = changes.locator('.monaco-list-row');
  await expect(list.filter({ hasText: 'gone.ts' })).toBeVisible({ timeout: 15_000 });
  expect((await letterOf(list.filter({ hasText: 'gone.ts' }))).letter).toBe('D');
  await expect(list.filter({ hasText: 'new.ts' })).toBeVisible();
  await list.filter({ hasText: 'gone.ts' }).click();
  await expect(sides.diff).toBeVisible({ timeout: 15_000 });
  await expect(sides.left).toContainText('export const gone = true;');
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-changes-list.png') });
  await closeActive();

  // 9. The source changed: the project's checkout, its own base (HEAD), its own letters.
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').first().click();
  await unfold(rootRows.nth(1), false);
  await unfold(rootRows.nth(0), true);
  const psrc = explorer.locator('.monaco-list-row[aria-level="2"]', { hasText: /^src$/ });
  await unfold(psrc, true);
  const prow = (name: string) =>
    explorer.locator('.monaco-list-row[aria-level="3"]', { hasText: new RegExp(`^${name.replace('.', '\\.')}$`) });
  // The worktree's changes are not the project's.
  await expect(prow('gone.ts')).toBeVisible();
  expect((await letterOf(prow('app.ts'))).letter).toBeNull();
  await prow('app.ts').click();
  await expect(editorLines(page)).toContainText('answer = 42');
  await expect(bar).toHaveText('Identique à HEAD');
  await expect(view.getByTestId('vscode-compare')).toHaveCount(0);
  // Changed on disk in the project: its letter, its marks.
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'export const answer = 42;\nexport const name = "changed";\n');
  await expect(editorLines(page)).toContainText('"changed"', { timeout: 15_000 });
  await expect.poll(() => marks(page), { timeout: 30_000 }).toEqual(['modified']);
  await expect(bar).toHaveText('1 ligne modifiée par rapport à HEAD');
  await expect.poll(async () => (await letterOf(prow('app.ts'))).letter, { timeout: 30_000 }).toBe('M');

  // 10. The base moves: committed, the file is its base again: no mark, no letter.
  git(app.repo, 'commit', '-qam', 'name changed');
  await expect.poll(() => marks(page), { timeout: 30_000 }).toEqual([]);
  await expect(bar).toHaveText('Identique à HEAD', { timeout: 30_000 });
  await expect.poll(async () => (await letterOf(prow('app.ts'))).letter, { timeout: 30_000 }).toBeNull();

  // 11. No view of VS Code's source control, nor its commit or push: its entry of the activity bar is not shown (the
  // explorer's and the search's are), its key opens nothing, and its « Afficher » from the palette shows the explorer.
  await expect(wb.locator('.activitybar .codicon-explorer-view-icon')).toBeVisible();
  await expect(wb.locator('.activitybar .codicon-search-view-icon')).toBeVisible();
  await expect(wb.locator('.activitybar .codicon-source-control-view-icon')).toBeHidden();
  const scmShown = () => wb.locator('.scm-view, .scm-editor, .scm-input').filter({ visible: true }).count();
  await editorLines(page).click();
  await page.keyboard.press('Control+Shift+G');
  await page.waitForTimeout(800);
  expect(await scmShown()).toBe(0);
  await page.keyboard.press('F1');
  await page.keyboard.type('contrôle de code source');
  const rows = wb.locator('.quick-input-widget .monaco-list-row');
  await expect(rows.first()).toBeVisible();
  // No command that commits, pushes or syncs.
  await expect(rows.filter({ hasText: /Valider|Commit|Push|Envoyer|Synchroniser/ })).toHaveCount(0);
  const show = rows.filter({ hasText: /Afficher.*[Cc]ontrôle de code source/ }).first();
  if (await show.count()) {
    await show.click();
    await page.waitForTimeout(800);
  } else await page.keyboard.press('Escape');
  expect(await scmShown()).toBe(0);
  await expect(wb.locator('.explorer-folders-view')).toBeVisible();
});
