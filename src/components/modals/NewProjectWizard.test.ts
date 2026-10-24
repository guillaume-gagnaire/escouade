import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { FolderInfo, Project, SetupInfo } from '../../lib/types';
import { agent, board, fakeBackend, project, resetApp } from '../../test/ipc';
import NewProjectModal from './NewProjectModal.svelte';

// The assistant of « Nouveau projet »: its steps, the copy of another project, the checks, and what « Créer » saves.

const folder = (over: Partial<FolderInfo> = {}): FolderInfo => ({
  exists: true,
  isRepo: true,
  branch: 'main',
  dirty: 0,
  name: 'atlas',
  history: null,
  ...over,
});

const setup = (over: Partial<SetupInfo> = {}): SetupInfo => ({
  hasCommit: true,
  branches: ['main', 'feature'],
  missingDirs: [],
  worktreesTracked: false,
  ...over,
});

/** The project the assistant copies: set up as one that works. */
const webShop = project({
  id: 'web',
  name: 'web-shop',
  path: 'C:\\code\\web-shop',
  worktreePerAgent: true,
  worktreeDir: '../wt',
  agentBranchPrefix: 'agent/',
  worktreeCopy: ['.env*', 'config/local.json'],
  worktreeSetup: [{ id: 's1', command: 'npm ci', shell: 'pwsh', cwd: '' }],
  runCommands: [
    { id: 'r1', name: 'api', command: 'cargo run', shell: 'pwsh', cwd: 'api' },
    { id: 'r2', name: 'web', command: 'npm run dev', shell: 'pwsh', cwd: 'web' },
  ],
  board: board({
    target: 'develop',
    strategy: 'rebase',
    conflict: 'agent',
    maxParallel: 3,
    model: 'opus',
    testsFirst: true,
    testCommand: 'npm test',
    prefix: 'WEB',
    nextNumber: 9,
  }),
  commitMode: 'direct',
  integrations: { links: [{ service: 'jira', container: 'WEB', name: 'Web', states: {} }], comments: ['done'] },
});

/** The project the backend makes: a new one, with Escouade's defaults. */
const made = project({ id: 'new', name: 'atlas', path: 'C:\\code\\atlas', board: board({ prefix: 'ATL' }) });

async function typePath(path = 'C:\\code\\atlas') {
  await userEvent.type(screen.getByPlaceholderText(/^C:/), path);
}

const next = () => userEvent.click(screen.getByRole('button', { name: 'Suivant' }));
const step = () => document.querySelector('.page')?.getAttribute('data-step');

