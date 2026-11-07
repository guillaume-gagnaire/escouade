// The Docs tab (ESC-258) on a temporary repository with a spec of its own and a bare remote: the tab and its three spaces,
// the empty space that offers the spec found, its cut by subject imported once agreed to, the map of the subjects with a
// gap against the code, the tree of the pages, a page with its gaps and its sources, edited, then « Partager » from the
// status bar, which commits the docs alone and pushes them. The captures go to ESCOUADE_E2E_SHOTS, dark and light.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const SPEC = `# Portail des adhérents

Le portail sert aux adhérents de l’association.

## Adhésion en ligne

### Formulaire

Le formulaire a trois étapes : identité, pièces, paiement.

### Validation par le bureau

Le bureau valide chaque adhésion sous 7 jours.

## Paiement

### Moyens de paiement

Carte bancaire ou prélèvement.

### En plusieurs fois

Trois échéances **au plus**.

## Notifications

Un e-mail de bienvenue, puis un reçu.
`;

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
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `docs-${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

test('the Docs tab imports the spec found, shows its subjects, a page and its gaps, edits it and shares the docs alone', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: app.repo, encoding: 'utf8' });
  // The project's spec, and a remote to share to.
  fs.mkdirSync(path.join(app.repo, 'docs'));
  fs.writeFileSync(path.join(app.repo, 'docs', 'SPEC.md'), SPEC);
  fs.mkdirSync(path.join(app.repo, 'src'), { recursive: true });
  fs.writeFileSync(path.join(app.repo, 'src', 'payment.ts'), 'export const MAX_INSTALMENTS = 4;\n');
  git('add', '.');
  git('commit', '-qm', 'spec et code');
  const remote = path.join(app.root, 'remote.git');
  execFileSync('git', ['init', '-q', '--bare', remote]);
  git('remote', 'add', 'origin', remote);
  git('push', '-q', '-u', 'origin', 'main');

  await addProject(page, app.repo, { firstAgent: false });

  // Agents · Kanban · PR (ESC-65) · Docs; the three spaces under Docs.
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await expect(tabs.getByRole('button')).toHaveCount(4);
  await tabs.getByRole('button', { name: 'Docs' }).click();
  const spaces = page.getByRole('region', { name: 'Espaces des docs' });
  for (const s of ['Spécification', 'Décisions', 'Documents'])
    await expect(spaces.getByRole('button', { name: new RegExp(s) })).toBeVisible();

  // No spec yet: the empty space offers the one the project has.
  await expect(page.getByText('Pas encore de spécification')).toBeVisible();
  const cand = page.locator('[data-candidate="docs/SPEC.md"]');
  await expect(cand).toBeVisible();
  // The status bar: where the docs live, nothing to share.
  const bar = page.locator('[data-docs-share]');
  await expect(bar).toContainText('Docs sur la branche main');
  await expect(bar).toContainText('rien à partager');
  await shoot(page, 'empty');

  // Its cut, by subject; nothing written before « Importer ».
  await cand.getByRole('button', { name: 'Voir le découpage' }).click();
  const dialog = page.getByRole('dialog', { name: 'Importer docs/SPEC.md' });
  await expect(dialog.locator('[data-draft]')).toHaveCount(6);
  await expect(dialog.getByRole('heading', { name: 'Paiement' })).toBeVisible();
  expect(fs.existsSync(path.join(app.repo, '.escouade'))).toBe(false);
  await shoot(page, 'import');
  await dialog.getByRole('button', { name: 'Importer 6 pages' }).click();
  await expect(dialog).toBeHidden();
  const file = path.join(app.repo, '.escouade', 'spec', 'paiement', 'en-plusieurs-fois.md');
  await expect.poll(() => fs.existsSync(file)).toBe(true);
  expect(fs.readFileSync(file, 'utf8')).toContain('sources: ["docs/SPEC.md"]');

  // What a comparison with the code would leave on a page (ESC-263 writes it): a gap, and where.
  const text = fs.readFileSync(file, 'utf8');
  fs.writeFileSync(
    file,
    text.replace(
      'compared: null\n',
      'compared: "2026-10-28T08:00:00Z"\ngaps: [{"text":"La spec dit 3 fois au plus ; le code accepte 4 échéances.","code":["src/payment.ts"]}]\n',
    ),
  );

  // The map of the subjects, in the spec's order, with where each stands against the code.
  await spaces.getByRole('button', { name: /Spécification/ }).click();
  const map = page.getByRole('list', { name: 'Sujets de la spécification' });
  await expect(map.getByRole('button', { name: 'Ouvrir le sujet Paiement' })).toContainText('1 écart');
  await expect(map.getByRole('button', { name: 'Ouvrir le sujet Adhésion en ligne' })).toContainText('À jour');
  await expect(map.locator('[data-subject-card]')).toHaveCount(4);
  await expect(spaces.getByRole('button', { name: /Spécification/ })).toContainText('1 écart');
  // Its pages, and the list of what was imported (ESC-278, .escouade/spec/.imports.json).
  await expect(bar).toContainText('7 changements à partager');
  // The tree, by subject, as a Confluence.
  const tree = page.getByRole('navigation', { name: 'Pages de la spécification' });
  const subjects = await tree.locator('[data-subject] > .subj .label').allTextContents();
  expect(subjects).toEqual(['Introduction', 'Adhésion en ligne', 'Paiement', 'Notifications']);
  await expect(tree.getByRole('button', { name: /En plusieurs fois/ })).toContainText('écart');
  await shoot(page, 'map');

  // A page: its gaps on top, its Markdown, its source.
  await tree.getByRole('button', { name: /En plusieurs fois/ }).click();
  const gaps = page.getByRole('region', { name: 'Un écart avec le code' });
  await expect(gaps).toContainText('le code accepte 4 échéances');
  await expect(gaps.getByRole('button', { name: 'Ouvrir un ticket pour le code' })).toBeVisible();
  await expect(gaps.getByRole('button', { name: 'Voir le code' })).toBeEnabled();
  await expect(gaps.getByRole('button', { name: 'Mettre la spec à jour' })).toBeVisible();
  await expect(page.locator('.p-body strong', { hasText: 'au plus' })).toBeVisible();
  await expect(page.locator('.sources')).toContainText('docs/SPEC.md');
  await shoot(page, 'page');

  // Edited in the simple editor: the file changes, its header kept.
  await page.getByRole('button', { name: 'Modifier' }).click();
  const body = page.getByRole('textbox', { name: 'Texte de la page, en Markdown' });
  await body.fill('Quatre échéances **au plus**, comme le code.');
  await shoot(page, 'edit');
  await page.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(page.locator('.p-body')).toContainText('Quatre échéances');
  expect(fs.readFileSync(file, 'utf8')).toContain('Quatre échéances **au plus**, comme le code.');
  expect(fs.readFileSync(file, 'utf8')).toContain('gaps: [');

  // « Voir le code » opens the file the gap names in the editor.
  await page.getByRole('button', { name: 'Voir le code' }).click();
  await expect(page.locator('.cm-content')).toContainText('MAX_INSTALMENTS');
  await tabs.getByRole('button', { name: 'Docs' }).click();

  // « Partager »: the docs alone are committed and pushed, the code changed meanwhile left out.
  fs.writeFileSync(path.join(app.repo, 'src', 'payment.ts'), 'export const MAX_INSTALMENTS = 3;\n');
  await expect(bar).toContainText('7 changements à partager');
  await bar.getByRole('button', { name: 'Partager' }).click();
  await expect(bar).toContainText('rien à partager', { timeout: 30_000 });
  const pushed = execFileSync('git', ['--git-dir', remote, 'show', '--name-only', '--format=%s', 'main'], { encoding: 'utf8' });
  expect(pushed).toContain('.escouade/spec/paiement/en-plusieurs-fois.md');
  expect(pushed).toContain('.escouade/spec/.imports.json');
  expect(pushed).not.toContain('src/payment.ts');
  expect(git('status', '--porcelain')).toContain('src/payment.ts');
  // What changed: the commit just shared.
  await page.getByRole('button', { name: 'Carte des sujets' }).click();
  await expect(page.locator('[data-moved]')).toContainText(/a modifié/);
  await shoot(page, 'shared');
});

test('the docs branch is set in the project’s settings: current, or fixed', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('group', { name: 'Vue du projet' }).getByRole('button', { name: 'Docs' }).click();
  await expect(page.locator('[data-docs-share]')).toContainText('Docs sur la branche main');
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Projets' }).click();
  const group = dialog.getByRole('group', { name: 'Branche des docs' });
  await group.scrollIntoViewIfNeeded();
  await expect(group.getByRole('button', { name: 'La branche courante' })).toHaveAttribute('aria-pressed', 'true');
  await group.getByRole('button', { name: 'Une branche fixe' }).click();
  await dialog.getByRole('textbox', { name: 'Nom de la branche des docs' }).fill('docs');
  await dialog.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator('[data-docs-share]')).toContainText('Docs sur la branche docs');
  // The fixed branch, made by Escouade in a worktree of its own: the project's folder stays on main.
  await expect(page.getByText('Pas encore de spécification')).toBeVisible();
  await expect.poll(() => execFileSync('git', ['branch', '--list', 'docs'], { cwd: app.repo, encoding: 'utf8' })).toContain('docs');
  expect(execFileSync('git', ['branch', '--show-current'], { cwd: app.repo, encoding: 'utf8' }).trim()).toBe('main');
});
