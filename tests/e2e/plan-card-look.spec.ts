// The card of what « Planifier » made (ESC-291), on top of the Specification, in each of its states: none of its tickets
// started, some, all (folded into their keys), undone, nothing new made; ten tickets with long titles, at the runner's size
// (1004 × 705) and in a large window. Nothing of the card goes out of it, and it lines up with the subjects' cards below.
// The tickets are real (made on the board, started by `ticket_start` with the fake claude); the planning shown is given in
// the page, on the `fetch` of Tauri's IPC that answers `docs_spec_tree`. The captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZES = [
  { width: 1004, height: 705, name: 'small' },
  { width: 1440, height: 900, name: 'large' },
];

const TITLES = [
  'Implémenter : Relance automatique des adhésions qui arrivent à échéance dans les trente jours, avec un e-mail puis un SMS',
  'Implémenter : Notifications',
  'Écran des relances, filtré par antenne régionale, par type d’adhésion et par statut de paiement du dernier appel de cotisation',
  'Reçu fiscal en PDF',
  'Implémenter : Paiement en plusieurs fois (trois échéances au plus) avec un rappel avant chaque prélèvement',
  'Export comptable',
  'Archivage des adhésions closes depuis plus de cinq ans, avec la trace de qui a archivé et quand, consultable par le bureau',
  'Tableau de bord du bureau',
  'Implémenter : Validation par le bureau sous sept jours',
  'Supprimer l’ancien formulaire',
];

interface Made {
  id: string;
  key: string;
  title: string;
}

/** Records the tickets made, and answers `docs_spec_tree` with `window.esc291Plan` as the latest planning. */
async function fakePlanning(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { esc291Made: unknown[]; esc291Plan: unknown };
    w.esc291Made = [];
    w.esc291Plan = null;
    const fetch = window.fetch.bind(window);
    window.fetch = async (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      const res = await fetch(input, init);
      if (command.endsWith('/ticket_create')) {
        const t = await res.clone().json();
        w.esc291Made.push({ id: t.id, key: t.key, title: t.title });
      }
      if (command.endsWith('/docs_spec_tree') && w.esc291Plan && res.ok) {
        const tree = await res.clone().json();
        tree.planning = { pages: [], decisions: [], running: false, ...(tree.planning ?? {}), last: w.esc291Plan };
        return new Response(JSON.stringify(tree), { headers: res.headers, status: res.status });
      }
      return res;
    };
  });
}

/** Shows `plan` on top of the Specification: the Docs tab opened again reads the tree again. */
async function showPlan(page: Page, plan: object) {
  await page.evaluate((p) => ((window as unknown as { esc291Plan: unknown }).esc291Plan = { at: Date.now(), ...p }), plan);
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await tabs.getByRole('button', { name: 'Docs' }).click();
  await expect(page.locator(`section[data-plan="${(plan as { id: string }).id}"]`)).toBeVisible();
}

/** Nothing of the card out of it, the card within the map's width and on its cards' edges. */
async function inside(page: Page) {
  const bad = await page.evaluate(() => {
    const card = document.querySelector('section[data-plan]') as HTMLElement;
    const c = card.getBoundingClientRect();
    const out: string[] = [];
    for (const el of card.querySelectorAll<HTMLElement>('*')) {
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height) continue;
      if (r.left < c.left - 0.5 || r.right > c.right + 0.5 || r.bottom > c.bottom + 0.5) {
        out.push(`${el.tagName}.${el.className}: ${el.textContent?.slice(0, 40)}`);
      }
    }
    if (card.scrollWidth > card.clientWidth + 1) out.push(`scrolls: ${card.scrollWidth} > ${card.clientWidth}`);
    const subject = document.querySelector('[data-subject-card]') as HTMLElement;
    const s = subject.getBoundingClientRect();
    if (Math.abs(s.left - c.left) > 1) out.push(`left ${c.left} ≠ subjects ${s.left}`);
    const map = document.querySelector('.map') as HTMLElement;
    const right = map.getBoundingClientRect().right - parseFloat(getComputedStyle(map).paddingRight);
    if (Math.abs(right - c.right) > 1) out.push(`right ${c.right} ≠ map ${right}`);
    return out;
  });
  expect(bad).toEqual([]);
}

async function shoot(page: Page, name: string) {
  for (const size of SIZES) {
    await page.setViewportSize(size);
    await page.mouse.move(0, 0);
    await page.waitForTimeout(150);
    await inside(page);
    if (!SHOTS) continue;
    fs.mkdirSync(SHOTS, { recursive: true });
    for (const t of ['dark', 'light'] as const) {
      await page.evaluate((t) => {
        document.documentElement.dataset.theme = t;
        document.documentElement.style.colorScheme = t;
      }, t);
      await page.waitForTimeout(150);
      await page.screenshot({ path: path.join(SHOTS, `plan-card-${name}-${size.name}-${t}.png`) });
    }
    await page.evaluate(() => (document.documentElement.dataset.theme = 'dark'));
  }
}

