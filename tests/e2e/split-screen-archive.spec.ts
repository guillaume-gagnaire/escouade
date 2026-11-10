// The split screen when an agent goes (ESC-399): an agent on top, a terminal below; archiving the agent keeps the
// terminal where it was, and the agent the app selects in its place takes its pane. With no other agent, the terminal
// alone fills the work area. Run at 1024 × 728 too (ESCOUADE_E2E_VIEWPORT), as on the CI.

import type { Page } from '@playwright/test';
import { addProject, expect, test, unfoldSections } from './fixture';

/** The keys of the panes shown, in order. */
const keys = (page: Page) => page.getByTestId('pane').evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.paneKey));

/** A terminal opened, then put below the agent `agentId` shown alone, by its row's « Ouvrir en dessous ». */
async function terminalBelow(page: Page, agentId: string) {
  const side = page.locator('aside.side');
  await unfoldSections(page);
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('main.tv .xterm')).toBeVisible();
  const term = side.locator('.row-item.term').first();
  await side.locator(`.agent.card[data-agent="${agentId}"]`).click();
  await term.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Ouvrir en dessous' }).click();
  await expect(page.getByTestId('pane')).toHaveCount(2);
  const termKey = (await keys(page))[1]!;
  expect(termKey).toMatch(/^term:/);
  return termKey;
}

/** Archives the agent `agentId` from its card's menu. */
async function archive(page: Page, agentId: string) {
  await page.locator(`aside.side .agent.card[data-agent="${agentId}"]`).click({ button: 'right' });
  await page.getByRole('menuitem', { name: /^Archiver/ }).click();
}

test('archiving the agent on top keeps the terminal below, and the other agent takes its pane', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await page.keyboard.press('Control+n');
  await expect(side.locator('.agent.card')).toHaveCount(2);
  const firstId = (await side.locator('.agent.card').nth(0).getAttribute('data-agent'))!;
  const secondId = (await side.locator('.agent.card').nth(1).getAttribute('data-agent'))!;
  const termKey = await terminalBelow(page, firstId);
  expect(await keys(page)).toEqual([`agent:${firstId}`, termKey]);
  const before = await page.getByTestId('pane').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().height)));
  // The agent's pane active, as when one works in it.
  const top = page.locator(`[data-pane-key="agent:${firstId}"]`);
  await top
    .locator('header')
    .first()
    .click({ position: { x: 20, y: 20 } });
  await expect(top).toHaveAttribute('data-pane-active', 'true');

  await archive(page, firstId);
  await expect(side.locator('.agent.card:not(.archived)')).toHaveCount(1);
  // Still two panes: the other agent where the archived one was, the terminal below, same heights.
  await expect.poll(() => keys(page)).toEqual([`agent:${secondId}`, termKey]);
  await expect(page.locator(`[data-pane-key="${termKey}"] .xterm`)).toBeVisible();
  const after = await page.getByTestId('pane').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().height)));
  expect(after).toEqual(before);
  await expect(page.locator(`[data-pane-key="agent:${secondId}"]`)).toHaveAttribute('data-pane-active', 'true');
});

test('with no other agent, archiving it leaves the terminal alone in the work area', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await expect(side.locator('.agent.card')).toHaveCount(1);
  const firstId = (await side.locator('.agent.card').first().getAttribute('data-agent'))!;
  const termKey = await terminalBelow(page, firstId);

  await archive(page, firstId);
  // One content left: the terminal, shown alone, not the archived agent.
  await expect(page.getByTestId('panes')).toBeHidden();
  await expect(page.locator('.work').first()).toHaveAttribute('data-pane-key', termKey);
  await expect(page.locator('main.tv .xterm')).toBeVisible();
});
