// The synchronisation of the docs (ESC-534): nothing of the docs in the status bar any more; the Docs tab's badge counts
// the changes of the docs to share, on any tab, and marks a conflict apart; « Synchroniser », in the head of the Docs
// space, shares them (the badge goes), brings in what a teammate pushed with nothing to share, and stops on a conflict,
// shown in its place with « L'agent résout » and « Annuler ». On a project whose remote is a bare repository of the test,
// with a teammate's clone. Run at both sizes (ESCOUADE_E2E_VIEWPORT=1024x728); the captures go to ESCOUADE_E2E_SHOTS,
// dark and light, named after the size.

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
  const width = await page.evaluate(() => window.innerWidth);
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.mouse.move(0, 0);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `docs-sync-${name}-${width}-${t}.png`) });
  }
  await theme(page, 'dark');
}

const PAGE = path.join('.escouade', 'spec', 'paiement', 'echeances.md');
const page_ = (body: string) =>
  `---\ntitle: "Échéances"\nsubject: "Paiement"\nsources: []\ntickets: []\ndecisions: []\ncompared: null\n---\n\n# Échéances\n\n${body}\n`;

test('the Docs tab’s badge counts what to share, « Synchroniser » shares it, brings in the others’ work and stops on a conflict', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' });
  // The project's spec, pushed to a remote, and a teammate's clone of it.
  fs.mkdirSync(path.dirname(path.join(app.repo, PAGE)), { recursive: true });
  fs.writeFileSync(path.join(app.repo, PAGE), page_('Trois échéances au plus.'));
  git(app.repo, 'add', '.');
  git(app.repo, 'commit', '-qm', 'spec');
  const remote = path.join(app.root, 'remote.git');
  // Its HEAD named: a runner whose git starts on master would leave the teammate's clone pulling a branch that is not.
  execFileSync('git', ['init', '-q', '--bare', '--initial-branch=main', remote]);
  git(app.repo, 'remote', 'add', 'origin', remote);
  git(app.repo, 'push', '-q', '-u', 'origin', 'main');
  const other = path.join(app.root, 'other');
  execFileSync('git', ['clone', '-q', '-c', 'core.autocrlf=false', remote, other]);
  for (const [k, v] of [
    ['user.email', 'claire@example.test'],
    ['user.name', 'Claire'],
    ['core.autocrlf', 'false'],
  ])
    git(other, 'config', k, v);
  const teammatePushes = (rel: string, text: string) => {
    git(other, 'pull', '-q', '--no-rebase');
    fs.mkdirSync(path.dirname(path.join(other, rel)), { recursive: true });
    fs.writeFileSync(path.join(other, rel), text);
    git(other, 'add', '-A');
    git(other, 'commit', '-qm', 'les autres');
    git(other, 'push', '-q');
  };
  const remoteHead = () => git(app.repo, '--git-dir', remote, 'rev-parse', 'main').trim();

  await addProject(page, app.repo, { firstAgent: false });
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  const docsTab = tabs.getByRole('button', { name: /^Docs/ });
  const badge = docsTab.getByTestId('docs-badge');
  const bar = page.locator('[data-docs-sync]');
  const go = bar.getByRole('button', { name: /^Synchronis/ });
  await docsTab.click();

  // Up to date: the button is there all the same, no badge, nothing of the docs in the status bar.
  await expect(bar.locator('.branch')).toHaveText('main');
  await expect(bar).toContainText('à jour');
  await expect(go).toBeEnabled();
  await expect(badge).toHaveCount(0);
  await expect(page.locator('footer.bar')).not.toContainText(/partager|Docs sur la branche/);
  await shoot(page, 'up-to-date');

  // A change of the docs, written in the window: the badge counts it, on the Docs tab and elsewhere.
  const edit = async (text: string) => {
    await page
      .getByRole('navigation', { name: 'Pages de la spécification' })
      .getByRole('button', { name: /Échéances/ })
      .click();
    await page.getByRole('button', { name: 'Modifier' }).click();
    await page.getByRole('textbox', { name: 'Texte de la page, en Markdown' }).fill(text);
    await page.getByRole('button', { name: 'Enregistrer' }).click();
    await expect(page.locator('.p-body')).toContainText(text);
  };
  await edit('Quatre échéances au plus.');
  await expect(badge).toHaveText('1');
  await expect(badge).toHaveAttribute('data-state', 'pending');
  await expect(docsTab).toHaveAccessibleName('Docs, 1 changement des docs à partager');
  await expect(bar.locator('[data-pending]')).toHaveText('1 à partager');
  await shoot(page, 'pending');
  await tabs.getByRole('button', { name: /^Agents/ }).click();
  await expect(badge).toHaveText('1');
  await badge.hover();
  await expect(page.getByRole('tooltip')).toHaveText('1 changement des docs à partager');
  await shoot(page, 'badge-elsewhere');
  await page.mouse.move(0, 0);
  await docsTab.click();

  // « Synchroniser » shares it: pushed, the badge gone.
  await go.click();
  await expect(bar).toContainText('à jour', { timeout: 30_000 });
  await expect(badge).toHaveCount(0);
  expect(git(app.repo, '--git-dir', remote, 'show', `main:${PAGE.split(path.sep).join('/')}`)).toContain('Quatre échéances');

  // Nothing to share: what the teammate pushed is brought in, and said; nothing pushed.
  teammatePushes('.escouade/spec/paiement/recu.md', page_('Le reçu part par e-mail.').replace('Échéances', 'Reçu'));
  const theirs = remoteHead();
  await go.click();
  await expect(page.getByText('1 changement récupéré.')).toBeVisible({ timeout: 30_000 });
  expect(fs.existsSync(path.join(app.repo, '.escouade', 'spec', 'paiement', 'recu.md'))).toBe(true);
  expect(remoteHead()).toBe(theirs);
  await expect(badge).toHaveCount(0);

  // Up to date again: « Docs à jour ».
  await go.click();
  await expect(page.getByText('Docs à jour.')).toBeVisible({ timeout: 30_000 });

  // A conflict: the teammate and I changed the same page. The badge marks it apart; the space shows its two gestures.
  teammatePushes(PAGE.split(path.sep).join('/'), page_('Deux échéances au plus.'));
  await edit('Cinq échéances au plus.');
  await expect(badge).toHaveText('1');
  await go.click();
  await expect(badge).toHaveAttribute('data-state', 'conflict', { timeout: 30_000 });
  await expect(docsTab).toHaveAccessibleName('Docs, conflit dans les docs');
  const conflict = page.locator('[data-docs-sync][data-state="conflict"]');
  await expect(conflict).toContainText('Conflit dans les docs');
  await expect(conflict.getByRole('button', { name: 'L’agent résout' })).toBeVisible();
  await expect(conflict.getByRole('button', { name: 'Annuler' })).toBeVisible();
  await expect(go).toHaveCount(0);
  // Undone: my version kept, no marker, the remote untouched.
  expect(fs.readFileSync(path.join(app.repo, PAGE), 'utf8')).toContain('Cinq échéances');
  expect(fs.readFileSync(path.join(app.repo, PAGE), 'utf8')).not.toContain('<<<<');
  await shoot(page, 'conflict');
  await tabs.getByRole('button', { name: /^Agents/ }).click();
  await shoot(page, 'conflict-elsewhere');
  await docsTab.click();

  // « Annuler »: the button back, the change still to share.
  await conflict.getByRole('button', { name: 'Annuler' }).click();
  await expect(go).toBeVisible();
  await expect(badge).toHaveText('1');
  await expect(badge).toHaveAttribute('data-state', 'pending');
});
