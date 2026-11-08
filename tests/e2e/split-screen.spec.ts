// The split screen (ESC-392, direction B): an agent, then a terminal dragged from the sidebar onto the work area, each
// beside what is shown (the compass of five cells lit under the pointer); the active pane changed by a click and by
// Ctrl+Alt+arrow, followed by the sidebar; a drag called off by Escape; a pane closed by its ×, then back to one
// content by Ctrl+\; and at 1024 × 728 nothing past its pane. The captures go to ESCOUADE_E2E_SHOTS when it is set.

import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test, unfoldSections } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** Drags `from` to `to` (a point of the window) as a hand would, the compass shown on the way; `release: false` keeps it held. */
async function drag(page: Page, from: Locator, to: { x: number; y: number }, release = true) {
  const b = (await from.boundingBox())!;
  const x = b.x + Math.min(60, b.width / 2);
  const y = b.y + Math.min(14, b.height / 2);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 12, y + 12, { steps: 3 });
  await page.mouse.move(to.x, to.y, { steps: 12 });
  await expect(page.getByTestId('pane-drop')).toBeVisible();
  if (release) await page.mouse.up();
}

/** A point of `el` at (fx, fy) of its size. */
async function at(el: Locator, fx: number, fy: number) {
  const b = (await el.boundingBox())!;
  return { x: b.x + b.width * fx, y: b.y + b.height * fy };
}

/** The keys of the panes shown, in order. */
const keys = (page: Page) => page.getByTestId('pane').evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.paneKey));

/** What spills past its pane, or past the window: none of it. */
const spills = (page: Page) =>
  page.evaluate(() => {
    const out: string[] = [];
    if (document.documentElement.scrollWidth > innerWidth + 1) out.push('the window scrolls sideways');
    for (const pane of document.querySelectorAll<HTMLElement>('[data-testid="pane"]')) {
      const r = pane.getBoundingClientRect();
      if (r.right > innerWidth + 1 || r.bottom > innerHeight + 1) out.push(`${pane.dataset.paneKey} leaves the window`);
      for (const el of pane.querySelectorAll<HTMLElement>('header *, .composer *, .head *')) {
        if (!el.offsetParent) continue;
        const e = el.getBoundingClientRect();
        if (e.width && (e.left < r.left - 1 || e.right > r.right + 1))
          out.push(`${pane.dataset.paneKey}: ${el.tagName.toLowerCase()}.${el.className} « ${el.textContent?.trim().slice(0, 30)} »`);
      }
    }
    return out;
  });

test('an agent then a terminal dragged beside, the active pane followed, a pane closed, nothing spilling at 1024 × 728', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await expect(side.locator('.agent')).toHaveCount(1);
  // A second agent and a terminal.
  await page.keyboard.press('Control+n');
  await expect(side.locator('.agent')).toHaveCount(2);
  await unfoldSections(page);
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('main.tv .xterm')).toBeVisible();
  const term = side.locator('.row-item.term').first();
  const [first, second] = [side.locator('.agent.card').nth(0), side.locator('.agent.card').nth(1)];
  const firstId = await first.getAttribute('data-agent');
  const secondId = await second.getAttribute('data-agent');

  // The first agent alone, the second dragged onto the right of the work area: the cell « right » lit, then two panes.
  await first.click();
  const work = page.locator('.work').first();
  await drag(page, second, await at(work, 0.9, 0.5), false);
  await expect(page.getByTestId('pane-drop').locator('[data-zone="right"]')).toHaveClass(/hot/);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'split-drag.png') });
  await page.mouse.up();
  await expect(page.getByTestId('pane-drop')).toBeHidden();
  expect(await keys(page)).toEqual([`agent:${firstId}`, `agent:${secondId}`]);
  // The one dropped is active: ringed, its card highlighted, its composer has the keyboard.
  const right = page.locator(`[data-pane-key="agent:${secondId}"]`);
  await expect(right).toHaveAttribute('data-pane-active', 'true');
  await expect(second).toHaveClass(/\bsel\b/);
  await expect(first).toHaveClass(/\binpane\b/);
  await expect(right.locator('.composer textarea')).toBeFocused();

  // The terminal dragged under the second agent: three panes.
  await drag(page, term, await at(right, 0.5, 0.92));
  expect(await keys(page)).toEqual([`agent:${firstId}`, `agent:${secondId}`, expect.stringMatching(/^term:/)]);
  const termPane = page.getByTestId('pane').nth(2);
  await expect(termPane).toHaveAttribute('data-pane-active', 'true');
  await expect(termPane.locator('.xterm')).toBeVisible();
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'split-three.png') });

  // A click in the first agent's pane makes it active; Ctrl+Alt+→ goes to the pane on its right.
  const left = page.locator(`[data-pane-key="agent:${firstId}"]`);
  await left
    .locator('header')
    .first()
    .click({ position: { x: 20, y: 20 } });
  await expect(left).toHaveAttribute('data-pane-active', 'true');
  await expect(first).toHaveClass(/\bsel\b/);
  await page.keyboard.press('Control+Alt+ArrowRight');
  await expect(left).toHaveAttribute('data-pane-active', 'false');
  await expect(page.locator('[data-pane-active="true"]')).toHaveCount(1);

  // A drag called off by Escape changes nothing.
  await drag(page, first, await at(right, 0.5, 0.5), false);
  await page.keyboard.press('Escape');
  await expect(page.getByTestId('pane-drop')).toBeHidden();
  await page.mouse.up();
  expect(await keys(page)).toHaveLength(3);

  // The terminal's pane closed by its ×: two panes, the agents'.
  await termPane.hover();
  await termPane.getByTestId('pane-close').click();
  expect(await keys(page)).toEqual([`agent:${firstId}`, `agent:${secondId}`]);

  // In a small window, nothing past its pane.
  await page.setViewportSize({ width: 1024, height: 728 });
  await expect.poll(() => spills(page)).toEqual([]);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'split-small.png') });

  // Ctrl+\ comes back to one content: the active pane's.
  await left.click({ position: { x: 20, y: 20 } });
  await page.keyboard.press('Control+Backslash');
  await expect(page.getByTestId('panes')).toBeHidden();
  await expect(page.locator('.work').first()).toHaveAttribute('data-pane-key', `agent:${firstId}`);
});

test('a side with no room left is crossed out on the compass, and replacing still works', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await page.keyboard.press('Control+n');
  await expect(side.locator('.agent')).toHaveCount(2);
  await page.setViewportSize({ width: 1024, height: 728 });
  const [first, second] = [side.locator('.agent.card').nth(0), side.locator('.agent.card').nth(1)];
  await first.click();
  // Side by side once (each pane ~340 px wide): a pane too narrow to cut again, its sides crossed out.
  await drag(page, second, await at(page.locator('.work').first(), 0.9, 0.5));
  await expect(page.getByTestId('pane')).toHaveCount(2);
  const left = page.getByTestId('pane').first();
  await drag(page, second, await at(left, 0.95, 0.5), false);
  await expect(page.getByTestId('pane-drop').locator('[data-zone="right"]')).toHaveClass(/off/);
  // Its middle replaces: the two swap.
  await page.mouse.move(...(Object.values(await at(left, 0.5, 0.5)) as [number, number]), { steps: 6 });
  await expect(page.getByTestId('pane-drop').locator('[data-zone="center"]')).toHaveClass(/hot/);
  await page.mouse.up();
  const secondId = await second.getAttribute('data-agent');
  await expect(page.getByTestId('pane').first()).toHaveAttribute('data-pane-key', `agent:${secondId}`);
  await expect(page.getByTestId('pane')).toHaveCount(2);
});
