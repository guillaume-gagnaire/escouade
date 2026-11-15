// The VS Code view in the app (ESC-464, A3 of docs/EDITOR-STUDY.md): the keyboard between the view and the rest of
// Escouade, the keys both have (the rule of the spec, Éditeur › Les touches), « Ouvrir un fichier » (Ctrl+P) on the
// core's index, the links of a conversation opened in the agent's worktree at their line, and the closes asked first
// when files are modified: a tab (VS Code's own question), the view, its pane, a project.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';
import { editorLines, openInExplorer, vscodeTabs, workbench } from './vscode';

test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');

const lines = (n: number, what: string) => Array.from({ length: n }, (_, i) => `// ligne ${i + 1} ${what}`).join('\n') + '\n';

function writeProject(repo: string) {
  const write = (file: string, data: string) => {
    fs.mkdirSync(path.dirname(path.join(repo, file)), { recursive: true });
    fs.writeFileSync(path.join(repo, file), data);
  };
  write('src/app.ts', lines(12, 'du projet'));
  write('src/f1.ts', 'export function f1(x: number): number {\n  const y = x;\n  return y;\n}\n');
  const git = (...args: string[]) => execFileSync('git', args, { cwd: repo });
  git('add', '-A');
  git('commit', '-qm', 'Projet de test');
}

/** Where the keyboard is: in the VS Code view, in the message field of a conversation, or elsewhere. */
const keyboardIn = (page: Page) =>
  page.evaluate(() => {
    const el = document.activeElement;
    if (el?.closest('.vscode-host')) return 'view';
    if (el?.matches('.composer textarea')) return 'composer';
    return el?.tagName.toLowerCase() ?? 'none';
  });

/** The line the cursor is on, as the editor numbers it. */
const cursorLine = (page: Page) => workbench(page).locator('.editor-instance .line-numbers.active-line-number').first();

/** Escouade's dialog of that name. */
const dialog = (page: Page, name: string | RegExp) => page.getByRole('dialog', { name });

/** Lets a key VS Code might take be taken, if it does: nothing to wait for when it does not. */
const settle = (page: Page) => page.waitForTimeout(400);

function track(page: Page) {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  // VS Code's own race of its explorer, let through by the view's e2e too (vscode-view.spec.ts).
  return () => expect(errors.filter((e) => e !== 'TypeError: e.getId is not a function')).toEqual([]);
}

