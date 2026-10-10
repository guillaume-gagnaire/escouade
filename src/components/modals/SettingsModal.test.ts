import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { PROJECT_COLORS } from '../../lib/theme';
import type { RunCommand } from '../../lib/types';
import { board, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import SettingsModal from './SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const SHELLS = [
  { id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' },
  { id: 'bash', label: 'Git Bash', path: 'bash.exe' },
];

const tab = (name: string) => screen.getByRole('tab', { name });
const panel = () => screen.getByRole('tabpanel');
/** What is in a group of the settings, under its heading. */
const group = (name: string) => within(within(panel()).getByRole('heading', { name }).closest('section')!);
const save = () => userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
/** The backend as it answers: a board saved comes back in its project. */
const backendSaving = (over: Record<string, (a: any) => unknown> = {}) =>
  fakeBackend({
    save_settings: () => [],
    git_branches: () => ['main', 'release'],
    board_set: (a: any) => ({ ...app.projects.find((p) => p.id === a.projectId), board: a.settings }),
    ...over,
  });

describe('SettingsModal', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ runCommands: [FRONT] }), project({ id: 'p2', name: 'site', board: board({ action: 'pr' }) })] });
    app.shells = SHELLS;
    app.git.p1 = gitInfo();
    app.version = '1.3.1';
    app.modal = { kind: 'settings' };
  });

  it('shows one tab at a time, Claude Code first, with the app’s version under them', async () => {
    fakeBackend();
    render(SettingsModal);
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    expect(
      within(dialog)
        .getAllByRole('tab')
        .map((t) => t.textContent?.trim()),
    ).toEqual(['✳Claude Code', '♪Notifications', '▤Projets', '▦Kanban', '⧉Intégrations', '$_Terminaux', '⇄Réseau', 'ⓘÀ propos']);
    expect(tab('Claude Code')).toHaveAttribute('aria-selected', 'true');
    expect(within(dialog).getByText('Exécutable, modèle et permissions par défaut')).toBeInTheDocument();
    expect(within(panel()).getByRole('textbox', { name: /Chemin de l'exécutable/ })).toBeInTheDocument();
    expect(within(dialog).getByText('Escouade 1.3.1')).toBeInTheDocument();

    await userEvent.click(tab('Réseau'));
    expect(tab('Réseau')).toHaveAttribute('aria-selected', 'true');
    expect(within(panel()).getByRole('heading', { name: 'Proxy' })).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: /Chemin de l'exécutable/ })).not.toBeInTheDocument();
    // Each tab starts at its top, whatever the one before was scrolled to.
    panel().scrollTop = 300;
    await userEvent.click(tab('Terminaux'));
    expect(panel().scrollTop).toBe(0);
    await userEvent.click(tab('Réseau'));
    // Arrows move from tab to tab.
    await userEvent.keyboard('{ArrowDown}');
    expect(tab('À propos')).toHaveAttribute('aria-selected', 'true');
    expect(tab('À propos')).toHaveFocus();
  });

  it('saves the settings of every tab at once', async () => {
    const backend = fakeBackend({ save_settings: () => [{ id: 'pwsh', label: 'PowerShell', path: 'pwsh.exe' }] });
    render(SettingsModal);
    await userEvent.click(tab('Réseau'));
    await userEvent.type(screen.getByPlaceholderText('aucun'), 'http://proxy:3128');
    await userEvent.click(tab('Notifications'));
    const sound = screen.getByRole('switch', { name: 'Son activé' });
    expect(sound).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(sound);
    // Each tab with a change is marked until it is saved.
    expect(tab('Réseau')).toHaveClass('changed');
    expect(tab('Notifications')).toHaveClass('changed');
    expect(tab('Claude Code')).not.toHaveClass('changed');
    expect(backend.called('save_settings')).toHaveLength(0);
    await save();
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ proxyUrl: 'http://proxy:3128', sound: false });
    expect(app.modal).toBeNull();
    expect(app.shells.map((s) => s.id)).toEqual(['pwsh']);
    expect(app.toasts.at(-1)?.text).toBe('Réglages enregistrés');
  });

  it('turns TLS verification off for a proxy with a certificate of its own', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal);
    await userEvent.click(tab('Réseau'));
    const insecure = screen.getByRole('switch', { name: 'Ignorer la vérification des certificats TLS' });
    expect(insecure).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(insecure);
    expect(tab('Réseau')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ insecureTls: true });
    expect(app.settings.insecureTls).toBe(true);
  });

  it('chooses what to be told about, everything on by default', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    await userEvent.click(tab('Notifications'));
    const group = within(panel()).getByRole('heading', { name: 'Me prévenir pour' }).closest('section')!;
    const labels = ['Questions et autorisations', 'Tâches terminées', 'Erreurs', 'Tickets (prêt à tester, bloqué)'];
    expect(
      within(group)
        .getAllByRole('switch')
        .map((s) => s.getAttribute('aria-label')),
    ).toEqual(labels);
    for (const label of labels) expect(within(group).getByRole('switch', { name: label })).toHaveAttribute('aria-checked', 'true');
    expect(tab('Notifications')).not.toHaveClass('changed');
    // Off, a type is silent outside the window; what blinks or flashes still does.
    expect(group).toHaveTextContent(
      /Désactivé : ni notification système ni carillon pour ce type ; l’onglet, la carte et (la barre des tâches|le Dock) signalent toujours l’agent\./,
    );

    await userEvent.click(within(group).getByRole('switch', { name: 'Tâches terminées' }));
    expect(within(group).getByRole('switch', { name: 'Tâches terminées' })).toHaveAttribute('aria-checked', 'false');
    expect(within(group).getByRole('switch', { name: 'Erreurs' })).toHaveAttribute('aria-checked', 'true');
    expect(tab('Notifications')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings.notifyFor).toEqual({
      questions: true,
      done: false,
      errors: true,
      tickets: true,
    });
    expect(app.settings.notifyFor.done).toBe(false);
  });

  it('sets the defaults of new agents, named with the version Claude Code runs', async () => {
    const backend = backendSaving();
    app.models = [{ value: 'sonnet', resolvedModel: 'claude-sonnet-5-5' }];
    render(SettingsModal);
    const model = screen.getByRole('group', { name: 'Modèle par défaut' });
    expect(within(model).getByRole('button', { name: 'Sonnet 5.5' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(model).getByRole('button', { name: /Opus/ }));
    await userEvent.click(within(screen.getByRole('group', { name: 'Effort par défaut' })).getByRole('button', { name: 'Max' }));
    await userEvent.click(
      within(screen.getByRole('group', { name: 'Mode de permission par défaut' })).getByRole('button', { name: 'Plan' }),
    );
    const resume = screen.getByRole('switch', { name: 'Reprise automatique après la limite d’usage' });
    expect(resume).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(resume);
    await userEvent.clear(screen.getByRole('spinbutton', { name: /Arrêter les processus Claude inactifs/ }));
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({
      defaultModel: 'opus',
      defaultEffort: 'max',
      defaultMode: 'plan',
      autoResume: false,
      idleStopMinutes: 0,
    });
  });

  it('changes nothing when cancelled, nor when closed', async () => {
    const backend = fakeBackend();
    render(SettingsModal);
    await userEvent.click(tab('Terminaux'));
    await userEvent.type(screen.getByRole('textbox', { name: /Git Bash/ }), 'C:\\Git\\bin\\bash.exe');
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    expect(backend.calls).toEqual([]);
    expect(app.settings.bashPath).toBe('');
  });

  it('opens on the tab and the project asked for, and switches project', async () => {
    backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p2' });
    expect(tab('Kanban')).toHaveAttribute('aria-selected', 'true');
    await expect.poll(() => document.activeElement).toBe(tab('Kanban'));
    const projects = screen.getByRole('group', { name: 'Projet' });
    expect(within(projects).getByRole('button', { name: 'site' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('radio', { name: /Ouvrir une pull request/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(within(projects).getByRole('button', { name: 'demo-api' }));
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toHaveAttribute('aria-checked', 'true');
    // No project chooser where the settings are the app's.
    await userEvent.click(tab('Réseau'));
    expect(screen.queryByRole('group', { name: 'Projet' })).not.toBeInTheDocument();
  });

  it('sets what validating a ticket does, saved with the rest', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(await screen.findByRole('button', { name: '⎇ release' }));
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    expect(screen.queryByRole('switch', { name: 'PR en brouillon' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('radio', { name: /Ouvrir une pull request/ }));
    expect(screen.queryByRole('button', { name: 'Rebase' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'PR en brouillon' }));
    expect(backend.called('board_set')).toHaveLength(0);
    await save();
    expect(backend.called('board_set')[0].args).toMatchObject({
      projectId: 'p1',
      settings: { action: 'pr', draft: true, target: 'release', strategy: 'rebase' },
    });
    expect(app.projects[0].board).toMatchObject({ action: 'pr', target: 'release' });
    expect(app.modal).toBeNull();
  });

  it('hides the target branch when the work is only pushed or kept', async () => {
    backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    await userEvent.click(screen.getByRole('radio', { name: /Laisser en l'état/ }));
    expect(screen.queryByText('Branche cible')).not.toBeInTheDocument();
  });

  it('keeps the tests switch off until a command is given, and previews the commit message', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    const tests = screen.getByRole('switch', { name: 'Relancer les tests avant' });
    expect(tests).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Commande de tests' }), 'npm test');
    expect(tests).toBeEnabled();
    await userEvent.click(tests);
    expect(screen.getByText('feat: limiter les tentatives de connexion [DEM-42]')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Message de commit généré' }));
    expect(screen.getByText('DEM-42 Limiter les tentatives de connexion')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Supprimer le worktree une fois validé' }));
    await save();
    expect(backend.called('board_set')[0].args.settings).toMatchObject({
      testCommand: 'npm test',
      testsFirst: true,
      conventional: false,
      cleanup: false,
    });
  });

  it('sets the conflict policy, the autopilot, the agents in parallel and their model', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    await userEvent.click(within(screen.getByRole('group', { name: 'En cas de conflit' })).getByRole('button', { name: "L'agent résout" }));
    await userEvent.click(screen.getByRole('switch', { name: 'Attribuer les tickets automatiquement' }));
    await userEvent.click(within(screen.getByRole('group', { name: 'En parallèle' })).getByRole('button', { name: '4' }));
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Modèle' }), 'opus');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Mode' }), 'plan');
    await save();
    expect(backend.called('board_set')[0].args.settings).toMatchObject({
      conflict: 'agent',
      autopilot: false,
      maxParallel: 4,
      model: 'opus',
      mode: 'plan',
    });
  });

  it('pauses the autopilot past a quota, for every project', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    const pause = screen.getByRole('group', { name: 'Pause au-delà du quota' });
    expect(
      within(pause)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['80 %', '90 %', '95 %', '100 %']);
    expect(within(pause).getByRole('button', { name: '100 %' })).toHaveAttribute('aria-pressed', 'true');
    expect(
      screen.getByText('Aucun ticket ne démarre tant que la fenêtre de 5 h ou la fenêtre hebdomadaire dépasse ce seuil.'),
    ).toBeInTheDocument();
    expect(screen.getByText('pour tous les projets')).toBeInTheDocument();
    await userEvent.click(within(pause).getByRole('button', { name: '90 %' }));
    expect(tab('Kanban')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ quotaPause: 90 });
    // An app setting: no board changes.
    expect(backend.called('board_set')).toHaveLength(0);
    expect(app.settings.quotaPause).toBe(90);
  });

  it('keeps the modal open, the changes in it, when a part could not be saved', async () => {
    backendSaving({
      board_set: () => {
        throw 'disque plein';
      },
    });
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    await save();
    expect(app.toasts.at(-1)?.text).toContain('disque plein');
    expect(app.modal).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Rebase' })).toHaveAttribute('aria-pressed', 'true');
    expect(app.projects[0].board.strategy).toBe('squash');
  });

  it('sets a project’s name, color and worktrees', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p2' });
    expect(screen.getByText('C:\\code\\demo-api')).toBeInTheDocument();
    const name = screen.getByRole('textbox', { name: 'Nom du projet' });
    await userEvent.clear(name);
    await userEvent.type(name, 'vitrine');
    const colors = screen.getByRole('group', { name: 'Couleur' });
    expect(within(colors).getAllByRole('button')).toHaveLength(PROJECT_COLORS.length);
    await userEvent.click(within(colors).getByRole('button', { name: 'Couleur 4' }));
    expect(within(colors).getByRole('button', { name: 'Couleur 4' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('switch', { name: 'Un worktree par agent' }));
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({
      id: 'p2',
      name: 'vitrine',
      color: PROJECT_COLORS[3],
      worktreePerAgent: true,
    });
    expect(app.projects[1].name).toBe('vitrine');
  });

  it('edits, adds and saves the project’s launch commands', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const front = screen.getByRole('group', { name: 'Commande 1' });
    expect(within(front).getByLabelText('Nom')).toHaveValue('Front');
    expect(within(front).getByLabelText('Shell')).toHaveValue('pwsh');
    expect(within(front).getByLabelText(/Sous-dossier/)).toHaveValue('web');

    await userEvent.click(screen.getByRole('button', { name: '+ Ajouter une commande' }));
    const api = screen.getByRole('group', { name: 'Commande 2' });
    // A new command runs with the first detected shell, in the project's folder.
    expect(within(api).getByLabelText('Shell')).toHaveValue('pwsh');
    await userEvent.type(within(api).getByLabelText('Nom'), 'API');
    await userEvent.type(within(api).getByLabelText('Commande'), 'cargo run');
    await userEvent.selectOptions(within(api).getByLabelText('Shell'), 'bash');
    await save();

    const saved = backend.called('update_project')[0].args.project.runCommands;
    expect(saved).toEqual([FRONT, { id: expect.any(String), name: 'API', command: 'cargo run', shell: 'bash', cwd: '' }]);
    expect(app.projects[0].runCommands).toEqual(saved);
    expect(app.modal).toBeNull();
  });

  it('needs a name and a command line for each command, and a name for each project', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.clear(within(screen.getByRole('group', { name: 'Commande 1' })).getByLabelText('Commande'));
    expect(screen.getByRole('button', { name: 'Enregistrer' })).toBeDisabled();
    expect(screen.getByText('Chaque commande de lancement de « demo-api » demande un nom et une ligne de commande.')).toBeInTheDocument();
    await userEvent.type(within(screen.getByRole('group', { name: 'Commande 1' })).getByLabelText('Commande'), 'npm start');
    await userEvent.clear(screen.getByRole('textbox', { name: 'Nom du projet' }));
    expect(screen.getByRole('button', { name: 'Enregistrer' })).toBeDisabled();
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('removes a command, stopping it if it ran', async () => {
    const backend = backendSaving();
    app.launches.c1 = { status: 'running', ptyId: 't1', name: 'Front', stopping: false, code: null, startedAt: 1 };
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(within(screen.getByRole('group', { name: 'Commande 1' })).getByRole('button', { name: 'Supprimer' }));
    expect(screen.getByText("Aucune commande pour l'instant.")).toBeInTheDocument();
    await save();
    expect(backend.called('update_project')[0].args.project.runCommands).toEqual([]);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    expect(app.launches.c1).toBeUndefined();
  });

  it('chooses who writes the commits, the agent by default', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Commit' });
    expect(within(choice).getByRole('button', { name: "Rédigé par l'agent" })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('Direct : Escouade propose un message, tu le relis et tu commites toi-même.')).toBeInTheDocument();
    await userEvent.click(within(choice).getByRole('button', { name: 'Direct, avec un message proposé' }));
    expect(within(choice).getByRole('button', { name: 'Direct, avec un message proposé' })).toHaveAttribute('aria-pressed', 'true');
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', commitMode: 'direct' });
    expect(app.projects[0].commitMode).toBe('direct');
  });

  it('saves the files copied into new worktrees', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const field = screen.getByRole('textbox', { name: /Fichiers copiés dans les worktrees/ });
    expect(field).toHaveValue('.env*');
    // Only what git ignores is copied: a copy never shows as a change to commit.
    expect(field).toHaveAccessibleDescription(/^Seuls ceux que git ignore/);
    await userEvent.type(field, '{Enter}**/.env.local');
    await save();
    expect(backend.called('update_project')[0].args.project.worktreeCopy).toEqual(['.env*', '**/.env.local']);
  });

  it('sets the commands run in new worktrees, by hand or as Claude suggests them', async () => {
    let answer!: (v: unknown) => void;
    const backend = backendSaving({ suggest_worktree_steps: () => new Promise((r) => (answer = r)) });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter une commande de démontage' }));
    const down = screen.getByRole('group', { name: 'Commande de démontage 1' });
    expect(within(down).getByLabelText('Shell')).toHaveValue('pwsh');
    await userEvent.type(within(down).getByLabelText('Commande'), 'docker compose down');
    const worktrees = group('Worktrees');
    await userEvent.click(worktrees.getByRole('button', { name: '✦ Remplir automatiquement' }));
    expect(worktrees.getByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
    // The launch commands have their own button, which keeps its label.
    expect(group('Lancement').getByRole('button', { name: '✦ Remplir automatiquement' })).toBeEnabled();
    expect(backend.called('suggest_worktree_steps')[0].args).toEqual({ projectId: 'p1' });
    expect(backend.called('suggest_run_commands')).toHaveLength(0);
    answer({ setup: [{ id: 's1', command: 'npm ci', shell: 'bash', cwd: 'web' }], teardown: [], refused: 0 });
    const setup = await screen.findByRole('group', { name: 'Commande de préparation 1' });
    expect(within(setup).getByLabelText('Commande')).toHaveValue('npm ci');
    expect(within(setup).getByLabelText('Shell')).toHaveValue('bash');
    expect(within(setup).getByLabelText('Sous-dossier')).toHaveValue('web');
    // The suggestion replaces both lists, and the launch commands stay.
    expect(screen.queryByRole('group', { name: 'Commande de démontage 1' })).toBeNull();
    expect(worktrees.getByRole('button', { name: '✦ Remplir automatiquement' })).toBeEnabled();
    expect(screen.getByDisplayValue('Front')).toBeInTheDocument();
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({
      worktreeSetup: [{ id: 's1', command: 'npm ci', shell: 'bash', cwd: 'web' }],
      worktreeTeardown: [],
      runCommands: [FRONT],
    });
  });

  it('sets the launch commands as Claude suggests them, in place of those of the draft', async () => {
    let answer!: (v: unknown) => void;
    const backend = backendSaving({ suggest_run_commands: () => new Promise((r) => (answer = r)) });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const launch = group('Lancement');
    expect(
      launch.getByText(
        "Claude lit le projet (manifestes, scripts, docker-compose, README…) sans rien modifier et propose les commandes à lancer. Relis-les avant d'enregistrer.",
      ),
    ).toBeInTheDocument();
    // Not before it is asked for.
    expect(backend.called('suggest_run_commands')).toHaveLength(0);
    await userEvent.click(launch.getByRole('button', { name: '✦ Remplir automatiquement' }));
    expect(launch.getByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
    // The worktrees' button is another reading: it stays available.
    expect(group('Worktrees').getByRole('button', { name: '✦ Remplir automatiquement' })).toBeEnabled();
    expect(backend.called('suggest_run_commands')[0].args).toEqual({ projectId: 'p1' });
    expect(backend.called('suggest_worktree_steps')).toHaveLength(0);
    answer({
      commands: [
        { id: 's1', name: 'API', command: 'cargo watch -x run', shell: 'bash', cwd: '' },
        { id: 's2', name: 'Web', command: 'npm run dev', shell: 'bash', cwd: 'web' },
      ],
      refused: 1,
    });
    // In place of the draft's, FRONT.
    await screen.findByDisplayValue('API');
    expect(screen.queryByDisplayValue('Front')).toBeNull();
    const first = screen.getByRole('group', { name: 'Commande 1' });
    expect(within(first).getByLabelText('Nom')).toHaveValue('API');
    expect(within(first).getByLabelText('Commande')).toHaveValue('cargo watch -x run');
    expect(within(first).getByLabelText('Shell')).toHaveValue('bash');
    expect(within(first).getByLabelText('Sous-dossier', { exact: false })).toHaveValue('');
    const second = screen.getByRole('group', { name: 'Commande 2' });
    expect(within(second).getByLabelText('Sous-dossier', { exact: false })).toHaveValue('web');
    expect(screen.queryByRole('group', { name: 'Commande 3' })).toBeNull();
    expect(launch.getByRole('button', { name: '✦ Remplir automatiquement' })).toBeEnabled();
    expect(tab('Projets')).toHaveClass('changed');
    // What was left out is said, not lost in silence.
    expect(app.toasts.at(-1)?.text).toBe(
      "2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d'enregistrer.",
    );
    await save();
    expect(backend.called('update_project')[0].args.project.runCommands).toEqual([
      { id: 's1', name: 'API', command: 'cargo watch -x run', shell: 'bash', cwd: '' },
      { id: 's2', name: 'Web', command: 'npm run dev', shell: 'bash', cwd: 'web' },
    ]);
  });

  it('tells the two « ✦ Remplir automatiquement » buttons apart by the text of their own row', () => {
    backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const button = { name: '✦ Remplir automatiquement' };
    expect(group('Worktrees').getByRole('button', button)).toHaveAccessibleDescription(/lockfiles.*propose les commandes\. Relis-les/);
    expect(group('Lancement').getByRole('button', button)).toHaveAccessibleDescription(/docker-compose.*commandes à lancer\. Relis-les/);
  });

  it('keeps the launch commands when Claude proposes none readable, and says so', async () => {
    backendSaving({ suggest_run_commands: () => Promise.reject("Claude n'a pas proposé de commandes lisibles.") });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(group('Lancement').getByRole('button', { name: '✦ Remplir automatiquement' }));
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ text: "Claude n'a pas proposé de commandes lisibles.", kind: 'error' });
    expect(screen.getByDisplayValue('Front')).toBeInTheDocument();
    expect(group('Lancement').getByRole('button', { name: '✦ Remplir automatiquement' })).toBeEnabled();
  });

  it('reads the project at once when it is opened to propose the launch commands, but not when it comes back', async () => {
    let answer!: (v: unknown) => void;
    const backend = backendSaving({ suggest_run_commands: () => new Promise((r) => (answer = r)) });
    const { unmount } = render(SettingsModal, { tab: 'projects', projectId: 'p1', section: 'launch', suggest: true });
    expect(backend.called('suggest_run_commands')[0].args).toEqual({ projectId: 'p1' });
    expect(group('Lancement').getByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
    answer({ commands: [{ id: 's1', name: 'Web', command: 'npm start', shell: 'bash', cwd: '' }], refused: 0 });
    expect(await screen.findByDisplayValue('Web')).toBeInTheDocument();
    unmount();
    // Back from a modal it opened (the project's closing): the draft as it was, nothing asked again.
    render(SettingsModal, { tab: 'projects', projectId: 'p1', section: 'launch', suggest: true, resume: true });
    expect(backend.called('suggest_run_commands')).toHaveLength(1);
    expect(screen.getByDisplayValue('Web')).toBeInTheDocument();
  });

  it('asks before closing the project', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p2' });
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Fermer « site » ?', confirm: 'Fermer le projet' });
  });

  it('says there is no project to set when none is open', async () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(SettingsModal, { tab: 'projects' });
    expect(screen.getByText('Aucun projet ouvert.')).toBeInTheDocument();
    await userEvent.click(tab('Kanban'));
    expect(screen.getByText('Aucun projet ouvert.')).toBeInTheDocument();
  });

  it('checks for an update on demand', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'about' });
    expect(screen.getByText('Escouade 1.3.1', { selector: '[role="tabpanel"] *' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Aucune mise à jour disponible.');
  });

  it('installs updates by itself unless told not to', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { tab: 'about' });
    const auto = screen.getByRole('switch', { name: 'Installer les mises à jour automatiquement' });
    expect(auto).toHaveAttribute('aria-checked', 'true');
    expect(
      screen.getByText('Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré.'),
    ).toBeInTheDocument();
    await userEvent.click(auto);
    expect(tab('À propos')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ autoUpdate: false });
    expect(app.settings.autoUpdate).toBe(false);
  });

  it('keeps the focus inside, from the tab it opens on', async () => {
    fakeBackend();
    const outside = document.createElement('button');
    document.body.prepend(outside);
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    await expect.poll(() => document.activeElement).toBe(tab('Kanban'));
    await userEvent.tab({ shift: true });
    expect(dialog.contains(document.activeElement)).toBe(true);
    outside.remove();
  });

  it('goes to the first and last tabs with Home and End', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'network' });
    tab('Réseau').focus();
    await userEvent.keyboard('{Home}');
    expect(tab('Claude Code')).toHaveAttribute('aria-selected', 'true');
    expect(tab('Claude Code')).toHaveFocus();
    await userEvent.keyboard('{End}');
    expect(tab('À propos')).toHaveAttribute('aria-selected', 'true');
  });

  it('tells a screen reader which tabs have changes not saved yet', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'network' });
    expect(tab('Réseau')).not.toHaveAccessibleDescription();
    await userEvent.type(screen.getByPlaceholderText('aucun'), 'http://proxy:3128');
    expect(tab('Réseau')).toHaveAccessibleDescription('Modifié, pas encore enregistré');
  });

  it('opens on the launch commands when asked for them', async () => {
    fakeBackend();
    const scrolled = vi.spyOn(Element.prototype, 'scrollIntoView');
    render(SettingsModal, { tab: 'projects', projectId: 'p1', section: 'launch' });
    await expect.poll(() => scrolled.mock.contexts.length).toBeGreaterThan(0);
    const target = scrolled.mock.contexts.at(-1) as HTMLElement;
    expect(within(target).getByRole('heading', { name: 'Lancement' })).toBeInTheDocument();
    scrolled.mockRestore();
  });

  it('comes back with its draft as it was', async () => {
    fakeBackend();
    settingsForm.open({ tab: 'network' });
    settingsForm.settings.proxyUrl = 'http://proxy:3128';
    settingsForm.tab = 'projects';
    render(SettingsModal, { tab: 'network', resume: true });
    expect(tab('Réseau')).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByPlaceholderText('aucun')).toHaveValue('http://proxy:3128');
  });

  it('does not close when a selection dragged from a field ends outside', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'network' });
    const overlay = screen.getByRole('dialog', { name: 'Réglages' }).parentElement!;
    await fireEvent.mouseDown(screen.getByPlaceholderText('aucun'));
    await fireEvent.mouseUp(overlay);
    await fireEvent.click(overlay);
    expect(app.modal).not.toBeNull();
    await fireEvent.mouseDown(overlay);
    await fireEvent.click(overlay);
    expect(app.modal).toBeNull();
  });

  it('names the Kanban agents’ default after the default model chosen, saved or not', async () => {
    fakeBackend({ git_branches: () => ['main'] });
    render(SettingsModal);
    await userEvent.click(within(screen.getByRole('group', { name: 'Modèle par défaut' })).getByRole('button', { name: /Opus/ }));
    await userEvent.click(tab('Kanban'));
    expect(within(screen.getByRole('combobox', { name: 'Modèle' })).getByRole('option', { name: 'Par défaut (Opus)' })).toBeInTheDocument();
  });

  it('has no editor section any more', () => {
    fakeBackend();
    render(SettingsModal);
    expect(screen.queryByRole('heading', { name: 'Éditeur' })).not.toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: /Éditeur par défaut/ })).not.toBeInTheDocument();
  });
});
