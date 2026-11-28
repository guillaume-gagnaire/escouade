// The folders of the VS Code view's workspace (ESC-481): the sources shown are the reference, brought to the workspace
// the explorer shows until it shows them. A fault injected by the page's test key (`escouade.test.vscodeFolders`: the
// first reads of the root being added answer late that it is not a folder, which VS Code leaves out of the workspace
// without a word) stands for a machine under load: the worktree picked right after the view opened still comes, and
// nothing is said in the console. ESCOUADE_E2E_FOLDER_DROPS: how many reads answer so (3) ; ESCOUADE_E2E_FOLDER_DELAY: after how long (1500 ms).

import { expect, test, addProject } from './fixture';
import { workbench } from './vscode';

const FAULT = {
  drop: Number(process.env.ESCOUADE_E2E_FOLDER_DROPS ?? 3),
  delay: Number(process.env.ESCOUADE_E2E_FOLDER_DELAY ?? 1500),
};

test('a worktree picked right after the view opened comes to the explorer, even when its first adds are refused', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(180_000);
  const { page } = app;
  const errors: string[] = [];
  page.on('console', (m) => {
    if (m.type() === 'error' && m.text().includes('[vscode] workspace')) {
      errors.push(m.text());
      console.log(m.text());
    }
  });
  await addProject(page, app.repo, { worktrees: true });
  await expect(page.locator('aside.side .agent')).toHaveCount(1, { timeout: 30_000 });
  await page.evaluate(([key, f]) => localStorage.setItem(key as string, JSON.stringify(f)), ['escouade.test.vscodeFolders', FAULT]);
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  const view = page.getByTestId('vscode-view');
  const wb = workbench(page);
  await expect(wb).toBeVisible({ timeout: 60_000 });
  const rootRows = wb.locator('.explorer-folders-view .monaco-list-row[aria-level="1"]');
  await expect(rootRows).toHaveCount(1, { timeout: 30_000 });
  // At once, while the workbench settles.
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').nth(1).click();
  await expect(view.locator('.vscode-bar .src')).toContainText('agent-1');
  await expect(rootRows).toHaveCount(2, { timeout: 30_000 });
  await expect(rootRows.nth(1)).toContainText('agent-1');
  // Every refusal injected was met, and none was left unanswered.
  expect(await page.evaluate((key) => JSON.parse(localStorage.getItem(key) ?? '{}').drop, 'escouade.test.vscodeFolders')).toBe(0);
  expect(errors).toEqual([]);
  await page.evaluate((key) => localStorage.removeItem(key), 'escouade.test.vscodeFolders');
});

// The first agent of a project is made after its window closes, then selected; selected with the editor open, it takes
// the editor to its worktree. Its answer held back here (as a loaded machine is slow to give it), `addProject` gives the
// hand back once it is selected: the view opened then starts on the project, and the worktree picked is a second folder.
// Without that wait, the selection came while the view loaded, the view started on the worktree (ESC-481).
test('the first agent made late does not take the editor a test opened on the project', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: driven by its DevTools protocol');
  test.setTimeout(180_000);
  const { page } = app;
  // Its answer held until the editor is asked for (then the selection comes while the view loads), 5 s at most.
  await page.evaluate(() => {
    const w = window as unknown as { releaseAgent?: () => void };
    const released = new Promise<void>((r) => (w.releaseAgent = r));
    const fetch = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      const answer = await fetch(input, init);
      if (command.endsWith('/create_agent')) await Promise.race([released, new Promise((r) => setTimeout(r, 5000))]);
      return answer;
    };
  });
  await addProject(page, app.repo, { worktrees: true });
  await page.getByRole('button', { name: 'Éditeur ou workspace du projet' }).click();
  await page.getByRole('menuitemradio', { name: 'Éditeur' }).click();
  await page.evaluate(() => (window as unknown as { releaseAgent?: () => void }).releaseAgent?.());
  const view = page.getByTestId('vscode-view');
  const wb = workbench(page);
  await expect(wb).toBeVisible({ timeout: 60_000 });
  const rootRows = wb.locator('.explorer-folders-view .monaco-list-row[aria-level="1"]');
  await expect(rootRows).toHaveCount(1, { timeout: 30_000 });
  await expect(view.locator('.vscode-bar .src')).not.toContainText('agent-1');
  await view.locator('.vscode-bar .src').click();
  await page.getByRole('menuitemradio').nth(1).click();
  await expect(view.locator('.vscode-bar .src')).toContainText('agent-1');
  await expect(rootRows).toHaveCount(2, { timeout: 30_000 });
});