test('the keyboard in the VS Code view: the keys of each, Ctrl+P on the core’s index, in and out of its pane', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const noErrors = track(page);
  writeProject(app.repo);
  await addProject(page, app.repo);
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  // Shown, the view takes the keyboard.
  await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
  await expect.poll(() => keyboardIn(page)).toBe('view');
  await openInExplorer(page, 'src/app.ts');
  await editorLines(page).click();
  await expect.poll(() => keyboardIn(page)).toBe('view');

  // Ctrl+P is Escouade's « Ouvrir un fichier », on the core's index: VS Code's own quick open does not open.
  await page.keyboard.press('Control+P');
  const palette = dialog(page, 'Ouvrir un fichier');
  await expect(palette).toBeVisible();
  await expect(workbench(page).locator('.quick-input-widget')).toBeHidden();
  await page.keyboard.type('f1.ts:3');
  await expect(palette.getByRole('option', { name: /f1\.ts/ })).toBeVisible();
  await page.keyboard.press('Enter');
  await expect(palette).toBeHidden();
  // In a tab of the view, at its line, the keyboard in it.
  await expect.poll(() => vscodeTabs(page)).toContain('f1.ts');
  await expect(cursorLine(page)).toHaveText('3');
  await expect.poll(() => keyboardIn(page)).toBe('view');

  // Ctrl+Shift+P is VS Code's palette (it prints nothing).
  await page.keyboard.press('Control+Shift+P');
  const quick = workbench(page).locator('.quick-input-widget');
  await expect(quick).toBeVisible();
  await expect(quick.locator('input')).toHaveValue('>');
  // A key the app keeps is no longer VS Code's: its palette runs the command by its name, without showing Ctrl+J.
  // In French, as the app (ESC-472): its commands are found by their English names too.
  await page.keyboard.type('Toggle Panel Visibility');
  const row = quick.locator('.monaco-list-row', { hasText: 'Activer/désactiver la visibilité du panneau' }).first();
  await expect(row).toBeVisible();
  await expect(row).not.toContainText('Ctrl+J');
  // Ctrl+B, kept by VS Code, still shows on its command.
  await quick.locator('input').fill('>Toggle Primary Side Bar Visibility');
  await expect(quick.locator('.monaco-list-row').first()).toContainText('Ctrl+B');
  await page.keyboard.press('Escape');
  await expect(quick).toBeHidden();

  // Ctrl+K is VS Code's, with its chords: Ctrl+K Ctrl+C comments the line; the search of the conversations does not open.
  await page.keyboard.press('Control+K');
  await page.keyboard.press('Control+C');
  await expect(editorLines(page)).toContainText('// return y;');
  await expect(dialog(page, 'Rechercher dans les conversations')).toHaveCount(0);

  // Ctrl+F is VS Code's search in the file.
  await page.keyboard.press('Control+F');
  await expect(workbench(page).locator('.find-widget.visible')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(workbench(page).locator('.find-widget.visible')).toHaveCount(0);

  // Ctrl+, is Escouade's settings; closed, the keyboard is back in the view where it was.
  await page.keyboard.press('Control+,');
  const settings = dialog(page, 'Réglages');
  await expect(settings).toBeVisible();
  await settle(page);
  await expect(workbench(page).locator('.settings-editor')).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();
  await expect.poll(() => keyboardIn(page)).toBe('view');

  // Ctrl+\ is Escouade's split screen: the agent beside, which takes the keyboard.
  await page.keyboard.press('Control+\\');
  await expect(page.locator('[data-pane-key="editor"]')).toBeVisible();
  await expect.poll(() => keyboardIn(page)).toBe('composer');
  // Ctrl+Alt+← back to the view's pane: the keyboard in the view, where what is typed goes.
  await page.keyboard.press('Control+Alt+ArrowLeft');
  await expect(page.locator('[data-pane-key="editor"]')).toHaveAttribute('data-pane-active', 'true');
  await expect.poll(() => keyboardIn(page)).toBe('view');
  await page.keyboard.type('x');
  await expect.poll(() => vscodeTabs(page)).toContain('f1.ts*');
  // And Ctrl+Alt+→ out of it again: the agent's message field, nothing typed in the view.
  await page.keyboard.press('Control+Alt+ArrowRight');
  await expect.poll(() => keyboardIn(page)).toBe('composer');
  await page.keyboard.type('pour l’agent');
  await expect(page.locator('.composer textarea').last()).toHaveValue('pour l’agent');
  await page.locator('.composer textarea').last().fill('');

  // Ctrl+Shift+W closes the active pane, the view's: its file modified is asked for first. « Annuler »: nothing closes.
  await page.keyboard.press('Control+Alt+ArrowLeft');
  await expect.poll(() => keyboardIn(page)).toBe('view');
  await page.keyboard.press('Control+Shift+W');
  const ask = dialog(page, 'Enregistrer les modifications de « f1.ts » ?');
  await expect(ask).toBeVisible();
  for (const choice of ['Annuler', 'Ne pas enregistrer', 'Enregistrer'])
    await expect(ask.getByRole('button', { name: choice, exact: true })).toBeVisible();
  await ask.getByRole('button', { name: 'Annuler' }).click();
  await expect(ask).toBeHidden();
  await expect(page.locator('[data-pane-key="editor"]')).toBeVisible();
  // « Ne pas enregistrer »: the file read again from the disk, the pane closed; the keyboard not left in the hidden view.
  await page.keyboard.press('Control+Shift+W');
  await ask.getByRole('button', { name: 'Ne pas enregistrer' }).click();
  await expect(page.locator('[data-pane-key="editor"]')).toHaveCount(0);
  await expect(page.locator('#vscode-layer .vscode-host')).toHaveCSS('visibility', 'hidden');
  expect(await keyboardIn(page)).not.toBe('view');
  expect(fs.readFileSync(path.join(app.repo, 'src', 'f1.ts'), 'utf8')).toBe(
    'export function f1(x: number): number {\n  const y = x;\n  return y;\n}\n',
  );
  // Opened again: the same tabs, nothing left modified.
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  await expect.poll(async () => (await vscodeTabs(page)).sort()).toEqual(['app.ts', 'f1.ts']);
  noErrors();
});

