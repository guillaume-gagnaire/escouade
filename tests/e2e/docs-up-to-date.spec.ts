// The spec always up to date (ESC-263), with the fake claude: a ticket that changes how the product behaves keeps the
// spec as its protocol asks (a page written and a decision left by Escouade's docs tools); validated, its decision is
// accepted and the pages it touches are compared to the code by the short agent, which finds a gap: on the map of the
// subjects and on top of the page, with its three buttons. « Mettre la spec à jour » opens a discussion with the spec
// writer (ESC-261), whose page poured settles it; « Comparer au code »
// on the whole spec finds nothing more. The captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.mouse.move(0, 0);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `spec-${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

test('a ticket that changes the product keeps the spec, and once validated its page is compared to the code and its gap shown', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: app.repo, encoding: 'utf8' });
  // The spec the project has: a page of its subject Paiement.
  const spec = path.join(app.repo, '.escouade', 'spec', 'paiement');
  fs.mkdirSync(spec, { recursive: true });
  fs.writeFileSync(
    path.join(spec, 'echeances.md'),
    '---\ntitle: "Échéances"\nsubject: "Paiement"\nsources: []\ntickets: []\ndecisions: []\ncompared: null\n---\n\n# Échéances\n\nTrois échéances au plus.\n',
  );
  fs.writeFileSync(path.join(app.repo, 'paiement.ts'), 'export const MAX_INSTALMENTS = 3;\n');
  git('add', '.');
  git('commit', '-qm', 'spec et code');
  await addProject(page, app.repo, { firstAgent: false, name: 'Portail' });

  // The ticket: its agent keeps the spec ([spec:Portail]) before handing it in.
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await tabs.getByRole('button', { name: 'Kanban' }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Quatre échéances [ok] [spec:Portail]');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('POR-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  // In its own worktree until it is validated: the project's folder still says three, its decision proposed.
  const own = path.join(app.repo, '.escouade', 'spec', 'paiement', 'echeances.md');
  expect(fs.readFileSync(own, 'utf8')).toContain('Trois échéances au plus.');
  await review.getByRole('button', { name: 'Valider et merger' }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('Mergé dans main · squash')).toBeVisible({ timeout: 60_000 });
  // Merged with the code: the page and the decision, accepted by the validation, the page naming its ticket.
  const merged = fs.readFileSync(own, 'utf8');
  expect(merged).toContain('Quatre échéances au plus (POR-1).');
  expect(merged).toContain('tickets: ["POR-1"]');
  const decisions = fs.readdirSync(path.join(app.repo, '.escouade', 'decisions'));
  expect(decisions).toHaveLength(1);
  expect(fs.readFileSync(path.join(app.repo, '.escouade', 'decisions', decisions[0]), 'utf8')).toContain('state: "accepted"');
  // Numbered by the validation (ESC-303), after main's register: the first.
  expect(decisions[0]).toMatch(/^0001-/);

  // Compared to the code after the validation: a gap, on the map of the subjects.
  await tabs.getByRole('button', { name: 'Docs' }).click();
  const map = page.getByRole('list', { name: 'Sujets de la spécification' });
  const card = map.getByRole('button', { name: 'Ouvrir le sujet Paiement' });
  await expect(card).toContainText('1 écart', { timeout: 60_000 });
  await expect(card.locator('.c-when')).toContainText('comparé au code');
  await expect(page.locator('[data-stat="gaps"]')).toContainText('1 écart avec le code');
  await shoot(page, 'map-gap');

  // On top of the page, with its three buttons; the file it names is the project's.
  await card.click();
  const gaps = page.getByRole('region', { name: 'Un écart avec le code' });
  await expect(gaps).toContainText('le code en accepte encore trois au plus');
  await expect(gaps).toContainText('paiement.ts');
  await expect(gaps).not.toContainText('dehors');
  for (const name of ['Mettre la spec à jour', 'Ouvrir un ticket pour le code', 'Voir le code'])
    await expect(gaps.getByRole('button', { name })).toBeVisible();
  await expect(page.locator('.meta .when')).toContainText('Comparée au code');
  await shoot(page, 'page-gap');

  // « Mettre la spec à jour »: a discussion with the spec writer, told the gap (ESC-261); the page it proposes, poured,
  // says what the code does, its gap gone, to be shared.
  await gaps.getByRole('button', { name: 'Mettre la spec à jour' }).click();
  const start = page.getByRole('dialog', { name: 'Nouvelle discussion' });
  await expect(start.getByRole('radio', { name: /Discussion/ })).toHaveAttribute('aria-checked', 'true');
  await expect(start.getByRole('textbox').first()).toHaveValue('Mettre à jour « Échéances »');
  await start.getByRole('button', { name: 'Commencer' }).click();
  const proposed = page.getByRole('complementary', { name: 'Ce qui ira dans la spec' }).locator('[data-page="paiement/echeances"]');
  await expect(proposed).toContainText('Mis à jour d’après le code', { timeout: 60_000 });
  expect(fs.readFileSync(own, 'utf8')).toContain('[écart]');
  await page.getByRole('button', { name: 'Verser dans la spec' }).click();
  await expect(proposed).toHaveCount(0);
  await page.getByRole('button', { name: 'Ouvrir la page' }).click();
  await expect(page.getByRole('region', { name: 'Un écart avec le code' })).toHaveCount(0);
  await expect(page.locator('.p-body')).toContainText('Mis à jour d’après le code');
  expect(fs.readFileSync(own, 'utf8')).not.toContain('[écart]');
  expect(git('status', '--porcelain')).toContain('.escouade/spec/paiement/echeances.md');

  // « Comparer au code » on the page, then on the whole spec: nothing more.
  await page.getByRole('button', { name: 'Comparer au code' }).click();
  await expect(page.getByText('1 page comparée · aucun écart')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Carte des sujets' }).click();
  await page.getByRole('button', { name: 'Comparer au code' }).click();
  await expect(page.getByText('1 page comparée · aucun écart').last()).toBeVisible({ timeout: 30_000 });
  await expect(card).toContainText('À jour');
  await shoot(page, 'map-ok');
  // Nothing committed by Escouade since the ticket's merge.
  expect(git('log', '-1', '--format=%s').trim()).toBe('feat: travail du faux claude [POR-1]');
});
