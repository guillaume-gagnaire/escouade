// ESC-444 (study, never merged): the VS Code view (`@codingame/monaco-vscode-api`) in place of CodeMirror, behind the
// development setting `escouade.dev.vscodeEditor`. On the files of a project by the core, with a language server the
// core starts (typescript-language-server), Escouade's theme, extensions of Open VSX; then in a pane beside an agent,
// moved, resized, closed and opened again without losing its tabs. ESCOUADE_E2E_SHOTS=<folder>: the captures.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

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

async function at(el: Locator, fx: number, fy: number) {
  const b = (await el.boundingBox())!;
  return { x: b.x + b.width * fx, y: b.y + b.height * fy };
}

/** Whether the workbench is shown, laid over the place of `el` (a view, or a pane that holds one), to a pixel. */
const laidOver = (page: Page, el: Locator) =>
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

/** The labels of the editor tabs of the view. */
const tabs = (page: Page) =>
  page.locator('.monaco-workbench .tabs-container .tab .label-name').evaluateAll((els) => els.map((e) => e.textContent?.trim()));

/** Runs `f` with the view's extension API (`vscode`) in the page. */
function vs<T>(page: Page, f: string): Promise<T> {
  return page.evaluate(`(async () => { const { vscode } = globalThis.__escouadeVscode; ${f} })()`) as Promise<T>;
}

function writeProject(repo: string) {
  fs.writeFileSync(path.join(repo, 'package.json'), JSON.stringify({ name: 'demo-api', private: true }, null, 2));
  fs.writeFileSync(
    path.join(repo, 'tsconfig.json'),
    JSON.stringify({ compilerOptions: { strict: true, target: 'es2022', module: 'esnext', noEmit: true }, include: ['src'] }, null, 2),
  );
  fs.writeFileSync(path.join(repo, 'src', 'app.ts'), "export const a: number = 'pas un nombre';\n");
  for (let i = 1; i <= 10; i++)
    fs.writeFileSync(
      path.join(repo, 'src', `f${i}.ts`),
      `// Fichier ${i}\r\nexport function f${i}(x: number): number {\r\n  return x * ${i};\r\n}\r\n`,
    );
  fs.writeFileSync(path.join(repo, 'config.toml'), '[server]\nport = 8080\n');
  execFileSync('git', ['add', '-A'], { cwd: repo });
  execFileSync('git', ['commit', '-qm', 'Projet de test'], { cwd: repo });
}

test.use({
  appEnv: {
    ESCOUADE_STUDY_TS_LSP: path.resolve(import.meta.dirname, '..', '..', 'node_modules', '.bin', 'typescript-language-server.cmd'),
  },
});

