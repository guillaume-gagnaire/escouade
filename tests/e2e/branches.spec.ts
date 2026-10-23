import { execFileSync } from 'node:child_process';
import { addProject, expect, test } from './fixture';

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

test('a branch is created from the status bar, the folder switched to it, and the git graph shows it as the current one', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const branch = page.locator('footer .sync');
  await expect(branch).toContainText('⎇ main');

  // From the branch picker: « Nouvelle branche… », a name, « et y passer » (ticked by default).
  await branch.click();
  await page.getByRole('dialog', { name: 'Branches' }).getByRole('button', { name: 'Nouvelle branche…' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouvelle branche' });
  await dialog.getByRole('textbox', { name: 'Nom' }).fill('feat/e2e');
  await expect(dialog.getByRole('checkbox', { name: 'et y passer' })).toBeChecked();
  await dialog.getByRole('button', { name: 'Créer' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText('Branche « feat/e2e » créée : tu es dessus.')).toBeVisible();

  // The status bar and the repository agree on where the folder is.
  await expect(branch).toContainText('⎇ feat/e2e');
  expect(git(app.repo, 'branch', '--show-current')).toBe('feat/e2e');
  // It started where main is, and tracks nothing.
  expect(git(app.repo, 'rev-parse', 'feat/e2e')).toBe(git(app.repo, 'rev-parse', 'main'));

  // The graph (« Historique » of the side panel) labels the commit with the current branch, and keeps main as another.
  await page.getByRole('button', { name: /Fichiers/ }).click();
  await page.getByRole('tab', { name: /Historique/ }).click();
  const graph = page.locator('.graph');
  await expect(graph.locator('.ref.head')).toHaveText('feat/e2e');
  await expect(graph.locator('.ref.branch')).toHaveText('main');
});

test('the agents start from the target branch, brought up to date from the remote, and one is made without a worktree from the menu (ESC-144)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  // origin's main is a commit ahead of the local one, fetched but not pulled; the folder is on a branch of its own.
  const bare = `${app.repo}-remote.git`;
  git(app.repo, 'init', '-q', '--bare', '-b', 'main', bare);
  git(app.repo, 'remote', 'add', 'origin', bare);
  git(app.repo, 'push', '-qu', 'origin', 'main');
  git(app.repo, 'commit', '-q', '--allow-empty', '-m', 'on origin');
  git(app.repo, 'push', '-q', 'origin', 'main');
  const ahead = git(app.repo, 'rev-parse', 'main');
  git(app.repo, 'switch', '-q', '-c', 'work', 'main~1');
  git(app.repo, 'branch', '-qf', 'main', 'work');
  git(app.repo, 'commit', '-q', '--allow-empty', '-m', 'work');
  await expect(page.locator('footer .sync')).toContainText('⎇ work');

  // « Branche cible » in the branch picker, then the setting.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Kanban' }).click();
  await settings.getByRole('button', { name: 'Branche cible : work. Choisir une autre branche' }).click();
  const picker = page.getByRole('dialog', { name: 'Branche cible' });
  await picker.getByRole('option', { name: /^main/ }).click();
  await expect(picker).toBeHidden();
  // « Branche de départ des nouveaux worktrees », in the project's settings (ESC-160).
  await settings.getByRole('tab', { name: 'Projets' }).click();
  await settings
    .getByRole('group', { name: 'Branche de départ des nouveaux worktrees' })
    .getByRole('button', { name: 'Branche cible du Kanban' })
    .click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);

  // A new agent: from origin's main, the local main moved to it, the folder left on work.
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect.poll(() => git(app.repo, 'branch', '--list', 'escouade/*')).toContain('escouade/agent-1');
  expect(git(app.repo, 'rev-parse', 'escouade/agent-1')).toBe(ahead);
  expect(git(app.repo, 'rev-parse', 'main')).toBe(ahead);
  expect(git(app.repo, 'branch', '--show-current')).toBe('work');

  // « Nouvel agent sans worktree », beside « Nouvel agent » in a project that gives each agent its own.
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitem', { name: 'Nouvel agent sans worktree' }).click();
  await expect(page.getByRole('button', { name: /agent-2/ }).first()).toBeVisible();
  expect(git(app.repo, 'worktree', 'list').split('\n')).toHaveLength(2);
});

