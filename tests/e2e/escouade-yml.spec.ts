import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { expect, test, openSettingsPage, settingsEntry } from './fixture';

// A repository with a .escouade.yml (ESC-146): added through the assistant, the project is set up by the file and says
// so; the file's setup command runs in no worktree until the user trusts it, then runs in the next one.

const FILE = `# yaml-language-server: $schema=https://escouade.dev/schemas/escouade.schema.json
version: 1
worktrees:
  per_agent: true
  at_first_message: false
  setup:
    - node -e "require('fs').writeFileSync('setup-ran.txt', 'yes')"
git:
  agent_branch_prefix: team/
board:
  merge_strategy: rebase
  tests_before_approve: true
  unknown_thing: 1
`;

/** A git repository with one commit holding its .escouade.yml, named `name`, in the test's folder. */
function sharedRepo(root: string, name: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  git('config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  fs.writeFileSync(path.join(dir, '.escouade.yml'), FILE);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

/** The folders of the worktrees made in `repo`, where Escouade puts them by default. */
function worktreesOf(repo: string): string[] {
  const root = path.join(repo, '.claude', 'worktrees');
  return fs.existsSync(root) ? fs.readdirSync(root).map((d) => path.join(root, d)) : [];
}

test('a repository’s .escouade.yml sets the project up, its commands waiting for the user’s trust', async ({ app }) => {
  const { page } = app;
  const repo = sharedRepo(path.dirname(app.repo), 'shared-app');

  // « Nouveau projet »: the file is where the project starts from, above the recommended setup.
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await dialog.getByPlaceholder('C:\\chemin\\vers\\le\\projet').fill(repo);
  const start = dialog.getByRole('radio', { name: /^Réglages de \.escouade\.yml/ });
  await expect(start).toHaveAttribute('aria-checked', 'true');
  await expect(start).toContainText('Ses commandes attendront ton accord une fois le projet créé.');
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
  await expect(dialog.getByRole('switch', { name: 'Un worktree par agent' })).toHaveAttribute('aria-checked', 'true');
  await dialog.getByRole('button', { name: /Récapitulatif/ }).click();
  await dialog.getByRole('switch', { name: 'Créer un premier agent' }).click();
  await dialog.getByRole('button', { name: 'Créer le projet' }).click();
  await expect(dialog).toBeHidden();
  await expect(
    page.getByText('shared-app : réglages lus dans .escouade.yml. Ses commandes attendent ton accord (Réglages › Projets › Équipe).'),
  ).toBeVisible();

  // An agent before the trust: the file's branch prefix, and nothing of its setup ran.
  // The project on screen first: a click before it is lost.
  await expect(page.getByText('Aucun agent.')).toBeVisible();
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  // The first agent waits for the `claude update` the app runs a few seconds after it starts.
  await expect(page.locator('.card')).toHaveCount(1, { timeout: 45_000 });
  await expect.poll(() => worktreesOf(repo).length).toBe(1);
  const first = worktreesOf(repo)[0];
  await expect.poll(() => execFileSync('git', ['branch', '--show-current'], { cwd: first }).toString().trim()).toMatch(/^team\//);
  expect(fs.existsSync(path.join(first, 'setup-ran.txt'))).toBe(false);

  // The board says it, and leads to the commands.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByText('Les commandes de .escouade.yml attendent ton accord : elles ne tournent pas encore.')).toBeVisible();
  await page.getByRole('button', { name: 'Les lire' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  // On « Projets › shared-app › Équipe » (annexe B), scrolled to « Réglages partagés ».
  await expect(settingsEntry(settings, 'Équipe')).toHaveAttribute('aria-current', 'page');
  await expect(settings.getByRole('heading', { name: 'Réglages partagés' })).toBeInViewport();
  await expect(settings.getByText(/Réglages lus dans \.escouade\.yml \(version 1\)/)).toBeVisible();
  // What was left out is said with its line.
  await expect(settings.getByText(/Ligne 13 : clé inconnue : board\.unknown_thing/)).toBeVisible();
  // The commands waiting, read in full and trusted here.
  const pending = settings.getByRole('region', { name: 'Commandes en attente de ton accord' });
  await expect(pending.getByText(`node -e "require('fs').writeFileSync('setup-ran.txt', 'yes')"`)).toBeVisible();
  // The settings it set are marked where they are, on « Git et worktrees », those of the Kanban too.
  await openSettingsPage(settings, 'Git et worktrees');
  const perAgent = settings.locator('.sr', { hasText: 'Un worktree par agent' });
  await expect(perAgent.getByText('Défini par .escouade.yml')).toBeVisible();
  // Its worktrees at once (ESC-156), as the file says.
  const atFirst = settings.locator('.sr', { hasText: 'Créer le worktree au premier message' });
  await expect(atFirst.getByText('Défini par .escouade.yml')).toBeVisible();
  await expect(atFirst.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  await openSettingsPage(settings, 'Équipe');
  await pending.getByRole('button', { name: 'Faire confiance à ces commandes' }).click();
  await expect(pending).toBeHidden();
  await openSettingsPage(settings, 'Git et worktrees');
  await expect(settings.locator('.sr', { hasText: 'Un worktree par agent' }).getByText('Défini par .escouade.yml')).toBeVisible();
  await openSettingsPage(settings, 'Kanban');
  await expect(settings.locator('.sr', { hasText: 'Relancer les tests avant' }).getByText('Défini par .escouade.yml')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();
  await expect(page.getByText(/attendent ton accord : elles ne tournent pas encore/)).toBeHidden();

  // Trusted: the next agent's worktree is set up by it.
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await expect.poll(() => worktreesOf(repo).length).toBe(2);
  const second = worktreesOf(repo).find((w) => w !== first)!;
  await expect.poll(() => fs.existsSync(path.join(second, 'setup-ran.txt')), { timeout: 30_000 }).toBe(true);
});
