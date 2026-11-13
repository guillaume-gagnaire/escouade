// Where the card, the conversation and the Kanban meet the workflow (ESC-197, choices 145 and 159), at the runner's size
// (1024 × 728), on a project laid out as GSD (tests/fixtures/workflows/gsd) and the fake agent's facts (`faits-workflow`
// of tests/fixtures/fake-claude.mjs): the step on the agent's card in the place of its plan, a divider in the conversation
// at each entry in a step that leads to the Workflow tab, the agent of a ticket in two workflows (GSD with its ticket and
// its loop, « Tickets » of Escouade on « En cours »), its card keeping its loop with the step in its tooltip. The card and
// the divider are compared to the preview (`#loops` of design/app-redesign/index.html) in both themes; then in English.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium, type Locator, type Page } from '@playwright/test';
import { addProject, expect, send, test, type Language } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const GSD = path.join(ROOT, 'tests', 'fixtures', 'workflows', 'gsd');
const PREVIEW = path.join(ROOT, 'design', 'app-redesign', 'index.html');
const PREVIEW_FONT = path.join(ROOT, 'website', 'node_modules', '@fontsource-variable', 'inter', 'opsz.css');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const RUNNER = { width: 1024, height: 728 };

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
  const name = page.locator('.agent.card[aria-current="true"] .name');
  await expect(name).not.toHaveText(/^agent-\d+$/);
  return (await name.textContent())!.trim();
}

const L = {
  fr: {
    newTicket: 'Nouveau ticket',
    title: 'Titre du ticket',
    add: 'Ajouter',
    doing: 'En cours',
    columns: ['À faire', 'En cours', 'À tester', 'Terminé'],
    loop: 'boucle 1/5',
    // The step, then what comes after it (ESC-244).
    step: 'Étape : GSD · Plan phase\nEnsuite : Execute phase',
    ticketLoop: 'boucle 1/5 du ticket',
    sendBack: 'Renvoyer : retour à En cours',
    agentsIn: 'Agents dans Tickets',
    divider: 'Voir l’étape dans l’onglet Workflow',
  },
  en: {
    newTicket: 'New ticket',
    title: 'Ticket title',
    add: 'Add',
    doing: 'In progress',
    columns: ['To do', 'In progress', 'To review', 'Done'],
    loop: 'loop 1/5',
    step: 'Step: GSD · Plan phase\nNext: Execute phase',
    ticketLoop: 'ticket loop 1/5',
    sendBack: 'Send back: back to In progress',
    agentsIn: 'Agents in Tickets',
    divider: 'See the step in the Workflow tab',
  },
} as const;

/**
 * The whole way, in `lang`: an agent through two steps (its card, its dividers, the tab opened from one), then the agent
 * of a ticket that follows GSD too.
 */
async function goThrough(page: Page, repo: string, lang: Language) {
  const l = L[lang];
  await page.setViewportSize(RUNNER);
  layOutGsd(repo);
  await addProject(page, repo, { lang });

  await play(page, door('/gsd:plan-phase 3', [{ agent: 'gsd-planner' }]));
  const a1 = await openName(page);
  await play(page, door('/gsd:verify-work 3', [{ agent: 'gsd-verifier' }]));
  const card = page.locator('.agent.card').filter({ hasText: a1 });

  // The step in the place of the plan, as the corrections name it (none here: by its command).
  await expect(card.locator('.wf-mini')).toHaveText('GSD · Verify work');
  // A divider at each entry, before the message of its door.
  const dividers = page.getByTestId('step-divider');
  await expect(dividers).toHaveCount(2);
  await expect(dividers.nth(0)).toContainText('GSD · Plan phase/gsd:plan-phase');
  await expect(dividers.nth(1)).toContainText('GSD · Verify work/gsd:verify-work');
  await expect(dividers.nth(1).locator('xpath=following-sibling::*[1]')).toContainText('/gsd:verify-work 3');
  // It leads to the Workflow tab, on GSD.
  const lead = dividers.nth(1).getByRole('button');
  await expect(lead).toHaveAttribute('data-tip', l.divider);
  await lead.click();
  const side = page.locator('aside.git-side');
  await expect(side.getByRole('tab', { name: /^Workflow/ })).toHaveAttribute('aria-selected', 'true');
  const tab = side.getByTestId('workflow');
  await expect(tab.locator('.wf-top .select b')).toHaveText('GSD');
  await expect(tab.locator('.wf-step.here .wf-name')).toHaveText('Verify work');

  // A ticket whose agent follows GSD too: it plans, its turn held (its ticket stays under way).
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: l.newTicket }).click();
  await page.getByRole('textbox', { name: l.title }).fill('Export CSV des factures');
  await page.getByRole('textbox', { name: 'Description' }).fill(`faits-workflow: ${JSON.stringify([{ agent: 'gsd-planner' }])}`);
  await page.getByRole('button', { name: l.add, exact: true }).click();
  await expect(page.getByRole('region', { name: l.doing }).getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  const ticketCard = page.locator('.agent.card').filter({ hasText: 'DEM-1' });
  await ticketCard.click();
  // Its loop keeps its place, the step in its tooltip.
  const loop = ticketCard.locator('.a-meta .loop');
  await expect(loop).toContainText(l.loop);
  await expect(loop).toHaveAttribute('data-tip', l.step);
  await expect(ticketCard.locator('.wf-mini')).toHaveCount(0);

  // In GSD with its ticket and its loop.
  // The panel the divider opened stays open beside it.
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  await expect(tab.locator('.wf-top .select b')).toHaveText('GSD');
  const row = tab.locator('.wf-ag').filter({ hasText: 'DEM-1' });
  await expect(row.locator('.where')).toContainText('Plan phase');
  await expect(row.locator('.where')).toContainText(l.ticketLoop);
  // In « Tickets » on « En cours ».
  await tab.locator('.wf-top .select').click();
  await page.getByRole('menuitemradio', { name: 'Tickets' }).click();
  await expect(tab.locator('.wf-step .wf-name')).toHaveText([...l.columns]);
  await expect(tab.locator('.wf-step.here .wf-name')).toHaveText(l.doing);
  await expect(tab.locator('.wf-step.here .who[aria-current="true"]')).toBeVisible();
  await expect(tab.getByText(l.sendBack)).toBeVisible();
  await expect(tab.getByText(l.agentsIn)).toBeVisible();
  // The way back drawn in the gutter.
  await expect(tab.locator('.wf-edges path')).toHaveCount(1);
  return { a1, card, dividers, side, tab };
}