test('the closes asked when files are modified: a tab, the view, a project', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const noErrors = track(page);
  writeProject(app.repo);
  await addProject(page, app.repo);
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
  const f1 = path.join(app.repo, 'src', 'f1.ts');
  const before = fs.readFileSync(f1, 'utf8');
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  await openInExplorer(page, 'src/f1.ts');

  // A tab: VS Code's own question, « Ne pas enregistrer » gives up, « Enregistrer » writes through the core.
  await editorLines(page).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('// un');
  await expect.poll(() => vscodeTabs(page)).toContain('f1.ts*');
  await page.keyboard.press('Control+F4');
  const vsAsk = workbench(page).locator('.monaco-dialog-box');
  await expect(vsAsk).toBeVisible();
  await expect(vsAsk).toContainText('Voulez-vous enregistrer les modifications apportées à f1.ts ?');
  await vsAsk.getByRole('button', { name: 'Ne pas enregistrer' }).click();
  await expect.poll(() => vscodeTabs(page)).not.toContain('f1.ts');
  expect(fs.readFileSync(f1, 'utf8')).toBe(before);
  await openInExplorer(page, 'src/f1.ts');
  await editorLines(page).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('// deux');
  await page.keyboard.press('Control+F4');
  await vsAsk.getByRole('button', { name: 'Enregistrer', exact: true }).click();
  await expect.poll(() => fs.readFileSync(f1, 'utf8')).toContain('// deux');

  // The view, by « Conversation »: asked, « Annuler » keeps it; « Enregistrer » saves, closes, and the keyboard goes to
  // the conversation.
  await openInExplorer(page, 'src/f1.ts');
  await editorLines(page).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type(' trois');
  await expect.poll(() => vscodeTabs(page)).toContain('f1.ts*');
  await page.getByTestId('vscode-back').click();
  const ask = dialog(page, 'Enregistrer les modifications de « f1.ts » ?');
  await expect(ask).toBeVisible();
  await ask.getByRole('button', { name: 'Annuler' }).click();
  await expect(page.getByTestId('vscode-view')).toBeVisible();
  await page.getByTestId('vscode-back').click();
  await ask.getByRole('button', { name: 'Enregistrer', exact: true }).click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await expect.poll(() => fs.readFileSync(f1, 'utf8')).toContain('// deux trois');
  await expect.poll(() => keyboardIn(page)).toBe('composer');

  // Without a file modified: closed at once.
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  await expect(page.getByTestId('vscode-view')).toBeVisible();
  await page.getByTestId('vscode-back').click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await expect(dialog(page, /Enregistrer les modifications/)).toHaveCount(0);

  // A project: once its close is confirmed, its files modified in the view are asked for. « Annuler »: it stays.
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  await editorLines(page).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type(' quatre');
  await expect.poll(() => vscodeTabs(page)).toContain('f1.ts*');
  const name = path.basename(app.repo);
  const tab = page.getByRole('tab', { name: new RegExp(`^${name}`) });
  const closeIt = async () => {
    await tab.click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Fermer le projet…' }).click();
    await dialog(page, `Fermer « ${name} » ?`).getByRole('button', { name: 'Fermer le projet', exact: true }).click();
    await expect(ask).toBeVisible();
  };
  await closeIt();
  await ask.getByRole('button', { name: 'Annuler' }).click();
  await expect(tab).toHaveCount(1);
  // « Ne pas enregistrer »: given up, the project closed, out of the view.
  await closeIt();
  await ask.getByRole('button', { name: 'Ne pas enregistrer' }).click();
  await expect(tab).toHaveCount(0);
  expect(fs.readFileSync(f1, 'utf8')).not.toContain('quatre');
  noErrors();
});

test('a link of a conversation opens in the agent’s worktree, at its line, in the view', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const noErrors = track(page);
  writeProject(app.repo);
  await addProject(page, app.repo, { worktrees: true });
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  await expect.poll(() => fs.existsSync(path.join(wt, 'src', 'app.ts'))).toBe(true);
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), lines(40, 'du worktree'));

  // The agent edits its `src/app.ts` at line 33: the link of its tool opens it there, the worktree's, not the project's.
  await send(page, 'edit ligne 33');
  const link = page.getByRole('button', { name: /^Ouvrir .*app\.ts dans l’éditeur$/ }).first();
  await expect(link).toBeVisible({ timeout: 30_000 });
  await link.click();
  await expect(page.getByTestId('vscode-view')).toBeVisible();
  await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
  await expect.poll(() => vscodeTabs(page)).toContain('app.ts');
  await expect(editorLines(page)).toContainText('// ligne 33 du worktree');
  await expect(cursorLine(page)).toHaveText('33');
  await expect.poll(() => keyboardIn(page)).toBe('view');
  await expect(page.getByTestId('vscode-view').locator('.vscode-bar .src')).toContainText('worktree');

  // Asked again, the same file at the same line once the cursor moved: back to the line.
  await page.keyboard.press('Control+Home');
  await expect(cursorLine(page)).toHaveText('1');
  await page.getByTestId('vscode-back').click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await link.click();
  await expect(cursorLine(page)).toHaveText('33');
  noErrors();
});
