// The workflows in the « Workspace » (ESC-689), in the place of the Workflow tab of the side panel, at the window of
// 1440 × 900 and at the runner's 1024 × 728: a project laid out as GSD (tests/fixtures/workflows/gsd), two agents that go
// through its doors (`faits-workflow` of tests/fixtures/fake-claude.mjs), one sent back from Verify to Plan. The side
// panel has three tabs that hold beside its ×; the canvas opens from the menu of « Éditeur »; each agent's card says its
// step, a way back in red; a click on it opens the diagram of the old tab over the work panel with that agent's path;
// « Workflows » of the floating bar lists the project's workflows and their agents, opens a diagram without an agent
// (its door to choose), copies it in Mermaid; Escape closes the diagram, then « Workflows », then the canvas. Nothing
// sticks out, in both themes; after a restart, the steps are the same. ESCOUADE_E2E_SHOTS names a folder for the
// screenshots, in French and in English.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, send, test, type Language } from './fixture';

const GSD = path.resolve(import.meta.dirname, '..', 'fixtures', 'workflows', 'gsd');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1440, height: 900, name: '1440' },
  { width: 1024, height: 728, name: '1024' },
] as const;

/** GSD in the repository: the user's folder of the fixture as the project's `.claude`, its `@~/.claude/` made the project's. */
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
  // Committed: the project is added as a clean repository.
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'add', '-A'], { cwd: repo });
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'GSD'], { cwd: repo });
}

/** A door typed with the facts the fake agent plays in its turn. */
const door = (cmd: string, facts: object[]) => `${cmd} faits-workflow: ${JSON.stringify(facts)}`;

async function play(page: Page, text: string, done: string) {
  const ends = await page.getByText(done, { exact: true }).count();
  await send(page, text);
  await expect(page.getByText(done, { exact: true })).toHaveCount(ends + 1);
}

async function openName(page: Page): Promise<string> {
  const name = page.locator('.agent.card[aria-current="true"] .name');
  await expect(name).not.toHaveText(/^agent-\d+$/);
  return (await name.textContent())!.trim();
}

const L = {
  fr: {
    done: 'Fait.',
    newAgent: 'Nouvel agent',
    views: 'Éditeur ou workspace du projet',
    tabs: ['Non commités', 'Historique', 'PR et CI'],
    close: 'Fermer le panneau',
    pass: '2ᵉ passage',
    running: 'en cours',
    back: 'Renvoyé par Verify work',
    entered: 'entré par /gsd:plan-phase',
    path: (name: string) => `Chemin de ${name}, dans l’ordre`,
    following: '2 agents le suivent',
    diagram: 'Schéma',
    copy: 'Copier en Mermaid',
    copied: /^Code Mermaid copié : \d+ lignes, sans les agents$/,
    shut: 'Fermer (Échap)',
  },
  en: {
    // The fake agent's own words, in French whatever the interface.
    done: 'Fait.',
    newAgent: 'New agent',
    views: 'Editor or workspace of the project',
    tabs: ['Uncommitted', 'History', 'PR and CI'],
    close: 'Close the panel',
    pass: 'pass 2',
    running: 'running',
    back: 'Sent back by Verify work',
    entered: 'entered by /gsd:plan-phase',
    path: (name: string) => `Path of ${name}, in order`,
    following: '2 agents follow it',
    diagram: 'Diagram',
    copy: 'Copy as Mermaid',
    copied: /^Mermaid code copied: \d+ lines, without the agents$/,
    shut: 'Close (Esc)',
  },
} as const;

/** What of `root` sticks out of it, but the names cut by design (an ellipsis). */
const sticksOut = (root: Locator) =>
  root.evaluate((box) => {
    const b = box.getBoundingClientRect();
    return [...box.querySelectorAll<HTMLElement>('*')]
      .filter((el) => {
        const r = el.getBoundingClientRect();
        return r.width > 0 && (r.right > b.right + 0.5 || r.left < b.left - 0.5);
      })
      .map((el) => `${el.tagName}.${el.className}`);
  });

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** Two agents through GSD: a1 in by Plan, on to Execute and Verify, sent back to Plan; a2 executing. */
async function twoAgents(page: Page, repo: string, lang: Language) {
  const l = L[lang];
  layOutGsd(repo);
  await addProject(page, repo, { lang });
  await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }]), l.done);
  const a1 = await openName(page);
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }]), l.done);
  await play(page, door('/gsd:verify-work 3', [{ agent: 'gsd-verifier' }]), l.done);
  await play(page, door('/gsd:plan-phase 3 --gaps', [{ agent: 'gsd-planner' }, { wait: 300 }]), l.done);
  await page.getByRole('button', { name: l.newAgent, exact: true }).first().click();
  await expect(page.locator('.agent.card')).toHaveCount(2);
  await expect(page.getByText(l.done, { exact: true })).toHaveCount(0);
  await play(page, door('/gsd:execute-phase 3', [{ agent: 'gsd-executor' }, { wait: 300 }]), l.done);
  const a2 = await openName(page);
  return { a1, a2 };
}

