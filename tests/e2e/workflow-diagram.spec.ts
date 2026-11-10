// The Diagram view of the Workflow tab (ESC-245) on the fake agent of ESC-193, at the runner's size (1024 × 728), where the
// panel is 340 px: a project laid out as GSD, two agents on their steps; the switch to the diagram, its steps and the
// agents on them inside the panel, kept from an agent to another; enlarged over the work panel, zoomed, closed by Échap;
// « Copier en Mermaid » in the clipboard. The diagram is drawn by Escouade (choice 197), nothing of Mermaid is loaded.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const GSD = path.resolve(import.meta.dirname, '..', 'fixtures', 'workflows', 'gsd');

/** GSD in the repository, as workflow.spec.ts lays it out. */
function layOutGsd(repo: string) {
  const copy = (from: string, to: string) => {
    for (const e of fs.readdirSync(from, { withFileTypes: true })) {
      const src = path.join(from, e.name);
      const dst = path.join(to, e.name);
      if (e.isDirectory()) {
        fs.mkdirSync(dst, { recursive: true });
        copy(src, dst);
      } else fs.writeFileSync(dst, fs.readFileSync(src, 'utf8').split('@~/.claude/').join('@.claude/'));
    }
  };
  copy(path.join(GSD, 'home'), repo);
  copy(path.join(GSD, 'project'), repo);
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'add', '-A'], { cwd: repo });
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'GSD'], { cwd: repo });
}

const door = (cmd: string, facts: object[]) => `${cmd} faits-workflow: ${JSON.stringify(facts)}`;

async function play(page: Page, text: string) {
  const ends = await page.getByText('Fait.', { exact: true }).count();
  await send(page, text);
  await expect(page.getByText('Fait.', { exact: true })).toHaveCount(ends + 1);
}

async function openName(page: Page): Promise<string> {
  const name = page.locator('.card[aria-current="true"] .name');
  await expect(name).not.toHaveText(/^agent-\d+$/);
  return (await name.textContent())!.trim();
}

