import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
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

  it('shows one tab at a time, Application first, opens on Claude Code, with the app’s version under them', async () => {
    fakeBackend();
    render(SettingsModal);
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    expect(
      within(dialog)
        .getAllByRole('tab')
        .map((t) => t.textContent?.trim()),
    ).toEqual([
      'AaApplication',
      '✳Claude Code',
      '◎Comptes Claude',
      '♪Notifications',
      '▤Projets',
      '▦Kanban',
      '⧉Intégrations',
      '$_Terminaux',
      '⇄Réseau',
      'ⓘÀ propos',
    ]);
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

  it('lets a ticket’s agent stopped by the limit go on on another account, unless told not to', async () => {
    const backend = backendSaving();
    app.settings.accounts = [
      { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal);
    const move = screen.getByRole('switch', { name: 'Reprendre sur un autre compte un agent de ticket arrêté par la limite' });
    // On by default, beside the automatic resume it is the alternative to.
    expect(move).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByText('Sinon il attend la remise à zéro de son compte')).toBeInTheDocument();
    await userEvent.click(move);
    expect(tab('Claude Code')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ switchOnLimit: false });
  });

  it('has no such switch with a single account, nothing to go on on', () => {
    backendSaving();
    render(SettingsModal);
    expect(screen.getByRole('switch', { name: 'Reprise automatique après la limite d’usage' })).toBeInTheDocument();
    expect(screen.queryByRole('switch', { name: /Reprendre sur un autre compte/ })).not.toBeInTheDocument();
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

  it('offers no preferred account with a single account', () => {
    backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.queryByRole('group', { name: 'Compte préféré' })).not.toBeInTheDocument();
  });

  it('chooses the account a project’s agents and tickets go to, « Automatique » by default', async () => {
    const backend = backendSaving();
    app.settings.accounts = [
      ...app.settings.accounts,
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
      { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false },
    ];
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Compte préféré' });
    // The account switched off is not offered.
    expect(
      within(choice)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Automatique', 'Principal', 'Pro']);
    expect(within(choice).getByRole('button', { name: 'Automatique' })).toHaveAttribute('aria-pressed', 'true');
    expect(choice.closest('.row')).toHaveTextContent('Le compte sur lequel partent les nouveaux agents et les tickets de ce projet.');
    await userEvent.click(within(choice).getByRole('button', { name: 'Pro' }));
    expect(within(choice).getByRole('button', { name: 'Pro' })).toHaveAttribute('aria-pressed', 'true');
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', account: 'pro' });
    expect(app.projects[0].account).toBe('pro');
  });

  it('goes back to « Automatique », and shows an account switched off since that the project still prefers', async () => {
    const backend = backendSaving();
    app.projects[0].account = 'team';
    app.settings.accounts = [
      ...app.settings.accounts,
      { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false },
    ];
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Compte préféré' });
    expect(within(choice).getByRole('button', { name: 'Équipe (inactif)' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(choice).getByRole('button', { name: 'Automatique' }));
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', account: '' });
  });

  it('names the preferred account in English', async () => {
    setLang('en');
    backendSaving();
    app.settings.accounts = [
      ...app.settings.accounts,
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Preferred account' });
    expect(
      within(choice)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Automatic', 'Main', 'Pro']);
    expect(choice.closest('.row')).toHaveTextContent('The account this project’s new agents and tickets go to.');
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
    // Longer than its field: its end would only be seen by scrolling it, and it runs by itself in each new worktree.
    const long = 'npm ci && npx prisma generate --schema ./prisma/schema.prisma && npm run build:workers -- --filter web';
    answer({
      setup: [
        { id: 's1', command: 'npm ci', shell: 'bash', cwd: 'web' },
        { id: 's2', command: long, shell: 'pwsh', cwd: '' },
      ],
      teardown: [],
      refused: 0,
    });
    // Read in full first: the draft is left as it is until the proposal is taken.
    const proposal = within(await worktrees.findByRole('region', { name: 'Commandes de worktree proposées' }));
    expect(proposal.getByText(long)).toBeInTheDocument();
    expect(proposal.getByText('npm ci')).toBeInTheDocument();
    expect(proposal.getByText('web')).toBeInTheDocument();
    expect(proposal.getByText('la racine du worktree')).toBeInTheDocument();
    expect(proposal.getByText('Git Bash')).toBeInTheDocument();
    // When each list runs by itself, once saved.
    expect(
      proposal.getByText(
        "Proposition de Claude : relis chaque commande en entier. Elles remplacent les deux listes et, une fois enregistrées, tournent seules : la préparation à l'ouverture de chaque nouveau worktree, le démontage avant sa suppression.",
      ),
    ).toBeInTheDocument();
    // The steps of the draft it would remove are said too.
    expect(proposal.getByText('Aucune commande.')).toBeInTheDocument();
    expect(screen.queryByRole('group', { name: 'Commande de préparation 1' })).toBeNull();
    expect(within(down).getByLabelText('Commande')).toHaveValue('docker compose down');
    await userEvent.click(proposal.getByRole('button', { name: 'Remplacer les commandes' }));
    expect(worktrees.queryByRole('region', { name: 'Commandes de worktree proposées' })).toBeNull();
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
      worktreeSetup: [
        { id: 's1', command: 'npm ci', shell: 'bash', cwd: 'web' },
        { id: 's2', command: long, shell: 'pwsh', cwd: '' },
      ],
      worktreeTeardown: [],
      runCommands: [FRONT],
    });
  });

  it('leaves the draft as it was when Claude’s proposal is ignored', async () => {
    const backend = backendSaving({
      suggest_worktree_steps: () => ({ setup: [{ id: 's1', command: 'npm ci', shell: 'bash', cwd: '' }], teardown: [], refused: 0 }),
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(group('Worktrees').getByRole('button', { name: '✦ Remplir automatiquement' }));
    const proposal = await group('Worktrees').findByRole('region', { name: 'Commandes de worktree proposées' });
    // The reading's button was given up: the proposal takes the focus, to be read from the keyboard.
    expect(proposal).toHaveFocus();
    // Still there when coming back to the tab, which keeps the focus.
    await userEvent.click(tab('Réseau'));
    await userEvent.click(tab('Projets'));
    expect(tab('Projets')).toHaveFocus();
    const worktrees = group('Worktrees');
    const back = within(worktrees.getByRole('region', { name: 'Commandes de worktree proposées' }));
    expect(back.getByText('npm ci')).toBeInTheDocument();
    await userEvent.click(back.getByRole('button', { name: 'Ignorer' }));
    expect(worktrees.getByRole('button', { name: '✦ Remplir automatiquement' })).toHaveFocus();
    expect(worktrees.queryByRole('region', { name: 'Commandes de worktree proposées' })).toBeNull();
    expect(screen.queryByRole('group', { name: 'Commande de préparation 1' })).toBeNull();
    expect(tab('Projets')).not.toHaveClass('changed');
    await save();
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('reorders the commands run in new worktrees with their buttons, or Alt+↑ and Alt+↓ on one of them', async () => {
    const steps = ['npm ci', 'npm run gen', 'npm run db'].map((command, i) => ({ id: `s${i}`, command, shell: 'pwsh', cwd: '' }));
    app.projects = [project({ worktreeSetup: steps })];
    const backend = backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const step = (n: number) => within(screen.getByRole('group', { name: `Commande de préparation ${n}` }));
    const commands = () => [1, 2, 3].map((n) => (step(n).getByLabelText('Commande') as HTMLInputElement).value);
    const button = (name: string) => screen.getByRole('button', { name });
    // Nowhere to go past the ends.
    expect(button('Monter la commande 1')).toBeDisabled();
    expect(button('Descendre la commande 1')).toBeEnabled();
    expect(button('Monter la commande 3')).toBeEnabled();
    expect(button('Descendre la commande 3')).toBeDisabled();
    expect(button('Monter la commande 2')).toHaveAttribute('aria-keyshortcuts', 'Alt+ArrowUp');
    await userEvent.click(button('Descendre la commande 1'));
    expect(commands()).toEqual(['npm run gen', 'npm ci', 'npm run db']);
    await userEvent.click(button('Monter la commande 3'));
    expect(commands()).toEqual(['npm run gen', 'npm run db', 'npm ci']);
    // A button with no way left to go hands the focus to the other one.
    await userEvent.click(button('Monter la commande 2'));
    expect(commands()).toEqual(['npm run db', 'npm run gen', 'npm ci']);
    expect(button('Descendre la commande 1')).toHaveFocus();
    // From the keyboard, on any field of the step: the focus goes with it.
    const field = step(3).getByLabelText('Commande');
    await userEvent.click(field);
    await userEvent.keyboard('{Alt>}{ArrowUp}{/Alt}');
    expect(commands()).toEqual(['npm run db', 'npm ci', 'npm run gen']);
    expect(field).toHaveFocus();
    await userEvent.keyboard('{Alt>}{ArrowUp}{ArrowUp}{/Alt}');
    expect(commands()).toEqual(['npm ci', 'npm run db', 'npm run gen']);
    expect(field).toHaveFocus();
    await userEvent.keyboard('{Alt>}{ArrowDown}{/Alt}');
    expect(commands()).toEqual(['npm run db', 'npm ci', 'npm run gen']);
    // Ctrl+Alt is AltGr on French keyboards: nothing moves.
    await userEvent.keyboard('{Control>}{Alt>}{ArrowDown}{/Alt}{/Control}');
    expect(commands()).toEqual(['npm run db', 'npm ci', 'npm run gen']);
    await save();
    expect(backend.called('update_project')[0].args.project.worktreeSetup.map((s: { id: string }) => s.id)).toEqual(['s2', 's0', 's1']);
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
    // What was left out is said, not lost in silence.
    await expect
      .poll(() => app.toasts.at(-1)?.text)
      .toBe("2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d'enregistrer.");
    // Read in full first, in place of nothing yet.
    const proposal = within(await launch.findByRole('region', { name: 'Commandes de lancement proposées' }));
    expect(proposal.getByText('API')).toBeInTheDocument();
    expect(proposal.getByText('cargo watch -x run')).toBeInTheDocument();
    expect(proposal.getByText('le dossier du projet')).toBeInTheDocument();
    expect(proposal.getByText('npm run dev')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Front')).toBeInTheDocument();
    expect(tab('Projets')).not.toHaveClass('changed');
    // In place of the draft's, FRONT.
    await userEvent.click(proposal.getByRole('button', { name: 'Remplacer les commandes' }));
    expect(launch.queryByRole('region', { name: 'Commandes de lancement proposées' })).toBeNull();
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
    await save();
    expect(backend.called('update_project')[0].args.project.runCommands).toEqual([
      { id: 's1', name: 'API', command: 'cargo watch -x run', shell: 'bash', cwd: '' },
      { id: 's2', name: 'Web', command: 'npm run dev', shell: 'bash', cwd: 'web' },
    ]);
  });

  it('spells out in Claude’s proposal what a command would hide', async () => {
    // Written by their code: invisible in this file, they would hide from its reader too.
    const ZWSP = String.fromCodePoint(0x200b);
    const RLO = String.fromCodePoint(0x202e);
    backendSaving({
      suggest_run_commands: () => ({
        commands: [{ id: 's1', name: `We${ZWSP}b`, command: `npm${RLO} run dev`, shell: 'zsh', cwd: `a${ZWSP}pp` }],
        refused: 0,
      }),
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(group('Lancement').getByRole('button', { name: '✦ Remplir automatiquement' }));
    const proposal = within(await screen.findByRole('region', { name: 'Commandes de lancement proposées' }));
    expect(proposal.getByText('We⟨U+200B⟩b')).toBeInTheDocument();
    expect(proposal.getByText('npm⟨U+202E⟩ run dev')).toBeInTheDocument();
    expect(proposal.getByText('a⟨U+200B⟩pp')).toBeInTheDocument();
    // A shell this machine does not have.
    expect(proposal.getByText('zsh (introuvable)')).toBeInTheDocument();
  });

  it('styles Claude’s proposal with classes of its own, leaving the commands around it as they are', async () => {
    app.projects = [project({ runCommands: [FRONT], worktreeSetup: [{ id: 'w1', command: 'npm ci', shell: 'pwsh', cwd: '' }] })];
    backendSaving({
      suggest_worktree_steps: () => ({ setup: [{ id: 's1', command: 'npm ci', shell: 'pwsh', cwd: 'web' }], teardown: [], refused: 0 }),
      suggest_run_commands: () => ({ commands: [{ id: 's2', name: 'API', command: 'cargo run', shell: 'pwsh', cwd: '' }], refused: 0 }),
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(group('Worktrees').getByRole('button', { name: '✦ Remplir automatiquement' }));
    await userEvent.click(group('Lancement').getByRole('button', { name: '✦ Remplir automatiquement' }));
    const proposals = [
      await screen.findByRole('region', { name: 'Commandes de worktree proposées' }),
      await screen.findByRole('region', { name: 'Commandes de lancement proposées' }),
    ];
    // A rule of the tab applies to whatever has its scope's class and the rule's: a class of the proposal that the rest
    // of the tab uses too restyles both (a launch command's box, the proposal's commands). The app's own classes
    // (app.css), which the tab does not style, aside.
    const global = new Set(['btn', 'ghost', 'primary', 'mono', 'section-label']);
    const scope = [...proposals[0].classList].find((c) => c.startsWith('svelte-'))!;
    const classes = (inside: boolean) =>
      new Set(
        [...panel().querySelectorAll(`.${scope}`)]
          .filter((e) => proposals.some((p) => p.contains(e)) === inside)
          .flatMap((e) => [...e.classList])
          .filter((c) => c !== scope && !global.has(c)),
      );
    const others = classes(false);
    expect(others.size).toBeGreaterThan(0);
    expect([...classes(true)].filter((c) => others.has(c))).toEqual([]);
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
    const proposal = await group('Lancement').findByRole('region', { name: 'Commandes de lancement proposées' });
    await userEvent.click(within(proposal).getByRole('button', { name: 'Remplacer les commandes' }));
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
    expect(tab('Application')).toHaveAttribute('aria-selected', 'true');
    expect(tab('Application')).toHaveFocus();
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

describe('SettingsModal in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ runCommands: [FRONT] }), project({ id: 'p2', name: 'site', board: board({ action: 'pr' }) })] });
    app.shells = SHELLS;
    app.git.p1 = gitInfo();
    app.version = '1.3.1';
    app.modal = { kind: 'settings' };
    setLang('en');
  });

  it('names its tabs and what they set in English, and follows a change of language while it is open', async () => {
    fakeBackend();
    setLang('fr');
    render(SettingsModal);
    expect(screen.getByRole('dialog', { name: 'Réglages' })).toBeInTheDocument();
    expect(screen.getByText('Exécutable, modèle et permissions par défaut')).toBeInTheDocument();
    setLang('en');
    flushSync();
    const dialog = screen.getByRole('dialog', { name: 'Settings' });
    expect(
      within(dialog)
        .getAllByRole('tab')
        .map((x) => x.textContent?.trim()),
    ).toEqual([
      'AaApplication',
      '✳Claude Code',
      '◎Claude accounts',
      '♪Notifications',
      '▤Projects',
      '▦Kanban',
      '⧉Integrations',
      '$_Terminals',
      '⇄Network',
      'ⓘAbout',
    ]);
    expect(within(dialog).getByText('Executable, model, and default permissions')).toBeInTheDocument();
    expect(within(dialog).getByText('Escouade 1.3.1')).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Save' })).toBeInTheDocument();
    // The choices of a setting are read at each render: their words follow too.
    const effort = within(screen.getByRole('group', { name: 'Default effort' }));
    expect(effort.getAllByRole('button').map((b) => b.textContent)).toEqual(['Low', 'Medium', 'High', 'Very high', 'Max']);
    expect(effort.getByRole('button', { name: 'Low' })).toHaveAttribute('title', 'Quick answers, little thinking');
    expect(
      within(screen.getByRole('group', { name: 'Default permission mode' }))
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Auto', 'Ask', 'Plan', 'Accept edits', 'Bypass']);
    const path = screen.getByRole('textbox', { name: 'Executable path' });
    expect(path).toHaveAttribute('placeholder', 'claude (found in PATH)');
    app.claudePathFound = false;
    flushSync();
    expect(path).toHaveAttribute('placeholder', 'not found — enter the path to claude.exe');
    expect(screen.getByRole('switch', { name: 'Auto-resume after the usage limit' })).toBeInTheDocument();
    expect(screen.getByRole('spinbutton', { name: 'Stop idle Claude processes after (minutes)' })).toBeInTheDocument();
    await userEvent.type(screen.getByRole('spinbutton', { name: /Stop idle/ }), '0');
    expect(tab('Claude Code')).toHaveClass('changed');
    expect(tab('Claude Code')).toHaveAccessibleDescription('Modified, not saved yet');
  });

  it('says of the executable path whether it is found, not whether the current account’s Claude Code is', async () => {
    fakeBackend();
    // An account with a `claude` of its own is the current one: Claude Code is found, the settings' path is not.
    app.claudeFound = true;
    app.claudePathFound = false;
    render(SettingsModal, { tab: 'claude' });
    const path = screen.getByRole('textbox', { name: 'Executable path' });
    expect(path).toHaveAttribute('placeholder', 'not found — enter the path to claude.exe');
    app.claudePathFound = true;
    app.claudeFound = false;
    flushSync();
    expect(path).toHaveAttribute('placeholder', 'claude (found in PATH)');
  });

  it('says the switch to another account in English', () => {
    fakeBackend();
    app.settings.accounts = [
      { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal, { tab: 'claude' });
    expect(
      screen.getByRole('switch', { name: 'Resume a ticket’s agent stopped by the usage limit on another account' }),
    ).toBeInTheDocument();
    expect(screen.getByText('Otherwise it waits for its account to reset')).toBeInTheDocument();
  });

  it('says the notifications, the network and the terminals in English', async () => {
    fakeBackend();
    render(SettingsModal);
    await userEvent.click(tab('Notifications'));
    expect(
      within(panel())
        .getAllByRole('heading')
        .map((h) => h.textContent),
    ).toEqual(['Notify me about', 'Channels', 'Sound']);
    expect(
      group('Notify me about')
        .getAllByRole('switch')
        .map((s) => s.getAttribute('aria-label')),
    ).toEqual(['Questions and permissions', 'Finished tasks', 'Errors', 'Tickets (ready to review, blocked)']);
    expect(panel()).toHaveTextContent(
      /Off: no system notification or chime for this type; the tab, the card, and the (taskbar|Dock) still flag the agent\./,
    );
    expect(panel()).toHaveTextContent(/(Windows|macOS) notifications when the app is not in the foreground/);
    expect(screen.getByRole('button', { name: '▶ Test' })).toBeInTheDocument();

    await userEvent.click(tab('Terminals'));
    expect(panel()).toHaveTextContent('Detected: PowerShell 7, Git Bash');
    expect(screen.getByRole('textbox', { name: /WSL distribution/ })).toHaveAccessibleName('WSL distribution');

    await userEvent.click(tab('Network'));
    expect(screen.getByPlaceholderText('none')).toBeInTheDocument();
    expect(panel()).toHaveTextContent('NO_PROXY, separated by commas');
    expect(screen.getByRole('switch', { name: 'Skip TLS certificate verification' })).toHaveAttribute('aria-checked', 'false');

    await userEvent.click(tab('About'));
    expect(screen.getByRole('button', { name: 'Check for updates' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Install updates automatically' })).toBeInTheDocument();
    expect(screen.getByText('Local data')).toBeInTheDocument();
  });

  it('says a project’s settings in English, the worktree commands and the launch commands included', async () => {
    app.projects = [project({ runCommands: [FRONT], worktreeSetup: [{ id: 'w1', command: 'npm ci', shell: 'ghost', cwd: '' }] })];
    backendSaving({
      suggest_run_commands: () => ({ commands: [{ id: 's1', name: 'API', command: 'cargo run', shell: 'bash', cwd: '' }], refused: 2 }),
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(
      within(panel())
        .getAllByRole('heading', { level: 3 })
        .map((h) => h.textContent),
    ).toEqual(['Identity', 'Git', 'MCP server', 'Worktrees', 'Launch', 'Danger zone']);
    expect(screen.getByRole('textbox', { name: 'Project name' })).toHaveValue('demo-api');
    expect(screen.getByRole('button', { name: 'Color 4' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'One worktree per agent' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Files copied into worktrees' })).toHaveAccessibleDescription(/^Only files git ignores/);
    const commit = within(screen.getByRole('group', { name: 'Commit' }));
    expect(commit.getAllByRole('button').map((b) => b.textContent)).toEqual(['Written by the agent', 'Direct, with a suggested message']);

    // The steps of a worktree: their buttons name the command they move, and a shell this machine lacks is said so.
    const setup = within(screen.getByRole('group', { name: 'Setup command 1' }));
    expect(setup.getByRole('option', { name: 'ghost (not found)' })).toBeInTheDocument();
    expect(setup.getByRole('button', { name: 'Move command 1 down' })).toBeDisabled();
    expect(setup.getByRole('button', { name: 'Delete command 1' })).toBeInTheDocument();
    expect(setup.getByLabelText('Subfolder')).toHaveAttribute('placeholder', 'subfolder');
    await userEvent.click(screen.getByRole('button', { name: 'Add a teardown command' }));
    expect(screen.getByRole('group', { name: 'Teardown command 1' })).toBeInTheDocument();

    const launch = group('Launch');
    expect(launch.getByText('Command 1', { selector: 'legend' })).toBeInTheDocument();
    expect(launch.getByLabelText(/^Subfolder/)).toHaveValue('web');
    expect(launch.getByText('(blank = project folder)')).toBeInTheDocument();
    await userEvent.click(launch.getByRole('button', { name: '✦ Fill in automatically' }));
    // A plural, and what was left out said in the singular and the plural of the count.
    await expect
      .poll(() => app.toasts.at(-1)?.text)
      .toBe('1 command suggested, 2 left out (invisible characters or too long): review it before saving.');
    const proposal = within(await launch.findByRole('region', { name: 'Suggested launch commands' }));
    expect(proposal.getByText('the project folder')).toBeInTheDocument();
    expect(proposal.getByRole('button', { name: 'Ignore' })).toBeInTheDocument();
    expect(proposal.getByRole('button', { name: 'Replace the commands' })).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Close the project…' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Close “demo-api”?' });
  });

  it('says what keeps the settings from being saved in English', async () => {
    backendSaving();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.clear(screen.getByRole('textbox', { name: 'Project name' }));
    expect(screen.getByText('The project “demo-api” needs a name.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Project name' }), 'api');
    await userEvent.clear(within(screen.getByRole('group', { name: 'Command 1' })).getByLabelText('Command'));
    expect(screen.getByText('Every launch command in “api” needs a name and a command line.')).toBeInTheDocument();
  });

  it('says there is no project open in English', () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(SettingsModal, { tab: 'projects' });
    expect(screen.getByText('No project open.')).toBeInTheDocument();
  });

  it('tells the update check and the saved settings in English', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { tab: 'about' });
    await userEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('No update available.');
    await userEvent.click(screen.getByRole('switch', { name: 'Install updates automatically' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(app.toasts.at(-1)?.text).toBe('Settings saved');
  });
});