/** Nothing of `root` sticks out of it, nor is cut, but the names cut by design (an ellipsis). */
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

function look(el: Locator) {
  return el.evaluate((node) => {
    const s = getComputedStyle(node);
    return {
      height: Math.round(node.getBoundingClientRect().height),
      color: s.color,
      gap: s.gap,
      margin: s.margin,
      font: `${s.fontWeight} ${s.fontSize} ${s.fontFamily.split(',')[0]}`,
      border: `${s.borderTopWidth} ${s.borderTopStyle} ${s.borderTopColor}`,
    };
  });
}
const lineOf = (el: Locator) =>
  el.evaluate((node) => {
    const s = getComputedStyle(node, '::before');
    return `${s.borderTopWidth} ${s.borderTopStyle} ${s.borderTopColor}`;
  });
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

test('the step on the card and in the conversation, an agent of a ticket in GSD and in Tickets, as the preview draws them', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  const { card, dividers, side, tab } = await goThrough(page, app.repo, 'fr');

  // At the runner's size, in both themes: nothing of the card, the divider nor the tab sticks out.
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    expect.soft(await sticksOut(card), `the card, ${t}`).toEqual([]);
    expect.soft(await sticksOut(side), `the panel, ${t}`).toEqual([]);
    if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `workflow-card-${t}.png`) });
  }

  // Compared to the preview, in both themes.
  const needs = 'the preview needs Playwright’s Chromium and the site’s packages (website/npm ci)';
  test.skip(!fs.existsSync(PREVIEW_FONT), needs);
  await card.click();
  await expect(dividers).toHaveCount(2);
  // Launched headless, Chromium is Playwright's « headless shell », another download than `chromium.executablePath()`:
  // either may be missing (ESC-440).
  const browser = await chromium.launch().catch(() => null);
  test.skip(!browser, needs);
  if (!browser) return;
  const preview = await browser.newPage();
  try {
    await preview.setViewportSize(RUNNER);
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      await preview.goto('about:blank');
      await preview.goto(`${pathToFileURL(PREVIEW).href}?theme=${t}#loops`);
      await preview.evaluate(() => document.fonts.ready);
      const pmini = preview.locator('#ag-pag .wf-mini');
      const pdiv = preview.locator('#th-main .wf-div');
      expect.soft(await look(card.locator('.wf-mini')), `the step on the card, ${t}`).toEqual(await look(pmini));
      expect.soft(await look(card.locator('.wf-mini svg')), `its icon, ${t}`).toEqual(await look(pmini.locator('svg')));
      expect.soft(await look(dividers.nth(1)), `the divider, ${t}`).toEqual(await look(pdiv));
      expect.soft(await lineOf(dividers.nth(1)), `its line, ${t}`).toBe(await lineOf(pdiv));
      expect.soft(await look(dividers.nth(1).getByRole('button')), `its button, ${t}`).toEqual(await look(pdiv.locator('button')));
      expect.soft(await look(dividers.nth(1).locator('.mono')), `its door, ${t}`).toEqual(await look(pdiv.locator('.mono')));
      if (SHOTS) await preview.screenshot({ path: path.join(SHOTS, `workflow-card-preview-${t}.png`) });
    }
  } finally {
    await browser.close();
  }
  void tab;
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the same, its words in English and in their place at the runner’s size', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    const { card, side } = await goThrough(page, app.repo, 'en');
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      expect.soft(await sticksOut(card), `the card, ${t}`).toEqual([]);
      expect.soft(await sticksOut(side), `the panel, ${t}`).toEqual([]);
    }
  });
});
