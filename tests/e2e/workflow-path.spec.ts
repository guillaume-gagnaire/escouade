// The diagram of a workflow from an entry (ESC-406, direction A of design/workflow-path) on the fake agent of ESC-193, at
// the runner's size (1024 × 728, the panel 340 px): an agent comes into GSD by Plan, goes round its loop once, on to
// Verify, back to Plan, and executes again. The diagram starts from the door it took, draws only what that door reaches
// (Discuss folded, opened on demand), traces its path in full lines over the possibles with each passage numbered on
// its step, and says it in words in an ordered list; another agent stays a dot. Nothing sticks out, in the column and
// enlarged, in both themes. ESCOUADE_E2E_SHOTS names a folder for the screenshots.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const GSD = path.resolve(import.meta.dirname, '..', 'fixtures', 'workflows', 'gsd');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

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

const beads = (dg: Locator, step: string) => dg.locator(`[data-step="${step}"] .dg-bead`);
const lines = (dg: Locator, cls: string) => dg.locator(`path.e.${cls}:not(.head)`);

/** What of `scope` sticks out of `frame`, and whether a bead or a step's name leaves its step. */
const fits = (scope: Locator, frame: Locator) =>
  frame
    .evaluateHandle((f) => f)
    .then((f) =>
      scope.evaluate((el, box) => {
        const b = (box as Element).getBoundingClientRect();
        const out = [
          ...el.querySelectorAll('.dg-node rect, .dg-chip rect, .dg-door rect, .dg-exit rect, .dg-bead circle, .dg-off, .dg-pick, .dg-key'),
        ]
          .map((r) => r.getBoundingClientRect())
          .filter((r) => r.width && (r.left < b.left - 0.5 || r.right > b.right + 0.5)).length;
        const inside = [...el.querySelectorAll('.dg-node')].every((n) => {
          const r = n.querySelector('rect')!.getBoundingClientRect();
          return [...n.querySelectorAll('.nm, .dg-bead circle, .tag, .cmd')].every((x) => {
            const t = x.getBoundingClientRect();
            return t.left >= r.left - 0.5 && t.right <= r.right + 0.5;
          });
        });
        // The name of a step never runs under its numbers.
        const clear = [...el.querySelectorAll('.dg-node')].every((n) => {
          const name = n.querySelector('.nm')!.getBoundingClientRect();
          return [...n.querySelectorAll('.dg-bead circle')].every((c) => c.getBoundingClientRect().left >= name.right - 0.5);
        });
        return { out, inside, clear };
      }, f),
    );