test('the VS Code view: files by the core, a language server, the theme, Open VSX, in a pane that keeps its tabs', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(300_000);
  writeProject(app.repo);
  const errors: string[] = [];
  app.page.on('pageerror', (e) => errors.push(String(e)));
  await addProject(app.page, app.repo);
  // The development setting, then the window loaded again: the view's services start once per page.
  await app.page.evaluate(() => localStorage.setItem('escouade.dev.vscodeEditor', '1'));
  await app.page.reload();
  const page = app.page;
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') console.log(`ESC-444 console.${m.type()}: ${m.text().slice(0, 400)}`);
  });
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });

  // How many times the extension host's iframe started: once, whatever the panes do (an iframe moved in the page loads again).
  await page.evaluate(() =>
    addEventListener('message', (e) => {
      const d = e.data as { type?: string } | undefined;
      const w = window as unknown as { esc444Boots?: number };
      if (d && typeof d === 'object' && 'vscodeWebWorkerExtHostId' in d && d.type === 'vscode.bootstrap.nls')
        w.esc444Boots = (w.esc444Boots ?? 0) + 1;
    }),
  );
  // 1. The view in place of CodeMirror: the explorer, the tabs, the editor.
  const t0 = Date.now();
  await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
  const view = page.getByTestId('vscode-view');
  await expect(view).toBeVisible();
  // The workbench lives in a layer of its own, laid over the view's place.
  const wb = page.locator('#vscode-layer .monaco-workbench');
  await expect(wb).toBeVisible({ timeout: 60_000 });
  await expect.poll(() => laidOver(page, view)).toBe(true);
  await expect(page.locator('main.editor .cm-editor')).toHaveCount(0);
  const explorer = wb.locator('.explorer-folders-view');
  await expect(explorer.getByText('src', { exact: true })).toBeVisible({ timeout: 30_000 });
  console.log(`ESC-444 open: ${Date.now() - t0} ms; marks ${JSON.stringify(await page.evaluate('globalThis.__escouadeVscode.marks'))}`);
  await explorer.getByText('src', { exact: true }).click();
  await explorer.getByText('app.ts', { exact: true }).click();
  await expect.poll(() => tabs(page)).toContain('app.ts');

  // The file was read by the core: its text, from the project's folder.
  const editor = wb.locator('.editor-instance .monaco-editor').first();
  await expect(editor.locator('.view-lines')).toContainText('pas un nombre');

  // 2. The language server, started by the core: its error under the text and in the problems.
  await expect(editor.locator('.squiggly-error').first()).toBeVisible({ timeout: 90_000 });
  const diags = await vs<string[]>(
    page,
    "return vscode.languages.getDiagnostics().flatMap(([u, ds]) => ds.map((d) => u.path.split('/').pop() + ': ' + d.message));",
  );
  console.log('ESC-444 diagnostics:', diags);
  expect(diags.join('\n')).toMatch(/app\.ts: .*string.*number/i);

  // A change saved by Ctrl+S goes to the disk through the core, its line endings kept (CRLF for f1.ts).
  await explorer.getByText('f1.ts', { exact: true }).click();
  await expect.poll(() => tabs(page)).toContain('f1.ts');
  const f1 = wb.locator('.editor-instance .monaco-editor').first();
  await f1.locator('.view-lines').click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('// ajouté dans VS Code');
  await page.keyboard.press('Control+s');
  await expect.poll(() => fs.readFileSync(path.join(app.repo, 'src', 'f1.ts'), 'utf8')).toContain('// ajouté dans VS Code');
  expect(fs.readFileSync(path.join(app.repo, 'src', 'f1.ts'), 'utf8')).toMatch(/\r\n\/\/ ajouté/);

  // 3. Escouade's theme: the editor and the tree are the app's panel and frame.
  const colors = await page.evaluate(() => {
    const probe = document.createElement('span');
    document.body.appendChild(probe);
    const of = (v: string) => {
      probe.style.backgroundColor = `var(${v})`;
      return getComputedStyle(probe).backgroundColor;
    };
    const ed = document.querySelector('.monaco-workbench .part.editor .monaco-editor-background');
    const theme = document.querySelector('.monaco-workbench')!.classList;
    return {
      themeClass: [...theme].join(' '),
      editor: ed ? getComputedStyle(ed).backgroundColor : null,
      panel: of('--panel'),
    };
  });
  console.log('ESC-444 theme:', colors);
  expect(await vs<string>(page, "return vscode.workspace.getConfiguration('workbench').get('colorTheme');")).toBe('Escouade');
  if (SHOTS) {
    fs.mkdirSync(SHOTS, { recursive: true });
    await page.screenshot({ path: path.join(SHOTS, 'vscode-alone.png') });
  }

  // 4. Extensions of Open VSX: a theme and a language, installed from the gallery, run in the page.
  // VS Code asks first whether the publisher is trusted (its dialog, in the workbench): yes.
  const install = async (id: string) => {
    await page.evaluate(`(() => {
      const { vscode } = globalThis.__escouadeVscode;
      globalThis.esc444Install = globalThis.esc444Install ?? {};
      globalThis.esc444Install['${id}'] = 'pending';
      vscode.commands.executeCommand('workbench.extensions.installExtension', '${id}').then(
        () => (globalThis.esc444Install['${id}'] = 'ok'),
        (e) => (globalThis.esc444Install['${id}'] = 'error: ' + e),
      );
    })()`);
    const state = () => page.evaluate(`globalThis.esc444Install['${id}']`) as Promise<string>;
    const dialog = wb.locator('.monaco-dialog-box');
    const deadline = Date.now() + 90_000;
    let asked = '';
    while (Date.now() < deadline && (await state()) === 'pending') {
      if (await dialog.isVisible()) {
        asked = (await dialog.innerText()).replace(/\s+/g, ' ').slice(0, 300);
        if (SHOTS && !fs.existsSync(path.join(SHOTS, 'vscode-trust.png')))
          await page.screenshot({ path: path.join(SHOTS, 'vscode-trust.png') });
        await dialog.getByRole('button').first().click();
      }
      await page.waitForTimeout(250);
    }
    const s = await state();
    console.log(`ESC-444 install ${id}: ${s}${asked ? ` (asked: ${asked})` : ''}`);
    return s;
  };
  for (const id of ['dracula-theme.theme-dracula', 'tamasfe.even-better-toml']) expect(await install(id)).toBe('ok');
  type Ext = { id: string; activated: boolean; errors: string[]; browser: boolean; node: boolean };
  const exts = () => page.evaluate('globalThis.__escouadeVscode.extensions()') as Promise<Ext[]>;
  await expect
    .poll(async () => (await exts()).map((e) => e.id), { timeout: 60_000 })
    .toEqual(expect.arrayContaining(['dracula-theme.theme-dracula', 'tamasfe.even-better-toml']));
  // The theme of the extension is one of the workbench's, and can be put on; then Escouade's again.
  expect(await page.evaluate('globalThis.__escouadeVscode.colorThemes()')).toEqual(expect.arrayContaining(['Dracula Theme']));
  const editorBg = () =>
    wb
      .locator('.part.editor .monaco-editor-background')
      .first()
      .evaluate((e) => getComputedStyle(e).backgroundColor);
  const ours = await editorBg();
  await vs(page, "await vscode.workspace.getConfiguration('workbench').update('colorTheme', 'Dracula Theme', 1);");
  await expect.poll(editorBg).not.toBe(ours);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-dracula.png') });
  await vs(page, "await vscode.workspace.getConfiguration('workbench').update('colorTheme', 'Escouade', 1);");
  await expect.poll(editorBg).toBe(ours);
  // Even Better TOML: its language is known, but its server (a worker of 35 MB) is not served by Open VSX: logged.
  await explorer.getByText('config.toml', { exact: true }).click();
  await expect
    .poll(() => vs<string>(page, 'return vscode.window.activeTextEditor?.document.languageId ?? null;'), { timeout: 30_000 })
    .toBe('toml');
  await page.waitForTimeout(3000);
  console.log('ESC-444 extensions:', JSON.stringify((await exts()).filter((e) => !e.builtin)));

  // A list of the extensions allowed (`extensions.allowed`, a setting a team could impose): another one is refused (VS
  // Code's message says « not available for the Web platform »), and installs once the list is lifted.
  await vs(
    page,
    "await vscode.workspace.getConfiguration('extensions').update('allowed', { 'dracula-theme': true, vscodevim: true, tamasfe: true }, 1);",
  );
  expect(await install('enkia.tokyo-night')).toMatch(/^error/);
  expect((await exts()).map((e) => e.id)).not.toContain('enkia.tokyo-night');
  await vs(page, "await vscode.workspace.getConfiguration('extensions').update('allowed', undefined, 1);");
  expect(await install('enkia.tokyo-night')).toBe('ok');
  await vs(page, "await vscode.commands.executeCommand('workbench.action.closeAllEditors');");
  await explorer.getByText('app.ts', { exact: true }).click();
  await explorer.getByText('f2.ts', { exact: true }).click();
  await expect.poll(() => tabs(page)).toEqual(['app.ts', 'f2.ts']);

  // 5. In a pane beside the agent: the chip of the editor dragged from the title bar onto the right of the work area.
  const agentKey = `agent:${await page.locator('aside.side .agent.card').first().getAttribute('data-agent')}`;
  await page.getByTestId('vscode-back').click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await expect(page.locator('#vscode-layer .vscode-host')).toHaveCSS('visibility', 'hidden');
  const before = await tabs(page);
  await drag(page, page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }), await at(page.locator('.work').first(), 0.9, 0.5));
  await expect.poll(() => keys(page)).toEqual([agentKey, 'editor']);
  const pane = page.locator('[data-pane-key="editor"]');
  await expect.poll(() => laidOver(page, pane)).toBe(true);
  await expect.poll(() => tabs(page)).toEqual(before);
  const width = () => wb.evaluate((e) => Math.round(e.getBoundingClientRect().width));
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-pane.png') });

  // Moved to the agent's place (the two swap: the compass has no room for a third on that side): the same tabs.
  await dragTo(
    page,
    page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }),
    page.locator(`[data-pane-key="${agentKey}"]`),
    'center',
  );
  await expect.poll(() => keys(page)).toEqual(['editor', agentKey]);
  await expect.poll(() => laidOver(page, pane)).toBe(true);
  expect(await tabs(page)).toEqual(before);

  // Resized by the handle between the two panes: the workbench follows its pane.
  const w0 = await width();
  const handle = page.getByTestId('pane-handle').first();
  const hb = (await handle.boundingBox())!;
  await page.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2);
  await page.mouse.down();
  await page.mouse.move(hb.x + 200, hb.y + hb.height / 2, { steps: 10 });
  await page.mouse.up();
  await expect.poll(width).toBeGreaterThan(w0 + 100);
  expect(await tabs(page)).toEqual(before);

  // A press on the workbench makes its pane active, as anywhere in a pane (the workbench is not in the pane's part
  // of the page: the view passes the press on).
  await page
    .locator(`[data-pane-key="${agentKey}"] header`)
    .first()
    .click({ position: { x: 20, y: 20 } });
  await expect(pane).toHaveAttribute('data-pane-active', 'false');
  await wb.locator('.part.editor').click({ position: { x: 200, y: 200 } });
  await expect(pane).toHaveAttribute('data-pane-active', 'true');
  // Closed by its ×, shown when the pointer is over the workbench; then opened again beside: its tabs are still there.
  await wb.hover({ position: { x: 100, y: 100 } });
  await expect(pane.getByTestId('pane-close')).toHaveCSS('opacity', '1');
  await pane.getByTestId('pane-close').click();
  await expect(page.getByTestId('vscode-view')).toHaveCount(0);
  await expect(page.locator('#vscode-layer .vscode-host')).toHaveCSS('visibility', 'hidden');
  await drag(page, page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }), await at(page.locator('.work').first(), 0.9, 0.5));
  await expect.poll(() => keys(page)).toEqual([agentKey, 'editor']);
  await expect.poll(() => laidOver(page, pane)).toBe(true);
  expect(await tabs(page)).toEqual(before);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-pane-back.png') });

  // Several columns of the editor. In the pane, VS Code's own groups of tabs side by side.
  await vs(page, "await vscode.commands.executeCommand('workbench.action.splitEditorRight');");
  await expect(wb.locator('.part.editor .editor-group-container')).toHaveCount(2);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-pane-split.png') });
  await vs(page, "await vscode.commands.executeCommand('workbench.action.joinAllGroups');");
  await expect(wb.locator('.part.editor .editor-group-container')).toHaveCount(1);
  // Elsewhere in the page (another column), a code editor of its own on the same model: the same text, the same
  // diagnostics, a change typed there is the tab's.
  await page.evaluate(async () => {
    const el = document.createElement('div');
    el.id = 'esc444-second';
    el.style.cssText = 'position:fixed;left:380px;top:120px;width:420px;height:240px;z-index:10;';
    document.body.appendChild(el);
    const s = (globalThis as unknown as { __escouadeVscode: { standaloneEditor(rel: string, el: HTMLElement): Promise<() => void> } })
      .__escouadeVscode;
    await s.standaloneEditor('src/app.ts', el);
  });
  const second = page.locator('#esc444-second');
  await expect(second.locator('.view-lines')).toContainText('pas un nombre');
  await expect(second.locator('.squiggly-error').first()).toBeVisible({ timeout: 30_000 });
  await second.locator('.view-lines').click();
  await page.keyboard.press('Control+Home');
  await page.keyboard.type('// colonne 2');
  await page.keyboard.press('Enter');
  await expect(wb.locator('.tabs-container .tab.dirty .label-name')).toContainText(['app.ts']);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-second-editor.png') });
  await vs(page, "await vscode.commands.executeCommand('workbench.action.files.revert');");

  // Vim, a web extension, last (its keys would take what is typed above, in the second editor too): its code runs in
  // the page's extension host (a web worker), activated at the start.
  expect(await install('vscodevim.vim')).toBe('ok');
  await expect.poll(async () => (await exts()).find((e) => e.id === 'vscodevim.vim')?.activated, { timeout: 60_000 }).toBe(true);
  expect((await exts()).find((e) => e.id === 'vscodevim.vim')?.errors).toEqual([]);

  // The extension host never started again: nothing of the workbench was moved in the page.
  expect(await page.evaluate(() => (window as unknown as { esc444Boots?: number }).esc444Boots)).toBe(1);
  console.log('ESC-444 page errors:', errors);
});