describe('the new project’s assistant', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), webShop] });
    app.ui.activeProject = 'web';
    app.modal = { kind: 'newProject' };
  });

  function backend(over: Record<string, (args: any) => unknown> = {}) {
    return fakeBackend({
      inspect_folder: () => folder(),
      inspect_setup: ({ dirs }) => setup({ missingDirs: (dirs as string[]).filter((d) => d === 'web') }),
      create_project: () => made,
      update_project: () => null,
      board_set: ({ settings }) => ({ ...made, board: settings }),
      create_agent: () => agent({ id: 'a9', projectId: 'new' }),
      ...over,
    });
  }

  it('copies the settings of the project on screen, checked in the new folder, and creates the project with them', async () => {
    const be = backend();
    render(NewProjectModal);
    await typePath();
    await userEvent.click(screen.getByRole('radio', { name: /^Copier la configuration/ }));
    expect(screen.getByRole('combobox', { name: 'Projet copié' })).toHaveValue('web');
    // The folders of its commands are asked for.
    await waitFor(() => expect(be.called('inspect_setup').at(-1)?.args.dirs).toEqual(['api', 'web']));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await next();

    // « Réglages copiés », by group, each to take or not.
    expect(step()).toBe('copied');
    expect(screen.getByRole('heading', { name: /Réglages copiés · web-shop/ })).toBeInTheDocument();
    const groups = screen.getAllByRole('region').map((r) => r.getAttribute('aria-label'));
    expect(groups).toEqual(['Worktrees', 'Lancement', 'Kanban', 'Git et compte']);
    const launch = screen.getByRole('region', { name: 'Lancement' });
    expect(within(launch).getByRole('checkbox', { name: 'api' })).toBeChecked();
    // A command whose folder is not in the new folder is not taken, and said.
    expect(within(launch).getByRole('checkbox', { name: 'web' })).not.toBeChecked();
    expect(within(launch).getByText(/dossier « web » introuvable ici/)).toBeInTheDocument();
    const kanban = screen.getByRole('region', { name: 'Kanban' });
    expect(within(kanban).getByText(/branche introuvable : main à la place/)).toBeInTheDocument();
    expect(screen.getByText('Pas copiés : les sources liées (Jira, Trello, GitHub, Linear) et les tickets.')).toBeInTheDocument();
    // The account and commits: the commit mode left out by hand.
    await userEvent.click(within(screen.getByRole('region', { name: 'Git et compte' })).getByRole('checkbox', { name: 'Commit' }));
    await next();

    // The steps that follow start from what was copied, and may be changed.
    expect(step()).toBe('worktrees');
    expect(screen.getByRole('switch', { name: 'Un worktree par agent' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('textbox', { name: 'Dossier des worktrees' })).toHaveValue('../wt');
    await next();
    expect(step()).toBe('launch');
    expect(screen.getAllByPlaceholderText(/npm run dev|commande/i).length).toBeGreaterThan(0);
    expect(screen.getByDisplayValue('cargo run')).toBeInTheDocument();
    expect(screen.queryByDisplayValue('npm run dev')).not.toBeInTheDocument();
    await next();
    expect(step()).toBe('kanban');
    // develop is not in the new folder: its current branch instead.
    expect(screen.getByRole('button', { name: '⎇ main' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Passer' }));
    await userEvent.click(screen.getByRole('button', { name: 'Passer' }));
    await userEvent.click(screen.getByRole('button', { name: 'Passer' }));
    expect(step()).toBe('summary');
    expect(screen.getByText('Harness trouvé · un compte sert le projet · dépôt prêt')).toBeInTheDocument();
    // The steps skipped are said in the rail.
    expect(screen.getByRole('button', { name: /Kanban/ })).toHaveTextContent('passé');

    await userEvent.click(screen.getByRole('button', { name: 'Créer le projet' }));
    await waitFor(() => expect(app.modal).toBeNull());
    // Made with no first agent: it comes once the settings are saved, its worktree set up by them.
    expect(be.called('create_project')[0].args).toMatchObject({
      path: 'C:\\code\\atlas',
      worktreePerAgent: true,
      firstAgent: null,
      restore: false,
    });
    const saved = be.called('update_project')[0].args.project as Project;
    expect(saved).toMatchObject({
      id: 'new',
      worktreePerAgent: true,
      worktreeDir: '../wt',
      agentBranchPrefix: 'agent/',
      worktreeCopy: ['.env*', 'config/local.json'],
      // Not taken: the commit mode stays the default.
      commitMode: 'agent',
      // Neither its integrations nor its key of tickets: the new project keeps its own columns commented, not web-shop's.
      integrations: { links: [], comments: ['review', 'done'] },
    });
    expect(saved.runCommands.map((c) => [c.name, c.cwd])).toEqual([['api', 'api']]);
    expect(saved.runCommands[0].id).not.toBe('r1');
    expect(saved.worktreeSetup.map((s) => s.command)).toEqual(['npm ci']);
    expect(be.called('board_set')[0].args).toMatchObject({
      projectId: 'new',
      settings: {
        target: '',
        strategy: 'rebase',
        conflict: 'agent',
        maxParallel: 3,
        model: 'opus',
        testsFirst: true,
        testCommand: 'npm test',
        prefix: 'ATL',
      },
    });
    expect(be.called('create_agent')[0].args).toEqual({ projectId: 'new', model: 'sonnet', engine: null, isolated: null, base: null });
    expect(app.projects.find((p) => p.id === 'new')?.worktreeDir).toBe('../wt');
    expect(app.ui.activeProject).toBe('new');
  });

  it('starts from the folder’s .escouade.yml above the recommended setup, its commands left for the project (ESC-146)', async () => {
    const preview = project({
      id: '',
      path: 'C:\\code\\atlas',
      worktreePerAgent: true,
      worktreeDir: '../atlas-wt',
      board: board({ strategy: 'rebase', draft: true }),
      shared: {
        local: [],
        trusted: '',
        file: {
          keys: ['worktrees.per_agent', 'worktrees.dir', 'board.merge_strategy', 'worktrees.setup'],
          values: {},
          warnings: [],
          error: null,
          version: 1,
          pending: { hash: 'h', commands: { 'worktrees.setup': [{ command: 'npm ci' }] } },
        },
      },
    });
    const be = backend({ shared_preview: () => preview });
    render(NewProjectModal);
    await typePath();
    const file = await screen.findByRole('radio', { name: /^Réglages de .escouade.yml/ });
    // Chosen by itself, first, and says what it read and that its commands will wait.
    await waitFor(() => expect(file).toHaveAttribute('aria-checked', 'true'));
    const starts = screen.getAllByRole('radio').map((r) => r.querySelector('.ct')?.textContent);
    expect(starts.slice(0, 3)).toEqual(['Réglages de .escouade.yml', 'Copier la configuration d’un projet', 'Configuration recommandée']);
    expect(file).toHaveTextContent(
      'Réglages lus dans .escouade.yml, le fichier partagé du dépôt : 4 réglages. Ses commandes attendront ton accord une fois le projet créé.',
    );
    expect(be.called('shared_preview').at(-1)?.args).toEqual({ path: 'C:\\code\\atlas' });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await next();
    expect(step()).toBe('worktrees');
    expect(screen.getByRole('switch', { name: 'Un worktree par agent' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('textbox', { name: 'Dossier des worktrees' })).toHaveValue('../atlas-wt');
    // Its setup is no command of the assistant: it waits for the user, once the project is made.
    expect(screen.queryByDisplayValue('npm ci')).toBeNull();
  });

  it('waits for the copy to be checked in the new folder before it goes on: what is taken is known by then', async () => {
    let answer: ((s: SetupInfo) => void) | undefined;
    backend({ inspect_setup: ({ dirs }) => ((dirs as string[]).length ? new Promise((r) => (answer = r)) : setup()) });
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await userEvent.click(screen.getByRole('radio', { name: /^Copier la configuration/ }));
    expect(screen.getByRole('button', { name: 'Suivant' })).toBeDisabled();
    expect(screen.getByRole('button', { name: /Récapitulatif/ })).toBeDisabled();
    await waitFor(() => expect(answer).toBeDefined());
    answer!(setup({ missingDirs: ['web'] }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await next();
    await next();
    // Taken as checked by default, at once.
    expect(screen.getByRole('textbox', { name: 'Commande' })).toHaveValue('npm ci');
  });

  it('never offers a private project to copy, and says why (ESC-142)', async () => {
    // The project on screen is private: the other one is offered in its place.
    resetApp({ projects: [project(), { ...webShop, private: true }] });
    app.ui.activeProject = 'web';
    app.modal = { kind: 'newProject' };
    const be = backend();
    const { unmount } = render(NewProjectModal);
    await typePath();
    await userEvent.click(screen.getByRole('radio', { name: /^Copier la configuration/ }));
    const source = screen.getByRole('combobox', { name: 'Projet copié' });
    expect(source).toHaveValue('p1');
    expect(
      within(source)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['demo-api']);
    expect(screen.getByText('Un projet privé n’est pas proposé : rien d’un projet privé ne passe à un autre.')).toBeInTheDocument();
    // Nothing of it is asked for in the new folder: the folders checked are those of the project copied.
    await waitFor(() => expect(be.called('inspect_setup').length).toBeGreaterThan(0));
    expect(be.called('inspect_setup').some((c) => (c.args.dirs as string[]).includes('api'))).toBe(false);
    unmount();

    // Only private projects besides: no copy at all, and the reason stays.
    resetApp({
      projects: [
        { ...project(), private: true },
        { ...webShop, private: true },
      ],
    });
    app.modal = { kind: 'newProject' };
    render(NewProjectModal);
    await typePath();
    expect(screen.queryByRole('radio', { name: /^Copier la configuration/ })).not.toBeInTheDocument();
    expect(screen.getByText('2 projets privés ne sont pas proposés : rien d’un projet privé ne passe à un autre.')).toBeInTheDocument();
  });

  it('starts from the recommended setup, and « Créer sans l’assistant » makes the project with Escouade’s defaults', async () => {
    const be = backend();
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /^Créer sans l.assistant$/ }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('create_project')[0].args).toMatchObject({ worktreePerAgent: false, firstAgent: 'sonnet', restore: false });
    expect(be.called('update_project')).toHaveLength(0);
    expect(be.called('board_set')).toHaveLength(0);
  });

  it('saves the recommended setup when gone through', async () => {
    const be = backend();
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    // Straight to the summary by the rail.
    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    await userEvent.click(screen.getByRole('switch', { name: 'Créer un premier agent' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer le projet' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('create_project')[0].args).toMatchObject({ worktreePerAgent: true });
    expect(be.called('update_project')[0].args.project).toMatchObject({ worktreePerAgent: true, commitMode: 'agent' });
    expect(be.called('board_set')[0].args.settings).toMatchObject({
      action: 'merge',
      target: '',
      strategy: 'squash',
      cleanup: true,
      conventional: true,
      conflict: 'agent',
    });
    expect(be.called('create_agent')).toHaveLength(0);
  });

  it('says a repository without commit, and makes its first commit', async () => {
    let commits = 0;
    const be = backend({
      inspect_setup: () => setup({ hasCommit: commits > 0, branches: commits ? ['main'] : [] }),
      first_commit: () => void commits++,
    });
    render(NewProjectModal);
    await typePath();
    expect(await screen.findByText('Le dépôt n’a aucun commit')).toBeInTheDocument();
    expect(screen.getByText(/Aucun ticket ne démarre/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Faire un premier commit' }));
    expect(be.called('first_commit')[0].args).toEqual({ path: 'C:\\code\\atlas' });
    await waitFor(() => expect(screen.queryByText('Le dépôt n’a aucun commit')).not.toBeInTheDocument());
  });

  it('says worktrees in a folder git tracks, and goes back to the default folder', async () => {
    backend({ inspect_setup: ({ worktreeDir }) => setup({ worktreesTracked: worktreeDir === 'src' }) });
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await next();
    const dir = screen.getByRole('textbox', { name: 'Dossier des worktrees' });
    await userEvent.type(dir, 'src');
    expect(await screen.findByText('Les worktrees iraient dans un dossier suivi par git')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Utiliser .claude/worktrees' }));
    expect(dir).toHaveValue('');
    await waitFor(() => expect(screen.queryByText('Les worktrees iraient dans un dossier suivi par git')).not.toBeInTheDocument());
  });

  it('says a target branch not found, and takes the current branch', async () => {
    backend({ inspect_setup: () => setup({ branches: ['main'] }) });
    app.projects[0].board.target = 'develop';
    app.modal = { kind: 'newProject', projectId: 'p1' };
    render(NewProjectModal);
    await userEvent.click(screen.getByRole('button', { name: /Kanban/ }));
    expect(await screen.findByText('La branche cible develop est introuvable')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Prendre la branche courante' }));
    expect(screen.getByRole('button', { name: '⎇ main' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByText('La branche cible develop est introuvable')).not.toBeInTheDocument();
  });

  it('says when no account serves the project, and when no harness is found, with what to do', async () => {
    backend();
    app.settings.accounts = [{ id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: false }];
    app.enginesFound = [];
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    expect(screen.getByText('Aucun compte ne sert ce projet')).toBeInTheDocument();
    expect(screen.getByText(/Réglages › Comptes/)).toBeInTheDocument();
    expect(screen.getByText('Aucun harness trouvé')).toBeInTheDocument();
    expect(screen.getByText(/indique son chemin dans Réglages › Harness/)).toBeInTheDocument();
    app.enginesFound = ['claude'];
  });

  it('leads to « Ajouter un compte… » over the assistant when its harness has no account, and keeps what was typed (ESC-128)', async () => {
    backend({
      account_shareable: () => [],
      account_machine: () => ({ found: true, added: false, connected: false, signsIn: true, email: null, dir: '' }),
    });
    app.settings.accounts = [];
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /Compte et commit/ }));
    const missing = screen.getByText(
      'Aucun compte Claude Code : les agents et les tickets de ce projet ne démarreront qu’une fois un compte ajouté.',
    );
    await userEvent.click(within(missing.parentElement!).getByRole('button', { name: 'Ajouter un compte…' }));
    expect(await screen.findByRole('dialog', { name: 'Ajouter un compte Claude Code' })).toBeInTheDocument();
    // Échap closes the one on top alone.
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Ajouter un compte Claude Code' })).not.toBeInTheDocument());
    // The assistant still there, at its step.
    expect(screen.getByText(/Aucun compte Claude Code : les agents/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Suivant' })).toBeInTheDocument();
  });

  it('fills the worktrees and the launch with what Claude reads of the folder, before the project is made', async () => {
    const be = backend({
      suggest_worktree_steps_at: () => ({ setup: [{ id: 'x', command: 'pnpm i', shell: 'pwsh', cwd: '' }], teardown: [], refused: 0 }),
      suggest_run_commands_at: () => ({ commands: [{ id: 'y', name: 'dev', command: 'pnpm dev', shell: 'pwsh', cwd: '' }], refused: 0 }),
    });
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await next();
    await userEvent.click(screen.getByRole('button', { name: '✦ Remplir automatiquement' }));
    expect(be.called('suggest_worktree_steps_at')[0].args).toEqual({ path: 'C:\\code\\atlas' });
    expect(await screen.findByDisplayValue('pnpm i')).toBeInTheDocument();
    await next();
    await userEvent.click(screen.getByRole('button', { name: '✦ Remplir automatiquement' }));
    expect(await screen.findByDisplayValue('pnpm dev')).toBeInTheDocument();
    expect(be.called('suggest_run_commands_at')[0].args).toEqual({ path: 'C:\\code\\atlas' });
  });

  it('links an integration source when asked, and only then', async () => {
    app.accounts = [{ service: 'github', connected: true, label: '@ada' }];
    const be = backend({
      integration_containers: () => [{ id: 'ada/atlas', name: 'ada/atlas' }],
      integration_states: () => ({ states: [], defaults: {} }),
    });
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /Intégrations/ }));
    const source = screen.getByRole('combobox', { name: 'Source GitHub Issues' });
    await waitFor(() => expect(source).toBeEnabled());
    await userEvent.selectOptions(source, 'ada/atlas');
    await userEvent.click(screen.getByRole('button', { name: 'Suivant' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer le projet' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('update_project')[0].args.project.integrations.links).toEqual([
      { service: 'github', container: 'ada/atlas', name: 'ada/atlas', states: {} },
    ]);
    app.accounts = [];
  });
});

describe('the assistant’s private project and Kanban (ESC-163)', () => {
  const WORK = { id: 'work', name: 'Travail', configDir: 'C:\\w', claudePath: '', active: true };
  const THEIRS = { id: 'theirs', name: 'Autre', configDir: 'C:\\o', claudePath: '', active: true, projects: ['p1'] };

  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), webShop] });
    app.settings.accounts = [...app.settings.accounts, WORK, THEIRS];
    app.modal = { kind: 'newProject' };
  });

  function backend() {
    return fakeBackend({
      inspect_folder: () => folder(),
      inspect_setup: () => setup(),
      create_project: () => made,
      update_project: () => null,
      board_set: ({ settings }) => ({ ...made, board: settings }),
      account_update: ({ account }) => app.settings.accounts.map((a) => (a.id === account.id ? account : a)),
    });
  }

  it('makes a private project with an account reserved to it, its Kanban as set, and says so in the summary', async () => {
    const be = backend();
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());

    // Kanban: its tickets' agents in full.
    await userEvent.click(screen.getByRole('button', { name: /Kanban/ }));
    expect(step()).toBe('kanban');
    await userEvent.click(within(screen.getByRole('group', { name: 'En parallèle' })).getByRole('button', { name: '4' }));
    await userEvent.click(screen.getByRole('switch', { name: 'Attribuer les tickets automatiquement' }));
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Effort' }), 'xhigh');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Mode' }), 'plan');
    await next();

    // Account: « Projet privé », then the accounts it may have to itself (not one reserved to another project).
    expect(step()).toBe('account');
    expect(screen.queryByRole('list', { name: 'Comptes réservés à ce projet' })).toBeNull();
    await userEvent.click(screen.getByRole('switch', { name: 'Projet privé' }));
    const list = screen.getByRole('list', { name: 'Comptes réservés à ce projet' });
    expect(
      within(list)
        .getAllByRole('checkbox')
        .map((c) => c.closest('label')?.textContent?.trim()),
    ).toEqual(['Principal', 'Travail']);
    await userEvent.click(within(list).getByRole('checkbox', { name: 'Travail' }));
    await userEvent.click(screen.getByRole('switch', { name: 'Comptes partagés à la limite' }));

    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    const row = (k: string) => screen.getByText(k, { selector: 'dt' }).nextElementSibling?.textContent;
    expect(row('Projet privé')).toBe('oui · 1 compte réservé · comptes partagés à la limite');
    expect(row('Agents des tickets')).toBe('4 en parallèle · xhigh · plan');
    expect(row('Harness')).toBe('Claude Code');
    await userEvent.click(screen.getByRole('switch', { name: 'Créer un premier agent' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer le projet' }));
    await waitFor(() => expect(app.modal).toBeNull());

    expect(be.called('update_project')[0].args.project).toMatchObject({ id: 'new', private: true, sharedOnLimit: true });
    expect(be.called('account_update').map((c) => c.args.account)).toEqual([expect.objectContaining({ id: 'work', projects: ['new'] })]);
    expect(app.settings.accounts.find((a) => a.id === 'work')?.projects).toEqual(['new']);
    expect(be.called('board_set')[0].args.settings).toMatchObject({ maxParallel: 4, autopilot: false, effort: 'xhigh', mode: 'plan' });
  });

  it('reserves nothing for a project made public again before it is created', async () => {
    const be = backend();
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: /Compte et commit/ }));
    await userEvent.click(screen.getByRole('switch', { name: 'Projet privé' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Travail' }));
    await userEvent.click(screen.getByRole('switch', { name: 'Projet privé' }));
    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    await userEvent.click(screen.getByRole('switch', { name: 'Créer un premier agent' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer le projet' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('update_project')[0].args.project).toMatchObject({ private: false, sharedOnLimit: false });
    expect(be.called('account_update')).toHaveLength(0);
  });

  it('starts private when « Nouveaux projets privés » says so, from the recommended setup, from scratch and from a copy', async () => {
    app.settings.privateProjects = true;
    backend();
    render(NewProjectModal);
    await typePath();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
    for (const start of [/^Configuration recommandée/, /^Partir de zéro/, /^Copier la configuration/]) {
      await userEvent.click(screen.getByRole('radio', { name: start }));
      await waitFor(() => expect(screen.getByRole('button', { name: /Compte et commit/ })).toBeEnabled());
      await userEvent.click(screen.getByRole('button', { name: /Compte et commit/ }));
      expect(screen.getByRole('switch', { name: 'Projet privé' })).toHaveAttribute('aria-checked', 'true');
      await userEvent.click(screen.getByRole('button', { name: /Départ/ }));
    }
  });

  it('takes « Projet privé » and the Kanban from the folder’s .escouade.yml, never public when new projects are private', async () => {
    const preview = (priv: boolean) =>
      project({
        id: '',
        path: 'C:\\code\\atlas',
        private: priv,
        board: board({ model: 'opus', effort: 'high', mode: 'plan' }),
        shared: {
          local: [],
          trusted: '',
          file: { keys: ['private', 'board.model'], values: {}, warnings: [], error: null, version: 1, pending: null },
        },
      });
    for (const [priv, byDefault] of [
      [true, false],
      [false, true],
      [false, false],
    ] as const) {
      app.settings.privateProjects = byDefault;
      fakeBackend({ inspect_folder: () => folder(), inspect_setup: () => setup(), shared_preview: () => preview(priv) });
      const { unmount } = render(NewProjectModal);
      await typePath();
      await waitFor(() =>
        expect(screen.getByRole('radio', { name: /^Réglages de .escouade.yml/ })).toHaveAttribute('aria-checked', 'true'),
      );
      await waitFor(() => expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled());
      await userEvent.click(screen.getByRole('button', { name: /Kanban/ }));
      expect(screen.getByRole('combobox', { name: 'Mode' })).toHaveValue('plan');
      await userEvent.click(screen.getByRole('button', { name: /Compte et commit/ }));
      expect(screen.getByRole('switch', { name: 'Projet privé' })).toHaveAttribute('aria-checked', String(priv || byDefault));
      unmount();
    }
  });

  it('shows an existing private project’s own accounts as reserved already, and reserves only the new ones', async () => {
    resetApp({ projects: [project({ private: true }), webShop] });
    app.settings.accounts = [...app.settings.accounts, WORK, THEIRS];
    app.modal = { kind: 'newProject', projectId: 'p1' };
    const be = backend();
    render(NewProjectModal);
    await userEvent.click(screen.getByRole('button', { name: /Compte et commit/ }));
    expect(screen.getByRole('switch', { name: 'Projet privé' })).toHaveAttribute('aria-checked', 'true');
    const list = screen.getByRole('list', { name: 'Comptes réservés à ce projet' });
    expect(within(list).getByRole('checkbox', { name: /Autre/ })).toBeChecked();
    expect(within(list).getByRole('checkbox', { name: /Autre/ })).toBeDisabled();
    expect(within(list).getByText('déjà réservé')).toBeInTheDocument();
    await userEvent.click(within(list).getByRole('checkbox', { name: 'Travail' }));
    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('account_update').map((c) => c.args.account.id)).toEqual(['work']);
  });
});

describe('the assistant opened again on an existing project', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), webShop] });
    app.modal = { kind: 'newProject', projectId: 'p1' };
  });

  it('starts from its settings, and saves them back without making a project', async () => {
    const be = fakeBackend({
      inspect_folder: () => folder(),
      inspect_setup: () => setup(),
      update_project: () => null,
      board_set: ({ settings }) => ({ ...project(), board: settings }),
    });
    render(NewProjectModal);
    expect(screen.getByRole('dialog', { name: 'Configurer demo-api' })).toBeInTheDocument();
    expect(screen.queryByPlaceholderText(/^C:/)).not.toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /^Garder la configuration actuelle/ })).toHaveAttribute('aria-checked', 'true');
    expect(screen.queryByRole('button', { name: /^Créer sans/ })).not.toBeInTheDocument();
    await next();
    await userEvent.click(screen.getByRole('switch', { name: 'Un worktree par agent' }));
    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    expect(screen.queryByRole('switch', { name: 'Créer un premier agent' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('create_project')).toHaveLength(0);
    expect(be.called('update_project')[0].args.project).toMatchObject({
      id: 'p1',
      worktreePerAgent: true,
      integrations: project().integrations,
    });
    expect(be.called('board_set')[0].args.projectId).toBe('p1');
    expect(app.projects[0].worktreePerAgent).toBe(true);
  });

  it('copies another project into it', async () => {
    const be = fakeBackend({
      inspect_folder: () => folder(),
      inspect_setup: () => setup({ branches: ['main', 'develop'] }),
      update_project: () => null,
      board_set: ({ settings }) => ({ ...project(), board: settings }),
    });
    render(NewProjectModal);
    await userEvent.click(screen.getByRole('radio', { name: /^Copier la configuration/ }));
    expect(screen.getByRole('combobox', { name: 'Projet copié' })).toHaveValue('web');
    await waitFor(() => expect(be.called('inspect_setup').length).toBeGreaterThan(0));
    await next();
    expect(within(screen.getByRole('region', { name: 'Kanban' })).getByText(/branche présente/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Récapitulatif/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(be.called('board_set')[0].args.settings).toMatchObject({ target: 'develop', strategy: 'rebase' });
  });
});

describe('the assistant in English', () => {
  beforeEach(() => {
    setLang('en');
    resetApp({ projects: [project(), webShop] });
    app.ui.activeProject = 'web';
    app.modal = { kind: 'newProject' };
  });

  it('writes its steps, the copy and the checks in English', async () => {
    fakeBackend({ inspect_folder: () => folder(), inspect_setup: () => setup({ hasCommit: false, branches: [], missingDirs: ['web'] }) });
    render(NewProjectModal);
    await typePath();
    expect(await screen.findByText('The repository has no commit')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Make a first commit' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('radio', { name: /^Copy a project’s setup/ }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(screen.getByRole('heading', { name: /Copied settings · web-shop/ })).toBeInTheDocument();
    expect(screen.getByText(/folder “web” not found here/)).toBeInTheDocument();
    expect(screen.getByText(/branch not found: the current branch instead/)).toBeInTheDocument();
    expect(screen.getByText('Not copied: the linked sources (Jira, Trello, GitHub, Linear) and the tickets.')).toBeInTheDocument();
  });
});