test('the diagram from the door an agent took: its path through a loop and a way back, over what is possible', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  layOutGsd(app.repo);
  await addProject(page, app.repo);

  // In by Plan; a round of its loop; on to Execute and Verify; back to Plan for the gaps; executing again.
  await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }, { agent: 'gsd-plan-checker' }]));
  const a1 = await openName(page);
  await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }]));
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }]));
  await play(page, door('/gsd:verify-work 3', [{ agent: 'gsd-verifier' }]));
  await play(page, door('/gsd:plan-phase 3 --gaps', [{ agent: 'gsd-planner' }]));
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }, { wait: 300 }]));
  // Another agent on the same workflow, in by Verify; then the first one open again.
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await expect(page.locator('.card')).toHaveCount(2);
  await expect(page.getByText('Fait.', { exact: true })).toHaveCount(0);
  await play(page, door('/gsd:verify-work 3', [{ agent: 'gsd-verifier' }, { wait: 300 }]));
  const a2 = await openName(page);
  await page.locator('.card').filter({ hasText: a1 }).click();

  await page.keyboard.press('Control+Shift+B');
  const side = page.locator('aside.git-side');
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  const tab = side.getByTestId('workflow');
  await tab.getByRole('tab', { name: 'Diagramme' }).click();
  const dg = tab.getByTestId('workflow-diagram');

  // It starts from the door the agent took, alone and drawn as taken; no choice of doors.
  const doors = dg.locator('.dg-door');
  await expect(doors).toHaveCount(1);
  await expect(doors).toHaveAttribute('data-door', '/gsd:plan-phase');
  await expect(doors).toHaveClass(/taken/);
  await expect(doors.locator('.lb')).toHaveText('Entrée');
  await expect(tab.getByRole('radiogroup')).toHaveCount(0);

  // Only what that door reaches: Discuss is out of this path, folded, opened on demand.
  const nodes = dg.locator('.dg-node');
  await expect(nodes).toHaveCount(3);
  expect(await nodes.evaluateAll((all) => all.map((n) => n.getAttribute('data-step')))).toEqual([
    'gsd:plan-phase',
    'gsd:execute-phase',
    'gsd:verify-work',
  ]);
  const fold = tab.getByRole('button', { name: '1 étape hors de ce chemin' });
  await expect(fold).toHaveAttribute('aria-expanded', 'false');
  await fold.click();
  const off = tab.getByTestId('workflow-off');
  await expect(off.getByText('Discuss phase')).toBeVisible();
  await expect(off.getByText('Pas atteignable depuis /gsd:plan-phase.')).toBeVisible();

  // Where it really went, in its order: each passage numbered on its step, the one in course apart.
  await expect(beads(dg, 'gsd:plan-phase')).toHaveText(['1', '2', '5']);
  await expect(beads(dg, 'gsd:execute-phase')).toHaveText(['3', '6']);
  await expect(beads(dg, 'gsd:verify-work')).toHaveText(['4']);
  await expect(dg.locator('.dg-bead.on')).toHaveText(['6']);
  await expect(nodes.nth(1)).toHaveAttribute('aria-label', 'Execute phase, en cours');
  await expect(nodes.nth(1).locator('.tag')).toHaveText('en cours');
  await expect(nodes.nth(2)).toHaveAttribute('aria-label', 'Verify work, ensuite');
  await expect(nodes.nth(2).locator('.tag')).toHaveText('ensuite');
  // A step passed stays drawn as passed (a full edge), the next one or not; the exit, never reached, is dashed.
  const edges = await dg.evaluate((el) =>
    [...el.querySelectorAll('.dg-node > rect'), ...el.querySelectorAll('.dg-exit > rect')].map(
      (r) => getComputedStyle(r).strokeDasharray !== 'none',
    ),
  );
  expect(edges).toEqual([false, false, false, true]);

  // Taken against possible: the entry and the two ways forward full, the exit dashed; the way back taken full, with
  // why; the loop on Plan, one round of three.
  await expect(lines(dg, 'walked')).toHaveCount(3);
  await expect(lines(dg, 'fwd')).toHaveCount(1);
  await expect(lines(dg, 'took')).toHaveCount(1);
  await expect(dg.locator('.dg-chip.ret.took')).toHaveCount(1);
  await expect(nodes.nth(0).locator('.dg-self.took text')).toHaveText('1/3');
  const strokes = await dg.evaluate((el) => {
    const of = (sel: string) => {
      const s = getComputedStyle(el.querySelector(sel)!);
      return { width: parseFloat(s.strokeWidth), dashed: s.strokeDasharray !== 'none' };
    };
    return { walked: of('path.e.walked:not(.head)'), fwd: of('path.e.fwd:not(.head)'), took: of('path.e.took:not(.head)') };
  });
  expect(strokes.walked).toEqual({ width: 2.4, dashed: false });
  expect(strokes.took).toEqual({ width: 2.4, dashed: false });
  expect(strokes.fwd).toEqual({ width: 1.2, dashed: true });

  // In words, for a screen reader: the ordered list of its passages.
  const told = tab.getByRole('list', { name: `Chemin de ${a1}, dans l’ordre` });
  const items = await told.getByRole('listitem').allTextContents();
  expect(items).toHaveLength(6);
  expect(items[0]).toBe('Plan phase, entrée par /gsd:plan-phase');
  expect(items[1]).toMatch(/^Plan phase, (tour de boucle : .+|nouveau tour de boucle), 2e passage$/);
  expect(items[2]).toBe('Execute phase');
  expect(items[3]).toBe('Verify work');
  expect(items[4]).toMatch(/^Plan phase, retour depuis Verify work( : .+)?, 3e passage$/);
  expect(items[5]).toBe('Execute phase, 2e passage, en cours');

  // In the panel of 340 px, in both themes: nothing sticks out, the numbers stay in their step, clear of its name.
  expect(Math.round((await side.boundingBox())!.width)).toBe(340);
  for (const theme of ['dark', 'light'] as const) {
    await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
    expect(await fits(tab, side), theme).toEqual({ out: 0, inside: true, clear: true });
    if (SHOTS) await side.screenshot({ path: path.join(SHOTS, `workflow-path-column-${theme}.png`), animations: 'disabled' });
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

  // The other agent of the workflow: a dot where it is, its path not drawn.
  await expect(nodes.nth(2).locator('circle.ag title')).toHaveText(a2);
  await expect(nodes.nth(1).locator('circle.ag.cur title')).toHaveText(a1);
  await expect(dg.locator('.dg-bead')).toHaveCount(6);
  // Opened, it is the one read: it came in by Verify, which reaches Plan and Execute by its way back; one passage.
  await page.locator('.card').filter({ hasText: a2 }).click();
  await expect(doors).toHaveAttribute('data-door', '/gsd:verify-work');
  await expect(dg.locator('.dg-bead')).toHaveText(['1']);
  await expect(nodes.nth(2).locator('circle.ag.cur title')).toHaveText(a2);
  await expect(nodes.nth(1).locator('circle.ag title')).toHaveText(a1);
  // Back to the first: its own door and path again, the other a dot on Verify.
  await page.locator('.card').filter({ hasText: a1 }).click();
  await expect(doors).toHaveAttribute('data-door', '/gsd:plan-phase');
  await expect(dg.locator('.dg-bead')).toHaveCount(6);
  await expect(nodes.nth(2).locator('circle.ag title')).toHaveText(a2);
  await expect(nodes.nth(1).locator('circle.ag.cur title')).toHaveText(a1);
  expect(await fits(tab, side)).toEqual({ out: 0, inside: true, clear: true });

  // Enlarged: the same logic over the work panel, nothing wider than it at 1024 px.
  await tab.getByRole('button', { name: 'Agrandir le diagramme' }).click();
  const big = page.getByTestId('workflow-diagram-big');
  await expect(big).toBeVisible();
  await expect(big.locator('.dg-title')).toContainText('entré par /gsd:plan-phase');
  const canvas = big.locator('.dg-canvas');
  await expect(canvas.locator('.dg-door')).toHaveAttribute('data-door', '/gsd:plan-phase');
  await expect(canvas.locator('.dg-node')).toHaveCount(3);
  await expect(beads(canvas, 'gsd:plan-phase')).toHaveText(['1', '2', '5']);
  await expect(lines(canvas, 'walked')).toHaveCount(3);
  await expect(lines(canvas, 'took')).toHaveCount(1);
  await expect(big.getByRole('list', { name: `Chemin de ${a1}, dans l’ordre` }).getByRole('listitem')).toHaveCount(6);
  await expect(big.getByRole('button', { name: '1 étape hors de ce chemin' })).toBeVisible();
  for (const theme of ['dark', 'light'] as const) {
    await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
    expect(await fits(big, big), `enlarged, ${theme}`).toEqual({ out: 0, inside: true, clear: true });
    if (SHOTS) await big.screenshot({ path: path.join(SHOTS, `workflow-path-big-${theme}.png`), animations: 'disabled' });
  }
  const bar = await big.locator('.dg-bar').evaluate((el) => el.scrollWidth <= el.clientWidth);
  expect(bar).toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await canvas.focus();
  await page.keyboard.press('Escape');
  await expect(big).toHaveCount(0);

  // After a restart, the path is back as it was: it comes from the history the tracking keeps.
  const back = await app.restart();
  await back.setViewportSize({ width: 1024, height: 728 });
  await back.locator('.card .name').filter({ hasText: a1 }).first().click();
  const side2 = back.locator('aside.git-side');
  await side2.waitFor({ state: 'visible', timeout: 3000 }).catch(() => back.keyboard.press('Control+Shift+B'));
  await side2.getByRole('tab', { name: /^Workflow/ }).click();
  const tab2 = side2.getByTestId('workflow');
  await tab2.getByRole('tab', { name: 'Diagramme' }).click();
  const dg2 = tab2.getByTestId('workflow-diagram');
  await expect(dg2.locator('.dg-door')).toHaveAttribute('data-door', '/gsd:plan-phase');
  await expect(dg2.locator('.dg-node')).toHaveCount(3);
  await expect(beads(dg2, 'gsd:plan-phase')).toHaveText(['1', '2', '5']);
  await expect(beads(dg2, 'gsd:execute-phase')).toHaveText(['3', '6']);
  await expect(beads(dg2, 'gsd:verify-work')).toHaveText(['4']);
  await expect(lines(dg2, 'walked')).toHaveCount(3);
  await expect(lines(dg2, 'took')).toHaveCount(1);
  await expect(dg2.locator('[data-step="gsd:plan-phase"] .dg-self.took text')).toHaveText('1/3');
  await expect(tab2.getByRole('list', { name: `Chemin de ${a1}, dans l’ordre` }).getByRole('listitem')).toHaveCount(6);
});
