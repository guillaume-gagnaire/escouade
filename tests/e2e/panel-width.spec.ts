// The side panel (Non commités, Historique, Workflow, PR et CI) widened by its left edge (ESC-659), as the sidebar is by
// its right one: beside the conversation and docked side by side, each its own width, kept after a restart, the
// keyboard, a double click, held to the room the conversation keeps, its tabs packed by its container queries when
// narrow and back when wide. In the runner's window (1024 × 728) and at 1440 × 900.

import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const panel = (page: Page) => page.locator('aside.git-side');
const edge = (page: Page) => page.getByRole('separator', { name: 'Largeur du panneau' });
const sideEdge = (page: Page) => page.getByRole('separator', { name: 'Largeur de la barre des agents' });
const width = async (page: Page) => Math.round((await panel(page).boundingBox())!.width);
const valueOf = async (page: Page, name: string) => Number(await edge(page).getAttribute(name));

/** Two frames: the container queries of a new width apply. */
const settle = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));

/** What breaks: the page scrolled sideways, the panel out of the window, the conversation under what it keeps, a tab cut. */
function breaks(page: Page) {
  return page.evaluate(() => {
    const out: string[] = [];
    const doc = document.scrollingElement!;
    if (doc.scrollWidth > window.innerWidth + 1) out.push(`the page scrolls: ${doc.scrollWidth} > ${window.innerWidth}`);
    const p = document.querySelector('aside.git-side')!.getBoundingClientRect();
    if (p.right > window.innerWidth + 1) out.push(`the panel out of the window: ${p.right}`);
    const conv = document.querySelector('main.conv')!.getBoundingClientRect();
    // Wider than its narrowest, the panel leaves the conversation its room (at its narrowest, it is what it was before).
    if (p.width > 341 && conv.width < 340) out.push(`the conversation too narrow: ${conv.width} beside ${p.width}`);
    if (conv.right > p.left + 1) out.push(`the conversation under the panel: ${conv.right} > ${p.left}`);
    for (const tab of document.querySelectorAll<HTMLElement>('aside.git-side [role=tab]')) {
      if (!tab.offsetWidth) continue;
      const r = tab.getBoundingClientRect();
      if (r.right > p.right + 1) out.push(`« ${tab.textContent?.trim()} » out of the panel`);
    }
    return out;
  });
}

/** Drags the panel's edge by `dx` px. */
async function drag(page: Page, dx: number) {
  const e = (await edge(page).boundingBox())!;
  const y = e.y + e.height / 2;
  const x = e.x + e.width / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx / 2, y, { steps: 4 });
  await page.mouse.move(x + dx, y, { steps: 4 });
  await page.mouse.up();
  await settle(page);
}

for (const size of [
  { width: 1024, height: 728 },
  { width: 1440, height: 900 },
])
  test(`the side panel is widened by its left edge, opened or docked, kept, held to the window, at ${size.width} × ${size.height}`, async ({
    app,
  }) => {
    test.setTimeout(150_000);
    let { page } = app;
    await page.setViewportSize(size);
    await addProject(page, app.repo);
    await expect(page.locator('main.conv')).toBeVisible();
    // In the runner's window, the sidebar at its narrowest leaves the panel room to grow.
    if (size.width === 1024) {
      await sideEdge(page).focus();
      await page.keyboard.press('Home');
    }
    await page.keyboard.press('Control+Shift+B');
    await expect(panel(page)).toBeVisible();
    await expect(panel(page)).not.toHaveClass(/docked/);
    await settle(page);

    // Its usual width, its bounds said to a screen reader, its tooltip.
    expect(await width(page)).toBe(340);
    await expect(edge(page)).toHaveAttribute('aria-valuenow', '340');
    await expect(edge(page)).toHaveAttribute('aria-valuemin', '340');
    const widest = await valueOf(page, 'aria-valuemax');
    expect(widest).toBeGreaterThan(340);
    await edge(page).hover();
    await expect(page.getByRole('tooltip')).toContainText('double clic : largeur par défaut');
    // Narrow, « PR et CI » leaves the strip for the × (its container query).
    await expect(page.locator('#side-tab-hosts')).toBeHidden();
    expect(await breaks(page)).toEqual([]);

    // Dragged left by 40 px: 40 px wider. Then far to the left: held at its widest, the conversation keeping its room.
    await drag(page, -40);
    expect(await width(page)).toBe(Math.min(380, widest));
    await drag(page, -1500);
    expect(await width(page)).toBe(widest);
    await expect(edge(page)).toHaveAttribute('aria-valuenow', String(widest));
    expect(await breaks(page)).toEqual([]);
    if (widest > 420) await expect(page.locator('#side-tab-hosts')).toBeVisible();

    // Kept after a restart.
    page = await app.restart();
    await page.setViewportSize(size);
    await expect(page.locator('main.conv')).toBeVisible();
    await page.keyboard.press('Control+Shift+B');
    await expect(panel(page)).toBeVisible();
    await settle(page);
    expect(await width(page)).toBe(widest);

    // The keyboard: the right arrow narrows it by 16 px, the left one widens it, Home to its narrowest.
    await edge(page).focus();
    await page.keyboard.press('ArrowRight');
    await settle(page);
    expect(await width(page)).toBe(widest - 16);
    await page.keyboard.press('ArrowLeft');
    await settle(page);
    expect(await width(page)).toBe(widest);
    await page.keyboard.press('Home');
    await settle(page);
    expect(await width(page)).toBe(340);
    await expect(page.locator('#side-tab-hosts')).toBeHidden();
    // Dragged far to the right: held at its narrowest.
    await drag(page, 1500);
    expect(await width(page)).toBe(340);
    expect(await breaks(page)).toEqual([]);
    await page.keyboard.press('End');
    await settle(page);

    // A narrower window draws it narrower, its width kept for the room that comes back.
    await page.setViewportSize({ width: 1000, height: size.height });
    await settle(page);
    expect(await width(page)).toBeLessThan(widest);
    expect(await breaks(page)).toEqual([]);
    await page.setViewportSize(size);
    await settle(page);
    expect(await width(page)).toBe(widest);

    // A double click: its usual width.
    await edge(page).dblclick();
    await settle(page);
    expect(await width(page)).toBe(340);

    // Docked side by side: its own width, 44 % of the work at first, widened the same way and kept.
    await page.keyboard.press('Control+Shift+L');
    await expect(panel(page)).toHaveClass(/docked/);
    await settle(page);
    const docked = await width(page);
    expect(docked).toBeGreaterThanOrEqual(340);
    await expect(edge(page)).toHaveAttribute('aria-valuenow', String(docked));
    const dockedMax = await valueOf(page, 'aria-valuemax');
    await drag(page, -1500);
    expect(await width(page)).toBe(dockedMax);
    expect(await breaks(page)).toEqual([]);
    page = await app.restart();
    await page.setViewportSize(size);
    await expect(page.locator('aside.git-side.docked')).toBeVisible();
    await settle(page);
    expect(await width(page)).toBe(dockedMax);
    // The panel beside the classic layout kept its own: the usual one.
    await page.keyboard.press('Control+Shift+L');
    await page.keyboard.press('Control+Shift+B');
    await expect(panel(page)).toBeVisible();
    await expect(panel(page)).not.toHaveClass(/docked/);
    await settle(page);
    expect(await width(page)).toBe(340);
  });