/** The controls over the canvas (« Quitter », the bar, « Workflows ») side by side, none over another. */
async function controlsApart(page: Page) {
  const boxes = await page.locator('.ws > .hud').evaluateAll((els) =>
    els.map((el) => {
      const r = el.getBoundingClientRect();
      return { name: el.className, left: r.left, right: r.right, top: r.top, bottom: r.bottom };
    }),
  );
  expect(boxes.length).toBe(3);
  return boxes.flatMap((a, i) =>
    boxes
      .slice(i + 1)
      .filter((b) => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom)
      .map((b) => `${a.name} × ${b.name}`),
  );
}

async function openCanvas(page: Page, lang: Language) {
  await page.getByRole('button', { name: L[lang].views }).click();
  await page.getByRole('menuitemradio', { name: 'Workspace' }).click();
  await expect(page.locator('.ws')).toBeVisible();
}

for (const size of SIZES)
  test(`the workflows in the Workspace at ${size.width} px: the step on the card, its diagram, « Workflows »`, async ({ app }) => {
    test.setTimeout(240_000);
    const l = L.fr;
    let { page } = app;
    await page.setViewportSize(size);
    const { a1, a2 } = await twoAgents(page, app.repo, 'fr');

    // The side panel: three tabs, the Workflow tab gone; they hold beside the × at 340 px too (ESC-689).
    await page.keyboard.press('Control+Shift+B');
    const side = page.locator('aside.git-side');
    await expect(side.getByRole('tab')).toHaveText(l.tabs.map((t) => new RegExp(`^${t}`)));
    for (const t of l.tabs) await expect(side.getByRole('tab', { name: new RegExp(`^${t}`) })).toBeVisible();
    expect(await sticksOut(side.locator('.utabs'))).toEqual([]);
    await expect(side.getByRole('button', { name: l.close })).toBeInViewport({ ratio: 1 });
    if (size.width === 1024) expect(Math.round((await side.boundingBox())!.width)).toBe(340);
    await side.getByRole('button', { name: l.close }).click();

    // The canvas, from the menu of « Éditeur »: each agent's step on its card, a way back in red with its reason.
    await openCanvas(page, 'fr');
    expect(await controlsApart(page)).toEqual([]);
    const card = (name: string) => page.locator('.ws article.agent').filter({ hasText: name });
    const step1 = card(a1).getByTestId('card-step');
    await expect(step1.locator('.wf-line')).toContainText('GSD');
    await expect(step1.locator('.wf-line b')).toHaveText('Plan phase');
    await expect(step1.locator('.wf-pass')).toContainText(l.pass);
    await expect(step1.locator('.wf-state')).toHaveText(l.running);
    await expect(step1.locator('.wf-back')).toContainText(l.back);
    // In the red of a failure.
    const red = await step1.locator('.wf-back').evaluate((el) => {
      const probe = el.appendChild(document.createElement('i'));
      probe.style.color = 'var(--del)';
      const same = getComputedStyle(probe).color === getComputedStyle(el).color;
      probe.remove();
      return same;
    });
    expect(red).toBe(true);
    await expect(card(a2).getByTestId('card-step').locator('.wf-line b')).toHaveText('Execute phase');
    await expect(card(a2).getByTestId('card-step').locator('.wf-back')).toHaveCount(0);
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      expect.soft(await sticksOut(card(a1)), `the card, ${t}`).toEqual([]);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `workspace-card-${size.name}-fr-${t}.png`), animations: 'disabled' });
    }
    await theme(page, 'dark');

    // A click on the step: the diagram of the old tab over the whole work panel, from the door the agent took, its path.
    await step1.click();
    const big = page.getByTestId('workflow-diagram-big');
    await expect(big).toBeVisible();
    const work = (await page.locator('.work').boundingBox())!;
    const bigBox = (await big.boundingBox())!;
    expect(Math.abs(bigBox.width - work.width)).toBeLessThan(2);
    expect(Math.abs(bigBox.height - work.height)).toBeLessThan(2);
    await expect(big.locator('.dg-title')).toContainText(l.entered);
    await expect(big.getByRole('list', { name: l.path(a1) }).getByRole('listitem')).toHaveCount(4);
    await expect(big.locator('.dg-node[data-step="gsd:plan-phase"] circle.ag.cur title')).toHaveText(a1);
    await expect(big.locator('.dg-node[data-step="gsd:execute-phase"] circle.ag title')).toHaveText(a2);
    await expect(big.locator('path.e.took:not(.head)')).toHaveCount(1);
    expect(await big.locator('.dg-bar').evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `workspace-diagram-${size.name}-fr-${t}.png`), animations: 'disabled' });
    }
    await theme(page, 'dark');
    // Escape: back to the canvas, as it was.
    await page.keyboard.press('Escape');
    await expect(big).toHaveCount(0);
    await expect(page.locator('.ws')).toBeVisible();

    // « Workflows » of the floating bar: GSD and its two agents where they are.
    const entry = page.locator('.ws').getByRole('button', { name: /^Workflows/ });
    await expect(entry).toContainText('2');
    await entry.click();
    const panel = page.getByTestId('workspace-workflows');
    const gsd = panel.locator('li.wf').filter({ has: page.locator('b', { hasText: /^GSD$/ }) });
    await expect(gsd.locator('.cap')).toContainText(l.following);
    await expect(gsd.locator('.who button').filter({ hasText: a1 }).locator('.at')).toHaveText(`Plan phase · ${l.pass}`);
    await expect(gsd.locator('.who button').filter({ hasText: a2 }).locator('.at')).toHaveText('Execute phase');
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      expect.soft(await sticksOut(panel), `« Workflows », ${t}`).toEqual([]);
      const box = (await panel.boundingBox())!;
      const ws = (await page.locator('.ws').boundingBox())!;
      expect(box.x + box.width).toBeLessThanOrEqual(ws.x + ws.width);
      if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `workspace-workflows-${size.name}-fr-${t}.png`), animations: 'disabled' });
    }
    await theme(page, 'dark');

    // « Copier en Mermaid »: the workflow, without its agents (what the app writes kept in the page).
    await page.evaluate(() => {
      const w = window as unknown as { __copied: string[] };
      w.__copied = [];
      navigator.clipboard.writeText = async (text: string) => void w.__copied.push(text);
    });
    await gsd.getByRole('button', { name: l.copy }).click();
    await expect(page.getByText(l.copied)).toBeVisible();
    const code = await page.evaluate(() => (window as unknown as { __copied: string[] }).__copied[0]);
    expect(code.split('\n')[0]).toBe('flowchart TD');
    expect(code).not.toContain(a1);

    // « Schéma »: without an agent, its door to choose; another door draws from it.
    await gsd.getByRole('button', { name: l.diagram }).click();
    await expect(big).toBeVisible();
    const doors = big.getByTestId('workflow-entries');
    await expect(doors.getByRole('radio').first()).toHaveAttribute('aria-checked', 'true');
    await expect(big.getByTestId('workflow-path')).toHaveCount(0);
    await doors.getByRole('radio').last().click();
    await expect(doors.getByRole('radio').last()).toHaveAttribute('aria-checked', 'true');
    await expect(doors.getByRole('radio', { checked: true })).toHaveCount(1);
    await big.getByRole('button', { name: l.shut }).click();
    await expect(big).toHaveCount(0);
    // Escape closes « Workflows » first, the focus back on its button; then the canvas.
    await expect(panel).toBeVisible();
    await panel.getByRole('heading').focus();
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await expect(entry).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(page.locator('.ws')).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    if (size.width !== 1024) return;
    // After a restart, the steps are the same: they come from the tracks the core keeps.
    page = await app.restart();
    await page.setViewportSize(size);
    await page.locator('.card .name').filter({ hasText: a1 }).first().click();
    await openCanvas(page, 'fr');
    const again = page.locator('.ws article.agent').filter({ hasText: a1 }).getByTestId('card-step');
    await expect(again.locator('.wf-line b')).toHaveText('Plan phase');
    await expect(again.locator('.wf-back')).toContainText(l.back);
  });

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the same words in English, in their place at both sizes', async ({ app }) => {
    test.setTimeout(240_000);
    const l = L.en;
    const { page } = app;
    await page.setViewportSize(SIZES[1]);
    const { a1 } = await twoAgents(page, app.repo, 'en');
    await openCanvas(page, 'en');
    const card = page.locator('.ws article.agent').filter({ hasText: a1 });
    const step = card.getByTestId('card-step');
    await expect(step.locator('.wf-pass')).toContainText(l.pass);
    await expect(step.locator('.wf-state')).toHaveText(l.running);
    await expect(step.locator('.wf-back')).toContainText(l.back);
    await page
      .locator('.ws')
      .getByRole('button', { name: /^Workflows/ })
      .click();
    const panel = page.getByTestId('workspace-workflows');
    await expect(panel.locator('li.wf .cap').first()).toContainText(l.following);
    for (const size of SIZES) {
      await page.setViewportSize(size);
      expect.soft(await controlsApart(page), `the controls, ${size.name}`).toEqual([]);
      for (const t of ['dark', 'light'] as const) {
        await theme(page, t);
        expect.soft(await sticksOut(card), `the card, ${size.name}, ${t}`).toEqual([]);
        expect.soft(await sticksOut(panel), `« Workflows », ${size.name}, ${t}`).toEqual([]);
        if (SHOTS)
          await page.screenshot({ path: path.join(SHOTS, `workspace-workflows-${size.name}-en-${t}.png`), animations: 'disabled' });
      }
    }
    await theme(page, 'dark');
    await step.click();
    const big = page.getByTestId('workflow-diagram-big');
    await expect(big.locator('.dg-title')).toContainText(l.entered);
    await expect(big.getByRole('list', { name: l.path(a1) })).toBeAttached();
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `workspace-diagram-1440-en-dark.png`), animations: 'disabled' });
    await page.keyboard.press('Escape');
    await expect(big).toHaveCount(0);
  });
});
