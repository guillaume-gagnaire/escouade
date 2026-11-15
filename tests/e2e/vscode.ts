// The project's editor is the VS Code view (ESC-447): its workbench lives in a layer of its own (`#vscode-layer`), laid
// over the view's place. What the e2e need of it: its explorer, its tabs, the text of its editor.

import type { Locator, Page } from '@playwright/test';
import { expect } from './fixture';

/** The workbench of VS Code. */
export const workbench = (page: Page) => page.locator('#vscode-layer .monaco-workbench');

/** The text of the editor shown (its visible lines). */
export const editorLines = (page: Page) => workbench(page).locator('.editor-instance .monaco-editor .view-lines').first();

/** The labels of the editor's tabs, a modified one marked `*`. */
export const vscodeTabs = (page: Page) =>
  workbench(page)
    .locator('.tabs-container .tab')
    .evaluateAll((els) =>
      els.map((e) => `${e.querySelector('.label-name')?.textContent?.trim()}${e.classList.contains('dirty') ? '*' : ''}`),
    );

/** A row of the explorer, by its name as shown (the last one of that name). */
export const explorerRow = (page: Page, name: string) =>
  workbench(page)
    .locator('.explorer-folders-view .monaco-list-row')
    .filter({ hasText: new RegExp(`^${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`) })
    .last();

/** Folds or unfolds a row of the explorer until it says so (a row drawn again may lose a click). */
export const unfold = (row: Locator, open = true) =>
  expect(async () => {
    if ((await row.getAttribute('aria-expanded')) !== String(open)) await row.click();
    await expect(row).toHaveAttribute('aria-expanded', String(open), { timeout: 2000 });
  }).toPass({ timeout: 30_000 });

/** Opens `path` (`src/app.ts`) of the folder shown by the explorer: its folders unfolded, the file clicked, its tab shown. */
export async function openInExplorer(page: Page, path: string) {
  await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
  const parts = path.split('/');
  for (const dir of parts.slice(0, -1)) await unfold(explorerRow(page, dir));
  await explorerRow(page, parts.at(-1)!).click();
  await expect.poll(() => vscodeTabs(page)).toContain(parts.at(-1));
}