test('« Nouvel agent sur une branche… » creates the branch typed, and the agent’s worktree is named after it (ESC-155)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  const newOn = async (name: string) => {
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitem', { name: 'Nouvel agent sur une branche…' }).click();
    const picker = page.getByRole('dialog', { name: 'Choisir une branche' });
    await picker.getByRole('combobox').fill(name);
    const create = picker.getByRole('option', { name: new RegExp(`Créer la branche « ${name.replace('/', '\/')} » et y lancer un agent`) });
    // Where it starts is said: the folder's branch here.
    await expect(create).toContainText('depuis main');
    await create.click();
    await expect(picker).toBeHidden();
  };
  /** The folder of the worktree that has `branch`, as git lists it. */
  const folderOf = (branch: string) => {
    const blocks = git(app.repo, 'worktree', 'list', '--porcelain').split(/\r?\n\r?\n/);
    const block = blocks.find((b) => b.includes(`branch refs/heads/${branch}`));
    return block?.split(/\r?\n/)[0].replace(/^worktree /, '') ?? '';
  };

  await newOn('feat/signup');
  await expect.poll(() => folderOf('feat/signup')).toMatch(/\.claude\/worktrees\/signup$/);
  expect(git(app.repo, 'rev-parse', 'feat/signup')).toBe(git(app.repo, 'rev-parse', 'main'));
  expect(git(folderOf('feat/signup'), 'branch', '--show-current')).toBe('feat/signup');
  // The folder stays on main, and the agent is shown.
  expect(git(app.repo, 'branch', '--show-current')).toBe('main');
  await expect(page.getByRole('button', { name: /agent-1/ }).first()).toBeVisible();

  // Another branch ending the same way: the next folder.
  await newOn('fix/signup');
  await expect.poll(() => folderOf('fix/signup')).toMatch(/\.claude\/worktrees\/signup-2$/);

  // A name git refuses is said, and nothing is made.
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitem', { name: 'Nouvel agent sur une branche…' }).click();
  const picker = page.getByRole('dialog', { name: 'Choisir une branche' });
  await picker.getByRole('combobox').fill('bad name');
  await expect(picker.getByRole('option', { name: /Créer la branche « bad name »/ })).toHaveAttribute('aria-disabled', 'true');
  await expect(picker.getByText('« bad name » n’est pas un nom de branche valide', { exact: false })).toBeVisible();
  expect(git(app.repo, 'worktree', 'list').split('\n')).toHaveLength(3);
});

test('the new worktrees start from the branch the project names, and an agent from another one chosen in « + Nouvel agent » (ESC-160)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  // Two branches from main with a commit each; the folder stays on main.
  for (const branch of ['release', 'hotfix']) {
    git(app.repo, 'switch', '-q', '-c', branch, 'main');
    git(app.repo, 'commit', '-q', '--allow-empty', '-m', `on ${branch}`);
    git(app.repo, 'switch', '-q', 'main');
  }
  const release = git(app.repo, 'rev-parse', 'release');
  const hotfix = git(app.repo, 'rev-parse', 'hotfix');

  // « Branche de départ des nouveaux worktrees »: a branch, chosen in the picker.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Projets' }).click();
  const choice = settings.getByRole('group', { name: 'Branche de départ des nouveaux worktrees' });
  await expect(choice.getByRole('button', { name: 'Branche actuelle du dossier' })).toHaveAttribute('aria-pressed', 'true');
  await choice.getByRole('button', { name: 'Une branche…' }).click();
  const base = page.getByRole('dialog', { name: 'Branche de départ' });
  await base.getByRole('combobox').fill('release');
  await base.getByRole('option', { name: /^release/ }).click();
  await expect(base).toBeHidden();
  await expect(choice.getByRole('button', { name: 'Branche de départ : release. Choisir une autre branche' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);

  // A new agent: its worktree starts from release, the folder left on main.
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect.poll(() => git(app.repo, 'branch', '--list', 'escouade/agent-1')).toContain('escouade/agent-1');
  expect(git(app.repo, 'rev-parse', 'escouade/agent-1')).toBe(release);
  expect(git(app.repo, 'branch', '--show-current')).toBe('main');

  // « Nouvel agent depuis une autre branche… »: another agent, from hotfix this time.
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitem', { name: 'Nouvel agent depuis une autre branche…' }).click();
  const other = page.getByRole('dialog', { name: 'Branche de départ' });
  await other.getByRole('combobox').fill('hotfix');
  await other.getByRole('option', { name: /^hotfix/ }).click();
  await expect(other).toBeHidden();
  await expect.poll(() => git(app.repo, 'branch', '--list', 'escouade/agent-2')).toContain('escouade/agent-2');
  expect(git(app.repo, 'rev-parse', 'escouade/agent-2')).toBe(hotfix);

  // « Nouvel agent sur une branche… »: a new branch, from hotfix too, chosen after its name.
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitem', { name: 'Nouvel agent sur une branche…' }).click();
  const picker = page.getByRole('dialog', { name: 'Choisir une branche' });
  await picker.getByRole('combobox').fill('fix/urgent');
  // Where the project's new worktrees start, said on the row that creates it at once.
  await expect(picker.getByRole('option', { name: /Créer la branche « fix\/urgent » et y lancer un agent/ })).toContainText(
    'depuis release',
  );
  await picker.getByRole('option', { name: /Créer « fix\/urgent » depuis une autre branche…/ }).click();
  const from = page.getByRole('dialog', { name: 'Branche de départ de « fix/urgent »' });
  await from.getByRole('combobox').fill('hotfix');
  await from.getByRole('option', { name: /^hotfix/ }).click();
  await expect.poll(() => git(app.repo, 'branch', '--list', 'fix/urgent')).toContain('fix/urgent');
  expect(git(app.repo, 'rev-parse', 'fix/urgent')).toBe(hotfix);
  // The project's own start did not move.
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect.poll(() => git(app.repo, 'branch', '--list', 'escouade/agent-4')).toContain('escouade/agent-4');
  expect(git(app.repo, 'rev-parse', 'escouade/agent-4')).toBe(release);
});