// The Node extension host of VS Code (its server, `ESCOUADE_STUDY_VSCODE_SERVER`), started by the core with the system's
// Node on the loopback with a token, and extensions of Open VSX that need Node: Code Spell Checker, ESLint (with the
// project's own eslint). Skipped without the server.
test.describe('with the Node extension host', () => {
  const server = process.env.ESCOUADE_STUDY_VSCODE_SERVER;
  test.use({
    appEnv: {
      ESCOUADE_STUDY_TS_LSP: path.resolve(import.meta.dirname, '..', '..', 'node_modules', '.bin', 'typescript-language-server.cmd'),
      ...(server ? { ESCOUADE_STUDY_VSCODE_SERVER: server } : {}),
      ...(process.env.ESCOUADE_STUDY_NODE ? { ESCOUADE_STUDY_NODE: process.env.ESCOUADE_STUDY_NODE } : {}),
      ...(process.env.ESCOUADE_STUDY_HOST_PORT ? { ESCOUADE_STUDY_HOST_PORT: process.env.ESCOUADE_STUDY_HOST_PORT } : {}),
    },
  });

  test('extensions that need Node run in VS Code’s server, with the system’s Node', async ({ app }) => {
    test.skip(!server, 'ESCOUADE_STUDY_VSCODE_SERVER: the folder of the VS Code server of the same commit');
    test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
    test.setTimeout(400_000);
    writeProject(app.repo);
    // ESLint of the project, as a team would have it: the extension uses the project's own.
    fs.writeFileSync(
      path.join(app.repo, 'eslint.config.mjs'),
      "export default [{ files: ['**/*.js'], rules: { 'no-unused-vars': 'error' } }];\n",
    );
    fs.writeFileSync(path.join(app.repo, 'src', 'lint.js'), 'const inutile = 1;\n');
    fs.writeFileSync(path.join(app.repo, 'src', 'notes.md'), '# Notes\n\nCe fichier a une fautte : recieve.\n');
    execFileSync('npm', ['install', '--no-audit', '--no-fund', '-D', 'eslint@9'], { cwd: app.repo, shell: true, stdio: 'pipe' });
    fs.writeFileSync(path.join(app.repo, '.gitignore'), 'node_modules/\n');
    execFileSync('git', ['add', '-A'], { cwd: app.repo });
    execFileSync('git', ['commit', '-qm', 'ESLint'], { cwd: app.repo });

    await addProject(app.page, app.repo);
    await app.page.evaluate(() => {
      localStorage.setItem('escouade.dev.vscodeEditor', '1');
      localStorage.setItem('escouade.dev.vscodeNodeHost', '1');
    });
    await app.page.reload();
    const page = app.page;
    page.on('console', (m) => {
      if (m.type() === 'error') console.log(`ESC-444 node console.error: ${m.text().slice(0, 400)}`);
    });
    await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
    await page.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }).click();
    const wb = page.locator('#vscode-layer .monaco-workbench');
    await expect(wb).toBeVisible({ timeout: 90_000 });
    const host = (await page.evaluate('globalThis.__escouadeVscode.nodeHost')) as { authority: string; node: string } | null;
    console.log('ESC-444 node host:', host, 'marks', JSON.stringify(await page.evaluate('globalThis.__escouadeVscode.marks')));
    expect(host?.authority).toMatch(/^127\.0\.0\.1:\d+$/);
    // The server answers only with its token (its /version is public).
    const port = host!.authority.split(':')[1];
    expect((await fetch(`http://127.0.0.1:${port}/`)).status).toBe(403);

    const explorer = wb.locator('.explorer-folders-view');
    await expect(explorer.getByText('src', { exact: true })).toBeVisible({ timeout: 30_000 });
    const install = async (id: string) => {
      await page.evaluate(`(() => {
        const { vscode } = globalThis.__escouadeVscode;
        globalThis.esc444Install = globalThis.esc444Install ?? {};
        globalThis.esc444Install['${id}'] = 'pending';
        vscode.commands.executeCommand('workbench.extensions.installExtension', '${id}').then(
          () => (globalThis.esc444Install['${id}'] = 'ok'),
          (e) => (globalThis.esc444Install['${id}'] = 'error: ' + e),
        );
      })()`);
      const state = () => page.evaluate(`globalThis.esc444Install['${id}']`) as Promise<string>;
      const dialog = wb.locator('.monaco-dialog-box');
      const deadline = Date.now() + 120_000;
      while (Date.now() < deadline && (await state()) === 'pending') {
        if (await dialog.isVisible()) await dialog.getByRole('button').first().click();
        await page.waitForTimeout(250);
      }
      const s = await state();
      console.log(`ESC-444 node install ${id}: ${s}`);
      return s;
    };
    const t0 = Date.now();
    expect(await install('streetsidesoftware.code-spell-checker')).toBe('ok');
    console.log(`ESC-444 node install cspell took ${Date.now() - t0} ms`);
    expect(await install('dbaeumer.vscode-eslint')).toBe('ok');
    type Ext = { id: string; activated: boolean; errors: string[] };
    const exts = () => page.evaluate('globalThis.__escouadeVscode.extensions()') as Promise<Ext[]>;
    const diags = () =>
      page.evaluate(
        "(async () => globalThis.__escouadeVscode.vscode.languages.getDiagnostics().flatMap(([u, ds]) => ds.map((d) => u.path.split('/').pop() + ' [' + (d.source ?? '') + '] ' + d.message)))()",
      ) as Promise<string[]>;

    await explorer.getByText('src', { exact: true }).click();
    await explorer.getByText('notes.md', { exact: true }).click();
    await expect.poll(async () => (await diags()).join('\n'), { timeout: 90_000 }).toMatch(/notes\.md \[cSpell\].*recieve/i);
    await explorer.getByText('lint.js', { exact: true }).click();
    await expect.poll(async () => (await diags()).join('\n'), { timeout: 120_000 }).toMatch(/lint\.js \[eslint\].*inutile/i);
    console.log('ESC-444 node diagnostics:', JSON.stringify(await diags()));
    // The files are still the core's: the workbench read them by its commands, not by the server.
    const reads = (await page.evaluate("globalThis.__escouadeVscode.coreTimings().filter((t) => t.op === 'read').length")) as number;
    console.log('ESC-444 node reads by the core:', reads);
    expect(reads).toBeGreaterThan(0);
    console.log('ESC-444 node extensions:', JSON.stringify((await exts()).filter((e) => !e.id.startsWith('vscode.'))));
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'vscode-node-host.png') });
  });
});
