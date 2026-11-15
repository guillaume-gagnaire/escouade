// The VS Code view (ESC-447, from ESC-444's study), the editor of a project: VS Code's workbench
// (`@codingame/monaco-vscode-api`) on the files of the core: a text and a binary file read in bytes, a file an agent changed on disk read again, the
// conflict shown when it changed under a modified tab, the agent's worktree as a source; then in a pane beside the
// agent, moved, resized, closed and opened again without losing its tabs. ESCOUADE_E2E_SHOTS=<folder>: captures.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

async function at(el: Locator, fx: number, fy: number) {
  const b = (await el.boundingBox())!;
  return { x: b.x + b.width * fx, y: b.y + b.height * fy };
}

/** Drags `from` to the point `to` of the work area, where the panes' compass shows. */
async function drag(page: Page, from: Locator, to: { x: number; y: number }) {
  const b = (await from.boundingBox())!;
  const x = b.x + b.width / 2;
  const y = b.y + b.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 12, y + 12, { steps: 3 });
  await page.mouse.move(to.x, to.y, { steps: 12 });
  await expect(page.getByTestId('pane-drop')).toBeVisible();
  await page.mouse.up();
}

/** Drags `from` over the pane `over`, onto the cell `zone` of the compass shown there. */
async function dragTo(page: Page, from: Locator, over: Locator, zone: 'left' | 'right' | 'center') {
  const b = (await from.boundingBox())!;
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width / 2 + 12, b.y + b.height / 2 + 12, { steps: 3 });
  const o = await at(over, 0.5, 0.5);
  await page.mouse.move(o.x, o.y, { steps: 12 });
  const cell = page.getByTestId('pane-drop').locator(`[data-zone="${zone}"]`);
  await expect(cell).toBeVisible();
  const c = await at(cell, 0.5, 0.5);
  await page.mouse.move(c.x, c.y, { steps: 6 });
  await expect(cell).toHaveClass(/hot/);
  await page.mouse.up();
}

/** Whether the workbench is shown, laid over the place of `el` (a view, or a pane that holds one), to a pixel. */
const laidOver = (el: Locator) =>
  el.locator('.vscode-place').evaluate((place) => {
    const host = document.querySelector<HTMLElement>('#vscode-layer .vscode-host');
    if (!host || getComputedStyle(host).visibility !== 'visible') return false;
    const a = place.getBoundingClientRect();
    const b = host.getBoundingClientRect();
    return (
      Math.abs(a.left - b.left) <= 1 &&
      Math.abs(a.top - b.top) <= 1 &&
      Math.abs(a.width - b.width) <= 1 &&
      Math.abs(a.height - b.height) <= 1
    );
  });

const keys = (page: Page) => page.getByTestId('pane').evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.paneKey));

/** The labels of the view's editor tabs, the modified ones marked `*`. */
const tabs = (page: Page) =>
  page
    .locator('#vscode-layer .tabs-container .tab')
    .evaluateAll((els) =>
      els.map((e) => `${e.querySelector('.label-name')?.textContent?.trim()}${e.classList.contains('dirty') ? '*' : ''}`),
    );

function writeProject(repo: string) {
  const write = (file: string, data: string | Buffer) => {
    fs.mkdirSync(path.dirname(path.join(repo, file)), { recursive: true });
    fs.writeFileSync(path.join(repo, file), data);
  };
  write('src/app.ts', 'export const answer = 42;\n');
  write('src/f1.ts', '// Fichier 1\r\nexport function f1(x: number): number {\r\n  return x;\r\n}\r\n');
  write('assets/logo.png', Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, 0x49, 0x48, 0x44, 0x52, 0xff, 0]));
  const git = (...args: string[]) => execFileSync('git', args, { cwd: repo });
  git('add', '-A');
  git('commit', '-qm', 'Projet de test');
}