test('the card of « Planifier » keeps to itself and to the subjects’ cards, and says where its tickets stand', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize(SIZES[1]);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: app.repo, encoding: 'utf8' });
  for (const [subject, title] of [
    ['adhesion', 'Adhésion en ligne'],
    ['paiement', 'Paiement'],
    ['notifications', 'Notifications'],
  ]) {
    const file = path.join(app.repo, '.escouade', 'spec', subject, 'general.md');
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(
      file,
      `---\ntitle: "${title}"\nsubject: "${title}"\nsources: []\ntickets: []\ndecisions: []\ncompared: null\n---\n\nCe que fait ${title}.\n`,
    );
  }
  git('add', '.');
  git('commit', '-qm', 'spec');
  await addProject(page, app.repo, { firstAgent: false, name: 'Portail' });
  await fakePlanning(page);

  // Ten tickets on the board, the autopilot off: none starts by itself.
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  for (const title of TITLES) {
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
    await page.keyboard.press('Control+Enter');
    await expect(page.locator('.card', { hasText: title })).toBeVisible();
  }
  const made = await page.evaluate(() => (window as unknown as { esc291Made: Made[] }).esc291Made);
  expect(made).toHaveLength(10);
  const start = (id: string) =>
    page.evaluate(
      (id) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: object) => Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
          'ticket_start',
          { id },
        ),
      id,
    );
  const card = page.locator('section[data-plan]');

  // None started: « À lancer », the note says so, « Annuler » removes them all.
  await showPlan(page, { id: 'pl-todo', pages: [], decisions: [], made, skipped: [{ title: 'Archive', key: 'POR-12' }], cancelled: false });
  await expect(card.locator('[data-plan-state]')).toHaveText('À lancer');
  await expect(card.locator('[data-plan-note]')).toHaveText(/^Pas encore lancés/);
  await expect(card.locator('[data-plan-cancel]')).toBeVisible();
  // A long title is cut, and read whole in its tooltip.
  await card.locator(`[data-made="${made[0].key}"] button`).hover();
  await expect(page.getByRole('tooltip')).toContainText('avec un e-mail puis un SMS');
  await shoot(page, 'todo');

  // Some started: the count, and « Annuler » for the others only.
  await start(made[0].id);
  await expect(card.locator(`[data-made="${made[0].key}"] .col`)).not.toHaveText('À faire', { timeout: 30_000 });
  await expect(card.locator('[data-plan-state]')).toHaveText('1 lancé sur 10');
  await expect(card.locator('[data-plan-note]')).toHaveText(/^9 tickets encore à faire/);
  await shoot(page, 'partial');

  // All started: no note, no « Annuler », the list folded into its keys, unfolded on demand.
  await start(made[1].id);
  await showPlan(page, { id: 'pl-started', pages: [], decisions: [], made: made.slice(0, 2), skipped: [], cancelled: false });
  await expect(card.locator('[data-plan-state]')).toHaveText('Tous lancés', { timeout: 30_000 });
  await expect(card.locator('[data-plan-note]')).toHaveCount(0);
  await expect(card.locator('[data-plan-cancel]')).toHaveCount(0);
  await expect(card.locator('[data-plan-keys]')).toContainText(made[0].key);
  await shoot(page, 'started');
  await card.getByRole('button', { name: 'Voir les tickets' }).click();
  await expect(card.locator('[data-made]')).toHaveCount(2);
  await shoot(page, 'started-open');

  // One ticket, not started.
  await showPlan(page, { id: 'pl-one', pages: [], decisions: [], made: [made[2]], skipped: [], cancelled: false });
  await expect(card.locator('[data-plan-note]')).toHaveText(/^Pas encore lancé :/);
  await shoot(page, 'one');

  // Undone: those removed struck out, the one kept said.
  await showPlan(page, {
    id: 'pl-undone',
    pages: [],
    decisions: [],
    made: [made[0], { id: 'gone-1', key: 'POR-40', title: 'Un ticket retiré par « Annuler »' }],
    skipped: [],
    cancelled: true,
  });
  await expect(card.locator('[data-plan-state]')).toHaveText('Annulé');
  await expect(card.locator('[data-plan-note]')).toContainText(`${made[0].key}, déjà lancé, est resté`);
  await expect(card.locator('[data-made="POR-40"] .col')).toHaveText('Retiré');
  await expect(card.locator('[data-plan-cancel]')).toHaveCount(0);
  await shoot(page, 'undone');

  // Nothing new made.
  await showPlan(page, {
    id: 'pl-none',
    pages: [],
    decisions: [],
    made: [],
    skipped: [{ title: 'Archive', key: 'POR-12' }],
    cancelled: false,
  });
  await expect(card).toContainText('Aucun ticket nouveau');
  await shoot(page, 'none');

  // Closed: gone, and still gone when the tab reads the tree again.
  await card.getByRole('button', { name: 'Fermer' }).click();
  await expect(card).toHaveCount(0);
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await tabs.getByRole('button', { name: 'Docs' }).click();
  await expect(page.locator('[data-subject-card]').first()).toBeVisible();
  await expect(card).toHaveCount(0);
});