test('the workflow as a diagram: switched to, inside 340 px, enlarged, copied in Mermaid', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  layOutGsd(app.repo);
  await addProject(page, app.repo);

  await play(page, door('/gsd:discuss-phase 3', [{ write: '.planning/CONTEXT.md', text: '# Contexte' }]));
  const a1 = await openName(page);
  await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }, { agent: 'gsd-plan-checker' }]));
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await expect(page.locator('.card')).toHaveCount(2);
  await expect(page.getByText('Fait.', { exact: true })).toHaveCount(0);
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }, { wait: 300 }]));
  const a2 = await openName(page);

  await page.keyboard.press('Control+Shift+B');
  const side = page.locator('aside.git-side');
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  const tab = side.getByTestId('workflow');
  await expect(tab.locator('.wf-step')).toHaveCount(4);

  // The switch: the column, then the diagram, loaded at that moment.
  await expect(tab.getByRole('tab', { name: 'Colonne' })).toHaveAttribute('aria-selected', 'true');
  await tab.getByRole('tab', { name: 'Diagramme' }).click();
  const dg = tab.getByTestId('workflow-diagram');
  const nodes = dg.locator('.dg-node');
  // From the door the open agent took (ESC-406): Execute, which reaches Verify, and Plan by the way back; not Discuss.
  await expect(nodes).toHaveCount(3);
  await expect(dg.locator('.dg-door')).toHaveAttribute('data-door', '/gsd:execute-phase');
  await expect(tab.locator('.wf-step')).toHaveCount(0);
  // As the column says them (ESC-244): the open agent only executed, nothing of its pass is done yet.
  await expect(nodes.nth(0)).toHaveAttribute('aria-label', 'Plan phase');
  await expect(nodes.nth(1)).toHaveAttribute('aria-label', 'Execute phase, en cours');
  await expect(nodes.nth(2)).toHaveAttribute('aria-label', 'Verify work, ensuite');
  // Each agent a dot on its step, the open one current; the loop on Plan, the way back from Verify, the exit.
  await expect(nodes.nth(1).locator('circle.ag.cur title')).toHaveText(a2);
  await expect(nodes.nth(0).locator('circle.ag title')).toHaveText(a1);
  await expect(nodes.nth(0).locator('.dg-self')).toContainText('×3');
  await expect(dg.locator('.dg-chip.ret')).toHaveCount(1);
  await expect(dg.locator('.dg-exit')).toContainText('Sortie');

  // In the panel of 340 px: nothing of the diagram sticks out of it, nothing overlaps from one step to the next.
  expect(Math.round((await side.boundingBox())!.width)).toBe(340);
  const fit = await dg.evaluate((el) => {
    const box = el.closest('aside')!.getBoundingClientRect();
    const out = [...el.querySelectorAll('.dg-node rect, .dg-chip rect, .dg-door rect, .dg-exit rect')]
      .map((r) => r.getBoundingClientRect())
      .filter((r) => r.left < box.left - 0.5 || r.right > box.right + 0.5);
    const steps = [...el.querySelectorAll('.dg-node > rect')].map((r) => r.getBoundingClientRect());
    const overlap = steps.some((a, i) =>
      steps.some((b, j) => i < j && a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom),
    );
    const names = [...el.querySelectorAll('.dg-node .nm')].map((t) => {
      const r = t.getBoundingClientRect();
      const n = t.parentElement!.querySelector('rect')!.getBoundingClientRect();
      return r.right <= n.right + 0.5;
    });
    return { out: out.length, overlap, names };
  });
  expect(fit).toEqual({ out: 0, overlap: false, names: [true, true, true] });

  // Kept from an agent to another: the diagram stays, the other agent current; it came in by Discuss, the four steps.
  await page.locator('.card').filter({ hasText: a1 }).click();
  await expect(nodes).toHaveCount(4);
  await expect(dg.locator('.dg-door')).toHaveAttribute('data-door', '/gsd:discuss-phase');
  await expect(nodes.nth(1).locator('circle.ag.cur title')).toHaveText(a1);
  await expect(nodes.nth(0)).toHaveAttribute('aria-label', 'Discuss phase, faite');
  await expect(nodes.nth(1)).toHaveAttribute('aria-label', 'Plan phase, en cours');
  await expect(nodes.nth(2)).toHaveAttribute('aria-label', 'Execute phase, ensuite');
  await expect(tab.getByRole('tab', { name: 'Diagramme' })).toHaveAttribute('aria-selected', 'true');

  // Enlarged over the whole work panel, zoomed, closed by Échap.
  await tab.getByRole('button', { name: 'Agrandir le diagramme' }).click();
  const big = page.getByTestId('workflow-diagram-big');
  await expect(big).toBeVisible();
  const work = (await page.locator('.work').boundingBox())!;
  const bigBox = (await big.boundingBox())!;
  expect(Math.abs(bigBox.width - work.width)).toBeLessThan(2);
  expect(Math.abs(bigBox.height - work.height)).toBeLessThan(2);
  await expect(big.locator('.dg-node')).toHaveCount(4);
  const zoom = big.getByTestId('workflow-zoom');
  const before = Number((await zoom.textContent())!.replace(/\D/g, ''));
  await big.getByRole('button', { name: 'Zoom avant (Ctrl +)' }).click();
  await expect(zoom).toHaveText(`${before + 10} %`);
  await big.getByRole('region').focus();
  await page.keyboard.press('Escape');
  await expect(big).toHaveCount(0);

  // « Copier en Mermaid »: the workflow, without its agents.
  // What the app writes, kept in the page: the real clipboard stays the user's, and WebView2 asks before reading it.
  await page.evaluate(() => {
    const w = window as unknown as { __copied: string[] };
    w.__copied = [];
    navigator.clipboard.writeText = async (text: string) => void w.__copied.push(text);
  });
  await tab.getByRole('button', { name: 'Copier en Mermaid' }).click();
  await expect(page.getByText(/^Code Mermaid copié : \d+ lignes, sans les agents$/)).toBeVisible();
  const code = await page.evaluate(() => (window as unknown as { __copied: string[] }).__copied[0]);
  expect(code.split('\n')[0]).toBe('flowchart TD');
  expect(code).toContain('s1["Discuss phase<br>/gsd:discuss-phase"]');
  expect(code).toContain('s2 -.->|');
  expect(code).not.toContain(a1);

  // Nothing of Mermaid in the page.
  expect(await page.evaluate(() => 'mermaid' in window)).toBe(false);
});