test('the VS Code view: the project’s editor, on the core’s files, a file changed on disk, a conflict, a worktree, in a pane', async ({
  app,
}) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  const { page } = app;
  writeProject(app.repo);
  const errors: string[] = [];
  page.on('pageerror', (e) => {
    errors.push(String(e));
    console.log(`pageerror: ${e.stack ?? e}`.slice(0, 1500));
  });
  page.on('console', (m) => {
    if (m.type() === 'error') console.log(`console.error: ${m.text().slice(0, 300)}`);
  });
  await addProject(page, app.repo, { worktrees: true });
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
  const openEditor = page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' });

  // 1-2. The project's editor is the view: its explorer, its tabs, its editor, laid over the view's place.
  await openEditor.click();
  const view = page.getByTestId('vscode-view');
  await expect(view).toBeVisible();
  const wb = page.locator('#vscode-layer .monaco-workbench');
  await expect(wb).toBeVisible({ timeout: 60_000 });
  await expect.poll(() => laidOver(view)).toBe(true);
  const explorer = wb.locator('.explorer-folders-view');
  const item = (name: string) => explorer.locator('.monaco-list-row', { hasText: new RegExp(`^${name.replaceAll('.', '\\.')}$`) }).first();
  await expect(item('src')).toBeVisible({ timeout: 30_000 });
  await item('src').click();
  await item('app.ts').click();
  await expect.poll(() => tabs(page)).toContain('app.ts');
  const lines = wb.locator('.editor-instance .monaco-editor .view-lines').first();
  await expect(lines).toContainText('export const answer = 42;');

  // A binary file, read in bytes: VS Code shows it as one, without error (in French, as the app: ESC-472).
  await item('assets').click();
  await item('logo.png').click();
  await expect(wb.getByText(/c’est un fichier binaire ou qu’il utilise un encodage de texte non pris en charge/)).toBeVisible();

  // 3. A file an agent changed on disk is read again, pushed by the core's watcher.
  await item('f1.ts').click();
  await expect.poll(() => tabs(page)).toContain('f1.ts');
  await expect(lines).toContainText('// Fichier 1');
  const f1 = path.join(app.repo, 'src', 'f1.ts');
  fs.writeFileSync(f1, '// Écrit par un agent\r\nexport function f1(x: number): number {\r\n  return x * 2;\r\n}\r\n');
  await expect(lines).toContainText('// Écrit par un agent', { timeout: 15_000 });
  await expect(lines).toContainText('return x * 2;');

  // A change saved by Ctrl+S goes to the disk through the core, its CRLF kept.
  await lines.click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('// ajouté dans VS Code');
  await expect.poll(() => tabs(page)).toContain('f1.ts*');
  await page.keyboard.press('Control+s');
  await expect.poll(() => fs.readFileSync(f1, 'utf8')).toContain('// ajouté dans VS Code');
  expect(fs.readFileSync(f1, 'utf8')).toMatch(/^\/\/ Écrit par un agent\r\n.*\r\n\/\/ ajouté dans VS Code$/s);
  await expect.poll(() => tabs(page)).toContain('f1.ts');

  // 4. A conflict: the file changes on disk while the tab is modified; Ctrl+S shows VS Code's conflict, nothing written.
  await page.keyboard.type(' (mien)');
  await expect.poll(() => tabs(page)).toContain('f1.ts*');
  fs.writeFileSync(f1, 'export const byTheAgent = true;\n');
  // Not read again over what is typed.
  await page.waitForTimeout(1500);
  await expect(lines).toContainText('(mien)');
  await page.keyboard.press('Control+s');
  const toast = wb.locator('.notifications-toasts .notification-toast', { hasText: 'le contenu du fichier est plus récent' });
  await expect(toast).toBeVisible({ timeout: 15_000 });
  expect(fs.readFileSync(f1, 'utf8')).toBe('export const byTheAgent = true;\n');
  if (SHOTS) {
    fs.mkdirSync(SHOTS, { recursive: true });
    await page.screenshot({ path: path.join(SHOTS, 'vscode-conflict.png') });
  }
  // « Remplacer » (Overwrite): mine, written over the agent's.
  await toast.getByRole('button', { name: 'Remplacer' }).click();
  await expect.poll(() => fs.readFileSync(f1, 'utf8')).toContain('(mien)');
  await expect.poll(() => tabs(page)).toContain('f1.ts');

  // 5. The agent's worktree as the source: a folder of its own in the workspace, its files the worktree's.
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  await expect.poll(() => fs.existsSync(path.join(wt, 'src', 'app.ts'))).toBe(true);
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'export const answer = 43; // dans le worktree\n');
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').nth(1).click();
  // Its folder, after the project's, named by the agent and its worktree.
  const rootRows = explorer.locator('.monaco-list-row[aria-level="1"]');
  await expect(rootRows).toHaveCount(2, { timeout: 30_000 });
  const wtRoot = rootRows.nth(1);
  await expect(wtRoot).toHaveText('agent-1 · .claude/worktrees/agent-1');
  /** Folds or unfolds a row of the explorer, until it says so (a row drawn again may lose a click). */
  const fold = (row: Locator, open: boolean) =>
    expect(async () => {
      if ((await row.getAttribute('aria-expanded')) !== String(open)) await row.click();
      await expect(row).toHaveAttribute('aria-expanded', String(open), { timeout: 2000 });
    }).toPass({ timeout: 30_000 });
  // The project's folder folded: the rows below are the worktree's alone, its own `app.ts`, not the project's.
  await fold(rootRows.nth(0), false);
  await fold(wtRoot, true);
  await fold(explorer.locator('.monaco-list-row[aria-level="2"]', { hasText: /^src$/ }), true);
  await explorer.locator('.monaco-list-row[aria-level="3"]', { hasText: /^app\.ts$/ }).click();
  await expect(lines).toContainText('dans le worktree');
  // Changed there by the agent: read again too.
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'export const answer = 44; // l’agent continue\n');
  await expect(lines).toContainText('l’agent continue', { timeout: 15_000 });
  expect(fs.readFileSync(path.join(app.repo, 'src', 'app.ts'), 'utf8')).toBe('export const answer = 42;\n');
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').first().click();

  // 6. In a pane beside the agent: the editor's chip dragged from the title bar onto the right of the work area.
  const before = await tabs(page);
  const agentKey = `agent:${await page.locator('aside.side .agent.card').first().getAttribute('data-agent')}`;
  await page.getByTestId('vscode-back').click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await expect(page.locator('#vscode-layer .vscode-host')).toHaveCSS('visibility', 'hidden');
  await drag(page, openEditor, await at(page.locator('.work').first(), 0.9, 0.5));
  await expect.poll(() => keys(page)).toEqual([agentKey, 'editor']);
  const pane = page.locator('[data-pane-key="editor"]');
  await expect.poll(() => laidOver(pane)).toBe(true);
  await expect.poll(() => tabs(page)).toEqual(before);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-pane.png'), style: '.conv .msgs { visibility: hidden }' });

  // Moved to the agent's place (the two swap): the same tabs.
  await dragTo(page, openEditor, page.locator(`[data-pane-key="${agentKey}"]`), 'center');
  await expect.poll(() => keys(page)).toEqual(['editor', agentKey]);
  await expect.poll(() => laidOver(pane)).toBe(true);
  expect(await tabs(page)).toEqual(before);

  // Resized by the handle between the two panes: the workbench follows its pane.
  const width = () => wb.evaluate((e) => Math.round(e.getBoundingClientRect().width));
  const w0 = await width();
  const handle = page.getByTestId('pane-handle').first();
  const hb = (await handle.boundingBox())!;
  await page.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2);
  await page.mouse.down();
  await page.mouse.move(hb.x + 200, hb.y + hb.height / 2, { steps: 10 });
  await page.mouse.up();
  await expect.poll(width).toBeGreaterThan(w0 + 100);
  expect(await tabs(page)).toEqual(before);

  // A press on the workbench makes its pane active (the view passes it on: the workbench is not in the pane's DOM).
  await page
    .locator(`[data-pane-key="${agentKey}"] header`)
    .first()
    .click({ position: { x: 20, y: 20 } });
  await expect(pane).toHaveAttribute('data-pane-active', 'false');
  await wb.locator('.part.editor').click({ position: { x: 200, y: 200 } });
  await expect(pane).toHaveAttribute('data-pane-active', 'true');

  // Closed by its ×, shown when the pointer is over the workbench; opened again beside: its tabs are still there.
  await wb.hover({ position: { x: 100, y: 100 } });
  await expect(pane.getByTestId('pane-close')).toHaveCSS('opacity', '1');
  await pane.getByTestId('pane-close').click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await expect(page.locator('#vscode-layer .vscode-host')).toHaveCSS('visibility', 'hidden');
  await drag(page, openEditor, await at(page.locator('.work').first(), 0.9, 0.5));
  await expect.poll(() => keys(page)).toEqual([agentKey, 'editor']);
  await expect.poll(() => laidOver(pane)).toBe(true);
  expect(await tabs(page)).toEqual(before);

  // One error of VS Code's own explorer is let through: a refresh still pending when a folder joins the workspace
  // looks for the roots' former list (`getId` of its identity provider on that list), now and then. The tree is right
  // just after; it is VS Code's race, not the files'.
  expect(errors.filter((e) => e !== 'TypeError: e.getId is not a function')).toEqual([]);
});
