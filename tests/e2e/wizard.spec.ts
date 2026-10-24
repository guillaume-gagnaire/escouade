import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, test } from './fixture';

// The new project's assistant (ESC-94): a project set up through it, opened again from Réglages › Projets, then a second
// project made by copying the first one's settings, checked in its own folder, and a ticket that runs with them; in the
// narrowest window the app is made for.

/** A git repository with one commit, named `name`, in the test's folder. */
function repoNamed(root: string, name: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  git('config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

/** The folders of the worktrees made in `repo`, where Escouade puts them by default. */
function worktreesOf(repo: string): string[] {
  const root = path.join(repo, '.claude', 'worktrees');
  return fs.existsSync(root) ? fs.readdirSync(root).map((d) => path.join(root, d)) : [];
}

test('a project made by copying another one’s settings runs its ticket with them', async ({ app }) => {
  const { page } = app;
  await page.setViewportSize({ width: 1028, height: 779 });
  // The first project has a branch the second will not have: its target is not taken as is.
  execFileSync('git', ['branch', 'develop'], { cwd: app.repo });
  await addProject(page, app.repo, { firstAgent: false });

  // Réglages › Projets › « Ouvrir l'assistant… »: the first project set up step by step.
  await page.keyboard.press('Control+,');
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Projets' }).click();
  await settings.getByRole('button', { name: 'Ouvrir l’assistant…' }).click();
  const setup = page.getByRole('dialog', { name: 'Configurer demo-api' });
  await expect(setup).toBeVisible();
  await expect(setup.getByRole('radio', { name: /^Garder la configuration actuelle/ })).toHaveAttribute('aria-checked', 'true');
  await setup.getByRole('button', { name: 'Suivant', exact: true }).click();
  // Worktrees: a step that leaves a mark in each new worktree.
  await setup.getByRole('button', { name: 'Ajouter une commande de préparation' }).click();
  await setup.getByRole('textbox', { name: 'Commande' }).fill('git rev-parse HEAD > setup-ran.txt');
  await setup.getByRole('button', { name: 'Suivant', exact: true }).click();
  // Launch: a command in a folder the second project will not have.
  await setup.getByRole('button', { name: '+ Ajouter une commande' }).click();
  const command = setup.getByRole('group', { name: 'Commande 1' });
  await command.getByRole('textbox', { name: 'Nom' }).fill('web');
  await command.getByRole('textbox', { name: 'Commande' }).fill('npm run dev');
  fs.mkdirSync(path.join(app.repo, 'web'));
  await command.getByRole('textbox', { name: /^Sous-dossier/ }).fill('web');
  // The folder is there: nothing said.
  await expect(command.getByText(/introuvable/)).toHaveCount(0);
  await setup.getByRole('button', { name: 'Suivant', exact: true }).click();
  // Kanban: rebase into develop, the agent resolving conflicts.
  await setup.getByRole('button', { name: '⎇ develop' }).click();
  await setup.getByRole('group', { name: 'Stratégie' }).getByRole('button', { name: 'Rebase' }).click();
  await setup
    .getByRole('group', { name: 'En cas de conflit' })
    .getByRole('button', { name: /L.agent résout/ })
    .click();
  await setup.getByRole('button', { name: /Récapitulatif/ }).click();
  await expect(setup.getByText('Harness trouvé · un compte sert le projet · dépôt prêt')).toBeVisible();
  // The whole window in the narrowest one of the app.
  expect(
    await setup.evaluate((e) => {
      const r = e.getBoundingClientRect();
      return r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight;
    }),
  ).toBe(true);
  await setup.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(setup).toBeHidden();

  // A second project, made by copying the first one.
  const atlas = repoNamed(app.root, 'atlas');
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await dialog.getByPlaceholder('C:\\chemin\\vers\\le\\projet').fill(atlas);
  await expect(dialog.getByText(/Dépôt git détecté/)).toBeVisible();
  await dialog.getByRole('radio', { name: /^Copier la configuration/ }).click();
  await expect(dialog.getByRole('combobox', { name: 'Projet copié' })).toHaveValue(/.+/);
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
  // What is copied, by group, checked in the new folder.
  await expect(dialog.getByRole('heading', { name: /Réglages copiés · demo-api/ })).toBeVisible();
  const launch = dialog.getByRole('region', { name: 'Lancement' });
  await expect(launch.getByRole('checkbox', { name: 'web' })).not.toBeChecked();
  await expect(launch.getByText(/dossier « web » introuvable ici/)).toBeVisible();
  const kanban = dialog.getByRole('region', { name: 'Kanban' });
  await expect(kanban.getByText(/branche introuvable : main à la place/)).toBeVisible();
  await expect(dialog.getByRole('region', { name: 'Worktrees' }).getByText('git rev-parse HEAD > setup-ran.txt')).toBeVisible();
  await expect(dialog.getByText(/Pas copiés : les sources liées/)).toBeVisible();
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();
  await expect(dialog.getByRole('textbox', { name: 'Commande' })).toHaveValue('git rev-parse HEAD > setup-ran.txt');
  // Straight to the summary, the rest skipped: what was copied stays.
  await dialog.getByRole('button', { name: 'Passer' }).click();
  await dialog.getByRole('button', { name: 'Passer' }).click();
  await expect(dialog.getByRole('button', { name: '⎇ main' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByRole('button', { name: /Récapitulatif/ }).click();
  await expect(dialog.getByText('merge rebase → branche courante')).toBeVisible();
  await dialog.getByRole('switch', { name: 'Créer un premier agent' }).click();
  await dialog.getByRole('button', { name: 'Créer le projet' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.tab', { hasText: 'atlas' })).toHaveClass(/active/);

  // A ticket of the new project: its worktree is prepared by the copied step, and it is validated as copied.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Le ticket copié');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('ATL-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  expect(worktreesOf(atlas).some((w) => fs.existsSync(path.join(w, 'setup-ran.txt')))).toBe(true);
  await review.getByRole('button', { name: /^Valider/ }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('⤵ Mergé dans main · rebase')).toBeVisible({ timeout: 60_000 });
  // Nothing of the first project's integrations or tickets came with it.
  await expect(done.getByText(/DEM-/)).toHaveCount(0);
});

test('a private project with its Kanban set, made by the assistant, exports its shared settings and none of this machine’s (ESC-163)', async ({
  app,
}) => {
  const { page } = app;
  const vault = repoNamed(app.root, 'vault');
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await dialog.getByPlaceholder('C:\\chemin\\vers\\le\\projet').fill(vault);
  await expect(dialog.getByText(/Dépôt git détecté/)).toBeVisible();

  // Kanban: its tickets' agents in full.
  await dialog.getByRole('button', { name: /Kanban/ }).click();
  await dialog.getByRole('group', { name: 'En parallèle' }).getByRole('button', { name: '4' }).click();
  await dialog.getByRole('switch', { name: 'Attribuer les tickets automatiquement' }).click();
  await dialog.getByRole('combobox', { name: 'Effort' }).selectOption('xhigh');
  await dialog.getByRole('combobox', { name: 'Mode' }).selectOption('plan');
  await dialog.getByRole('button', { name: 'Suivant', exact: true }).click();

  // « Projet privé », the machine's account reserved to it, and shared ones at the limit.
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  await dialog.getByRole('list', { name: 'Comptes réservés à ce projet' }).getByRole('checkbox', { name: 'Principal' }).check();
  await dialog.getByRole('switch', { name: 'Comptes partagés à la limite' }).click();
  await dialog.getByRole('button', { name: /Récapitulatif/ }).click();
  await expect(dialog.getByText('oui · 1 compte réservé · comptes partagés à la limite')).toBeVisible();
  await dialog.getByRole('switch', { name: 'Créer un premier agent' }).click();
  await dialog.getByRole('button', { name: 'Créer le projet' }).click();
  await expect(dialog).toBeHidden();

  // Réglages › Projets: private, its account its own; then « Exporter vers .escouade.yml ».
  await page.keyboard.press('Control+,');
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Projets' }).click();
  await expect(settings.getByRole('switch', { name: 'Projet privé' })).toHaveAttribute('aria-checked', 'true');
  await expect(settings.getByText(/^Compte réservé à ce projet : « Principal »/)).toBeVisible();
  await settings.getByRole('button', { name: 'Exporter vers .escouade.yml' }).click();
  const file = path.join(vault, '.escouade.yml');
  await expect.poll(() => fs.existsSync(file)).toBe(true);
  const text = fs.readFileSync(file, 'utf8');
  // What the team shares.
  expect(text).toMatch(/^private: true$/m);
  expect(text).toMatch(/^ {2}effort: xhigh$/m);
  expect(text).toMatch(/^ {2}mode: plan$/m);
  expect(text).toMatch(/^ {2}model: ''$/m);
  // Nothing of this machine or person: the accounts, the agents in parallel, the autopilot, Remote Control, the key of
  // its tickets.
  for (const local of ['principal', 'Principal', 'parallel', 'autopilot', 'shared_on_limit', 'remote', 'account', 'VAU'])
    expect(text, local).not.toContain(local);
  // The settings mark them as the file's from now on.
  await expect(
    settings
      .locator('.row')
      .filter({ has: page.getByRole('switch', { name: 'Projet privé' }) })
      .getByText('Défini par .escouade.yml'),
  ).toBeVisible();
});
