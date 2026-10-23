import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { PROJECT_COLORS } from '../../lib/theme';
import type { ClaudeUpdateStatus, EngineInfo, RunCommand } from '../../lib/types';
import { BARE_ENGINE, board, branchInfo, CLAUDE_ENGINE, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
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
    branch_list: () => [
      branchInfo({ name: 'main', current: true }),
      branchInfo({ name: 'release' }),
      branchInfo({ name: 'escouade/agent-1' }),
      branchInfo({ name: 'ticket/DEM-4' }),
      branchInfo({ name: 'origin/main', remote: true, trackedBy: 'main' }),
    ],
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

  it('dit sous le chemin de l’exécutable où est son claude, dans WSL aussi', async () => {
    fakeBackend({
      claude_locate: ({ path }: { path: string }) =>
        path === ''
          ? { path: null, wsl: null, nat: false, canWsl: true }
          : { path: '/home/moi/.local/bin/claude', wsl: 'Ubuntu', nat: true, canWsl: true },
    });
    render(SettingsModal, { tab: 'claude' });
    expect(
      await screen.findByText(
        'Introuvable. Indique le chemin de claude.exe, ou de Claude Code installé dans WSL : ~/.local/bin/claude, /home/<utilisateur>/…, ou \\\\wsl.localhost\\<distribution>\\….',
      ),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: "Chemin de l'exécutable" }), '~/.local/bin/claude');
    expect(await screen.findByText('Trouvé dans WSL (Ubuntu) : /home/moi/.local/bin/claude')).toBeInTheDocument();
    expect(screen.getByText(/^Le réseau de WSL est en NAT : le serveur MCP d’Escouade/)).toBeInTheDocument();
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
      '✳Harness',
      '◎Comptes',
      '♪Notifications',
      '◖Voix',
      '▤Projets',
      '▦Kanban',
      '⧉Intégrations',
      '$_Terminaux',
      '⇄Réseau',
      'ⓘÀ propos',
    ]);
    expect(tab('Harness')).toHaveAttribute('aria-selected', 'true');
    expect(within(dialog).getByText('Exécutables, versions et serveur MCP')).toBeInTheDocument();
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
    expect(tab('Harness')).not.toHaveClass('changed');
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

  it('has no defaults of new agents for every harness any more: each account has its own (ESC-128)', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    expect(screen.queryByRole('group', { name: 'Modèle par défaut' })).not.toBeInTheDocument();
    expect(screen.queryByRole('group', { name: 'Effort par défaut' })).not.toBeInTheDocument();
    expect(screen.queryByRole('group', { name: 'Mode de permission par défaut' })).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Nouveaux agents' })).not.toBeInTheDocument();
    const resume = screen.getByRole('switch', { name: 'Reprise automatique après la limite d’usage' });
    expect(resume).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(resume);
    await userEvent.clear(screen.getByRole('spinbutton', { name: /Arrêter les processus des agents inactifs/ }));
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ autoResume: false, idleStopMinutes: 0 });
    expect(backend.called('save_settings')[0].args.settings).not.toHaveProperty('defaultModel');
  });

  it('opens new agents in Escouade or in their harness’s terminal (ESC-34)', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const view = group('Processus').getByRole('group', { name: 'Vue des nouveaux agents' });
    const escouade = within(view).getByRole('button', { name: 'Escouade' });
    const harness = within(view).getByRole('button', { name: 'Terminal du harness' });
    expect(escouade).toHaveAttribute('aria-pressed', 'true');
    expect(harness).toHaveAttribute('aria-pressed', 'false');
    // Its help says that a ticket's agent, and one whose harness has no such view, stay in Escouade's.
    expect(screen.getByText(/ceux d’un harness sans vue terminal, restent dans la vue d’Escouade/)).toBeInTheDocument();
    await userEvent.click(harness);
    expect(harness).toHaveAttribute('aria-pressed', 'true');
    expect(escouade).toHaveAttribute('aria-pressed', 'false');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ terminalView: true });
    expect(app.settings.terminalView).toBe(true);
  });

  it('lets the agents keep a task list, from the group of the processes, on by default', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const processes = group('Processus');
    const todo = processes.getByRole('switch', { name: 'Les agents tiennent une liste de tâches' });
    expect(todo).toHaveAttribute('aria-checked', 'true');
    // Next to the other settings of the processes, with its help.
    expect(processes.getByRole('switch', { name: 'Reprise automatique après la limite d’usage' })).toBeInTheDocument();
    // The help says what the setting does, and that the model is the one to use the tool: the banner is not promised.
    const help = processes.getByText(/Donne aux agents l’outil de liste de tâches de Claude Code/);
    expect(help).toHaveTextContent(/s’ils la tiennent, Escouade montre leur avancée dans le bandeau « Plan »/);
    expect(help).toHaveTextContent(/le modèle n’est pas obligé de s’en servir/);
    expect(help).not.toHaveTextContent(/Escouade peut alors montrer/);
    expect(tab('Harness')).not.toHaveClass('changed');
    await userEvent.click(todo);
    expect(todo).toHaveAttribute('aria-checked', 'false');
    expect(tab('Harness')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ todoTools: false });
    expect(app.settings.todoTools).toBe(false);
  });

  describe('the attachments of the Claude Code tab', () => {
    const maxField = () => screen.getByRole('spinbutton', { name: 'Taille maximale d’un fichier joint (Mo)' });
    const warnField = () => screen.getByRole('spinbutton', { name: 'Avertir pour un fichier joint de plus de (Mo)' });
    const keepField = () => screen.getByRole('spinbutton', { name: 'Purger les copies après (jours)' });

    it('shows the five settings with their defaults: no maximum, a warning at 200 Mo, the original file, a big text by its path', () => {
      fakeBackend();
      render(SettingsModal);
      const attachments = group('Pièces jointes');
      expect(maxField()).toHaveValue(0);
      expect(warnField()).toHaveValue(200);
      expect(keepField()).toHaveValue(0);
      expect(attachments.getByRole('switch', { name: 'Joindre le fichier d’origine' })).toHaveAttribute('aria-checked', 'true');
      const bigText = attachments.getByRole('group', { name: 'Texte de plus de 256 Ko' });
      expect(within(bigText).getByRole('button', { name: 'Joint par son chemin' })).toHaveAttribute('aria-pressed', 'true');
      expect(within(bigText).getByRole('button', { name: 'Refusé' })).toHaveAttribute('aria-pressed', 'false');
      // Each says what 0 means.
      expect(attachments.getByText('Mo, 0 = aucune limite')).toBeInTheDocument();
      expect(attachments.getByText('Mo, 0 = jamais')).toBeInTheDocument();
      expect(attachments.getByText('jours, 0 = tant que leur propriétaire existe')).toBeInTheDocument();
    });

    it('saves what is set', async () => {
      const backend = backendSaving();
      render(SettingsModal);
      const attachments = group('Pièces jointes');
      expect(tab('Harness')).not.toHaveClass('changed');
      await userEvent.clear(maxField());
      await userEvent.type(maxField(), '500');
      await userEvent.clear(warnField());
      await userEvent.type(keepField(), '30');
      await userEvent.click(attachments.getByRole('switch', { name: 'Joindre le fichier d’origine' }));
      await userEvent.click(attachments.getByRole('button', { name: 'Refusé' }));
      expect(tab('Harness')).toHaveClass('changed');
      await save();
      expect(backend.called('save_settings')[0].args.settings).toMatchObject({
        attachMaxMb: 500,
        attachWarnMb: 0,
        attachOrigin: false,
        attachBigText: 'refuse',
        attachKeepDays: 30,
      });
      expect(app.settings).toMatchObject({ attachMaxMb: 500, attachBigText: 'refuse', attachOrigin: false });
    });

    it('is written in English too', () => {
      setLang('en');
      fakeBackend();
      render(SettingsModal);
      const attachments = group('Attachments');
      expect(attachments.getByRole('spinbutton', { name: 'Largest attached file (MB)' })).toBeInTheDocument();
      expect(attachments.getByRole('spinbutton', { name: 'Warn for an attached file over (MB)' })).toBeInTheDocument();
      expect(attachments.getByRole('spinbutton', { name: 'Purge copies after (days)' })).toBeInTheDocument();
      expect(attachments.getByRole('switch', { name: 'Attach the original file' })).toBeInTheDocument();
      expect(attachments.getByRole('button', { name: 'Attached by its path' })).toBeInTheDocument();
      expect(attachments.getByText('MB, 0 = no limit')).toBeInTheDocument();
    });
  });

  it('puts the task list back, and saves nothing when it is switched on again', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const todo = screen.getByRole('switch', { name: 'Les agents tiennent une liste de tâches' });
    await userEvent.click(todo);
    await userEvent.click(todo);
    expect(todo).toHaveAttribute('aria-checked', 'true');
    expect(tab('Harness')).not.toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')).toHaveLength(0);
  });

  it('makes every agent reachable by remote control from its own group, beside the MCP switch, off until the user switches it on', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const remote = group('Remote control');
    const all = remote.getByRole('switch', { name: 'Remote control pour tous les agents' });
    expect(all).toHaveAttribute('aria-checked', 'false');
    expect(remote.getByText(/^Chaque agent de chaque projet, existant ou nouveau \(agents des tickets compris\)/)).toHaveTextContent(
      'Chaque agent de chaque projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ».',
    );
    // Right before the MCP server’s switch, not after its activity log, which can be long.
    const headings = within(panel())
      .getAllByRole('heading')
      .map((h) => h.textContent);
    expect(headings.slice(-3)).toEqual(['Remote control', 'Serveur MCP', 'Activité MCP']);
    expect(tab('Harness')).not.toHaveClass('changed');
    await userEvent.click(all);
    expect(all).toHaveAttribute('aria-checked', 'true');
    // A setting of the draft like the others: marked, saved by « Enregistrer ».
    expect(tab('Harness')).toHaveClass('changed');
    expect(backend.called('save_settings')).toHaveLength(0);
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ remoteControlAll: true });
    expect(app.settings.remoteControlAll).toBe(true);
  });

  it('puts the remote control of all agents back, and saves nothing when it is switched on again', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const all = screen.getByRole('switch', { name: 'Remote control pour tous les agents' });
    await userEvent.click(all);
    await userEvent.click(all);
    expect(tab('Harness')).not.toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')).toHaveLength(0);
  });

  it('lets a ticket’s agent stopped by the limit go on on another account, off until the user switches it on', async () => {
    const backend = backendSaving();
    app.settings.accounts = [
      { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal);
    const move = screen.getByRole('switch', { name: 'Reprendre sur un autre compte un agent de ticket arrêté par la limite' });
    // Off by default (not verified with a second real account), beside the automatic resume it is the alternative to.
    expect(move).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText('Sinon il attend la remise à zéro de son compte')).toBeInTheDocument();
    await userEvent.click(move);
    expect(tab('Harness')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ switchOnLimit: true });
  });

  it('has no such switch with a single account, nothing to go on on', () => {
    backendSaving();
    render(SettingsModal);
    expect(screen.getByRole('switch', { name: 'Reprise automatique après la limite d’usage' })).toBeInTheDocument();
    expect(screen.queryByRole('switch', { name: /Reprendre sur un autre compte/ })).not.toBeInTheDocument();
  });

  it('lets an agent go on on the next harness once every account of its own is spent, off by default, with a model per harness (ESC-88)', async () => {
    const backend = backendSaving();
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, name: 'Codex' }];
    app.enginesFound = ['claude', 'codex'];
    render(SettingsModal);
    const next = screen.getByRole('switch', { name: 'Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés' });
    expect(next).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(next);
    // Codex's best model by default; another one chosen.
    const codex = screen.getByRole('group', { name: 'Modèle de passation · Codex' });
    expect(within(codex).getByRole('button', { name: 'Le meilleur (GPT-5.5)' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(codex).getByRole('button', { name: 'GPT-5.5 mini' }));
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({
      handoverOnLimit: true,
      handoverModels: { codex: 'gpt-5.5-mini' },
    });
  });

  it('has no such switch with a single harness', () => {
    backendSaving();
    render(SettingsModal);
    expect(screen.queryByRole('switch', { name: /Continuer sur le harness suivant/ })).not.toBeInTheDocument();
  });

  it('changes nothing when cancelled, nor when closed', async () => {
    const backend = fakeBackend();
    render(SettingsModal);
    await userEvent.click(tab('Terminaux'));
    await userEvent.type(screen.getByRole('textbox', { name: /Git Bash/ }), 'C:\\Git\\bin\\bash.exe');
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    // Nothing saved; the « Harness » tab it opened on only read the harnesses and what the MCP server is doing.
    expect(
      backend.calls.filter((c) => !['engines', 'mcp_status', 'mcp_declare_status', 'mcp_activity', 'claude_update_status'].includes(c.cmd)),
    ).toEqual([]);
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
    await userEvent.click(screen.getByRole('button', { name: 'Branche cible : main. Choisir une autre branche' }));
    await userEvent.click(await within(await screen.findByRole('dialog', { name: 'Branche cible' })).findByText('release'));
    expect(screen.getByRole('button', { name: 'Branche cible : release. Choisir une autre branche' })).toBeInTheDocument();
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

  it('chooses the target branch in the branch picker, without the agents’ and the tickets’ branches (ESC-144)', async () => {
    backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Branche cible : main. Choisir une autre branche' }));
    const picker = await screen.findByRole('dialog', { name: 'Branche cible' });
    await within(picker).findByText('release');
    const names = within(picker)
      .getAllByRole('option')
      .map((o) => o.querySelector('.name')!.textContent);
    expect(names).toEqual(['main', 'release']);
    // The folder's branch can be the target; the one it is now is checked.
    const main = within(picker).getByText('main').closest('[role="option"]')!;
    expect(main).not.toHaveAttribute('aria-disabled');
    expect(main).toHaveAttribute('aria-current', 'true');
    expect(within(main as HTMLElement).getByText('dossier du projet')).toBeInTheDocument();
    // Searched as the branch button's picker is.
    await userEvent.type(within(picker).getByRole('combobox'), 'rel');
    expect(within(picker).getAllByRole('option')).toHaveLength(1);
  });

  it('lets the agents start from the target branch, off until turned on (ESC-144)', async () => {
    const backend = backendSaving();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    const on = screen.getByRole('switch', { name: 'Les agents partent de la branche cible' });
    expect(on).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText(/^Chaque nouvel agent à worktree part de main,/)).toBeInTheDocument();
    await userEvent.click(on);
    await save();
    expect(backend.called('board_set')[0].args.settings).toMatchObject({ agentsFromTarget: true });
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
    await userEvent.click(screen.getByRole('switch', { name: "Supprimer le worktree d'un ticket terminé ou supprimé" }));
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

  describe('the projects closed with their history', () => {
    const OLD = {
      id: 'c1',
      name: 'ancien',
      color: 'oklch(0.72 0.12 300)',
      path: 'C:\code\ancien',
      worktreePerAgent: false,
      agents: 3,
      tickets: 5,
      lastActivity: new Date(2026, 9, 14, 18, 0).getTime(),
      closedAt: new Date(2026, 9, 15, 9, 0).getTime(),
      size: 5 * 1024,
    };

    it('lists them under the settings of the project, in the Projets tab', async () => {
      const backend = fakeBackend({ closed_projects: () => [OLD] });
      render(SettingsModal, { tab: 'projects', projectId: 'p2' });
      await screen.findByRole('heading', { name: 'Projets fermés' });
      expect(group('Projets fermés').getByText('ancien')).toBeInTheDocument();
      expect(backend.called('closed_projects')).toHaveLength(1);
      const headings = within(panel())
        .getAllByRole('heading')
        .map((h) => h.textContent);
      expect(headings.slice(-3)).toEqual(['Zone sensible', 'Nouveaux projets', 'Projets fermés']);
      // Another tab of the project does not carry them.
      await userEvent.click(tab('Kanban'));
      expect(screen.queryByRole('heading', { name: 'Projets fermés' })).not.toBeInTheDocument();
    });

    it('lists them when no project is open, under what says so', async () => {
      resetApp({ projects: [] });
      fakeBackend({ closed_projects: () => [OLD] });
      render(SettingsModal, { tab: 'projects' });
      expect(screen.getByText('Aucun projet ouvert.')).toBeInTheDocument();
      await screen.findByRole('heading', { name: 'Projets fermés' });
      expect(group('Projets fermés').getByText('ancien')).toBeInTheDocument();
      expect(group('Projets fermés').getByRole('button', { name: "Effacer l'historique…" })).toBeInTheDocument();
      // The other tabs of a project still have nothing to show.
      await userEvent.click(tab('Kanban'));
      expect(screen.getByText('Aucun projet ouvert.')).toBeInTheDocument();
      expect(screen.queryByRole('heading', { name: 'Projets fermés' })).not.toBeInTheDocument();
    });

    it('shows no group when none was closed', async () => {
      const backend = fakeBackend({ closed_projects: () => [] });
      render(SettingsModal, { tab: 'projects', projectId: 'p2' });
      await waitFor(() => expect(backend.called('closed_projects')).toHaveLength(1));
      await new Promise((r) => setTimeout(r));
      expect(screen.queryByRole('heading', { name: 'Projets fermés' })).not.toBeInTheDocument();
    });
  });

  it('checks for an update on demand', async () => {
    const backend = fakeBackend({ update_check: () => ({ kind: 'none' }) });
    render(SettingsModal, { tab: 'about' });
    expect(screen.getByText('Escouade 1.3.1', { selector: '[role="tabpanel"] *' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Aucune mise à jour disponible.');
    // Asked by the user: the backend is told, so that a failure is never kept quiet.
    expect(backend.called('update_check').map((c) => c.args)).toEqual([{ manual: true }]);
  });

  it('tells why a check on demand failed, instead of saying there is nothing new', async () => {
    fakeBackend({
      update_check: () => ({ kind: 'failed', trouble: { message: '403 Forbidden de github.com', show: true, page: null } }),
    });
    render(SettingsModal, { tab: 'about' });
    const button = screen.getByRole('button', { name: 'Rechercher une mise à jour' });
    await userEvent.click(button);
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Vérification des mises à jour impossible : 403 Forbidden de github.com');
    // The check is over once its button is back.
    await expect.poll(() => (button as HTMLButtonElement).disabled).toBe(false);
    expect(app.toasts.map((t) => t.text)).toEqual(['Vérification des mises à jour impossible : 403 Forbidden de github.com']);
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

  it('shares the usage statistics only once switched on', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    app.settings = { ...app.settings, telemetry: null };
    render(SettingsModal, { tab: 'about' });
    const share = screen.getByRole('switch', { name: 'Statistiques d’usage' });
    expect(share).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(share);
    expect(tab('À propos')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ telemetry: true });
    expect(app.settings.telemetry).toBe(true);
  });

  it('stops the usage statistics when switched off', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    app.settings = { ...app.settings, telemetry: true };
    render(SettingsModal, { tab: 'about' });
    const share = screen.getByRole('switch', { name: 'Statistiques d’usage' });
    expect(share).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(share);
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ telemetry: false });
  });

  it('opens the website, in the language of the interface, and the Discord server', async () => {
    const backend = fakeBackend({ 'plugin:opener|open_url': () => null });
    render(SettingsModal, { tab: 'about' });
    expect(screen.getByText('Site web')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir le site' }));
    await userEvent.click(screen.getByRole('button', { name: 'Rejoindre le Discord' }));
    expect(backend.called('plugin:opener|open_url').map((c) => c.args.url)).toEqual([
      'https://escouade.dev/',
      'https://discord.gg/m3KYtN8gxP',
    ]);
    cleanup();
    setLang('en');
    render(SettingsModal, { tab: 'about' });
    await userEvent.click(screen.getByRole('button', { name: 'Open the website' }));
    expect(backend.called('plugin:opener|open_url').at(-1)?.args.url).toBe('https://escouade.dev/en/');
    setLang('fr');
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

  it('names the Kanban agents’ default after the model of the account its new agents go to (ESC-128)', async () => {
    fakeBackend({ git_branches: () => ['main'] });
    app.settings.accounts = app.settings.accounts.map((a) => ({ ...a, defaultModel: 'opus' }));
    render(SettingsModal);
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
    expect(screen.getByText('Exécutables, versions et serveur MCP')).toBeInTheDocument();
    setLang('en');
    flushSync();
    const dialog = screen.getByRole('dialog', { name: 'Settings' });
    expect(
      within(dialog)
        .getAllByRole('tab')
        .map((x) => x.textContent?.trim()),
    ).toEqual([
      'AaApplication',
      '✳Harness',
      '◎Accounts',
      '♪Notifications',
      '◖Voice',
      '▤Projects',
      '▦Kanban',
      '⧉Integrations',
      '$_Terminals',
      '⇄Network',
      'ⓘAbout',
    ]);
    expect(within(dialog).getByText('Executables, versions and the MCP server')).toBeInTheDocument();
    expect(within(dialog).getByText('Escouade 1.3.1')).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Save' })).toBeInTheDocument();
    const path = screen.getByRole('textbox', { name: 'Executable path' });
    expect(path).toHaveAttribute('placeholder', 'claude (found in PATH)');
    app.claudePathFound = false;
    flushSync();
    expect(path).toHaveAttribute('placeholder', 'not found — enter the path to claude.exe');
    expect(screen.getByRole('switch', { name: 'Auto-resume after the usage limit' })).toBeInTheDocument();
    expect(screen.getByRole('spinbutton', { name: 'Stop idle agent processes after (minutes)' })).toBeInTheDocument();
    await userEvent.type(screen.getByRole('spinbutton', { name: /Stop idle/ }), '0');
    expect(tab('Harness')).toHaveClass('changed');
    expect(tab('Harness')).toHaveAccessibleDescription('Modified, not saved yet');
  });

  it('names the task list setting in English', async () => {
    fakeBackend();
    render(SettingsModal);
    const todo = screen.getByRole('switch', { name: 'Agents keep a task list' });
    expect(todo).toHaveAttribute('aria-checked', 'true');
    const help = screen.getByText(/Gives agents Claude Code’s task list tool/);
    expect(help).toHaveTextContent(/if they keep a list, Escouade shows their progress in the “Plan” banner/);
    expect(help).toHaveTextContent(/the model does not have to use it/);
    expect(help).not.toHaveTextContent(/so Escouade can show/);
    expect(screen.getByRole('heading', { name: 'Processes' })).toBeInTheDocument();
  });

  it('names the remote control setting for all agents in English', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const remote = group('Remote control');
    const all = remote.getByRole('switch', { name: 'Remote control for all agents' });
    expect(all).toHaveAttribute('aria-checked', 'false');
    expect(remote.getByText(/^Every agent of every project/)).toHaveTextContent(
      'Every agent of every project, existing or new (ticket agents included), is reachable from claude.ai and the Claude app. Escouade starts each one’s claude process right away, and it stays running. The session appears on claude.ai as “project · agent” (for a ticket agent, the name comes from the ticket). In Claude Code view, an interface that is already open only picks it up at its next launch. To set an agent apart: right-click, “Turn off remote control”.',
    );
    await userEvent.click(all);
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ remoteControlAll: true });
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

  it('says under the executable path where its claude is: on Windows, in WSL and in which distribution, or how to name it', async () => {
    const { called } = fakeBackend({
      claude_locate: ({ path }: { path: string }) =>
        path === '~/.local/bin/claude'
          ? { path: '/home/me/.local/bin/claude', wsl: 'Ubuntu-24.04', nat: true, canWsl: true }
          : path === ''
            ? { path: 'C:\\Users\\me\\.local\\bin\\claude.exe', wsl: null, nat: false, canWsl: true }
            : { path: null, wsl: null, nat: false, canWsl: true },
    });
    render(SettingsModal, { tab: 'claude' });
    const field = screen.getByRole('textbox', { name: 'Executable path' });
    // Blank: the one found by itself.
    expect(await screen.findByText('Found: C:\\Users\\me\\.local\\bin\\claude.exe')).toBeInTheDocument();
    expect(field).toHaveAccessibleDescription('Found: C:\\Users\\me\\.local\\bin\\claude.exe');
    // The form of the issue: Claude Code installed in WSL, its network in NAT.
    await userEvent.type(field, '~/.local/bin/claude');
    expect(await screen.findByText('Found in WSL (Ubuntu-24.04): /home/me/.local/bin/claude')).toBeInTheDocument();
    expect(
      screen.getByText(
        'WSL’s network is NAT: Escouade’s MCP server and the tracking of the Claude Code view can’t reach the app. Switch it to mirrored (networkingMode=mirrored under [wsl2] in %UserProfile%\\.wslconfig), then run wsl --shutdown.',
      ),
    ).toBeInTheDocument();
    // Asked with the distribution of the settings.
    expect(called('claude_locate').at(-1)!.args).toEqual({ path: '~/.local/bin/claude', wslDistro: '' });
    await userEvent.clear(field);
    await userEvent.type(field, '/nowhere/claude');
    expect(
      await screen.findByText(
        'Not found. Give the path to claude.exe, or to Claude Code installed in WSL: ~/.local/bin/claude, /home/<user>/…, or \\\\wsl.localhost\\<distribution>\\….',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText(/WSL’s network is NAT/)).toBeNull();
  });

  it('names no WSL where there is none, and says nothing without an answer', async () => {
    fakeBackend({ claude_locate: () => ({ path: null, wsl: null, nat: false, canWsl: false }) });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('Not found. Give the path to claude.')).toBeInTheDocument();
    cleanup();
    fakeBackend();
    render(SettingsModal, { tab: 'claude' });
    await new Promise((r) => setTimeout(r, 400));
    expect(screen.queryByText(/^Found|^Not found/)).toBeNull();
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
    ).toEqual([
      '.escouade.yml',
      'Identity',
      'Remote control',
      'Privacy',
      'Git',
      'Worktrees',
      'Launch',
      'Setup assistant',
      'Danger zone',
      'New projects',
    ]);
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
    const backend = fakeBackend({ save_settings: () => [], update_check: () => ({ kind: 'none' }) });
    render(SettingsModal, { tab: 'about' });
    await userEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('No update available.');
    await userEvent.click(screen.getByRole('switch', { name: 'Install updates automatically' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(app.toasts.at(-1)?.text).toBe('Settings saved');
  });
});

describe('SettingsModal, the version of Claude Code', () => {
  beforeEach(() => resetApp());
  const located = { path: 'C:\Users\me\.local\bin\claude.exe', wsl: null, nat: false, canWsl: true };
  const info = (over: Partial<EngineInfo>): EngineInfo => ({
    ...CLAUDE_ENGINE,
    located,
    version: '2.1.301',
    state: 'ready',
    note: null,
    ...over,
  });

  it('shows the version found alone, or what keeps it from running', async () => {
    let answer = info({});
    fakeBackend({ claude_locate: () => located, engines: () => [answer] });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('2.1.301')).toHaveAttribute('data-state', 'ready');
    // No word of the tested versions under it (ESC-159).
    expect(screen.queryByText(/testé/i)).toBeNull();
    expect(screen.getByText('2.1.301')).not.toHaveAttribute('aria-describedby');
    cleanup();
    // Older than the tested ones: refused, its reason as the backend writes it.
    const refusal =
      'Claude Code 2.0.14 est trop ancien pour Escouade, qui demande au moins la version 2.1.283 : son flux JSON n’a été vérifié qu’à partir de celle-ci (docs/PROTOCOL.md). Mets-le à jour (claude update).';
    answer = info({ version: '2.0.14', state: 'tooOld', note: refusal });
    fakeBackend({ claude_locate: () => located, engines: () => [answer] });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText(refusal)).toBeInTheDocument();
    expect(screen.getByText('2.0.14')).toHaveClass('bad');
    expect(screen.getByText('2.0.14')).toHaveAccessibleDescription(refusal);
    cleanup();
    answer = info({ state: 'notConnected' });
    fakeBackend({ claude_locate: () => located, engines: () => [answer] });
    render(SettingsModal, { tab: 'claude' });
    expect(
      await screen.findByText('Aucun compte connecté : ajoute ou connecte un compte de ce harness dans l’onglet Comptes.'),
    ).toBeInTheDocument();
  });

  it('shows a version newer than the tested ones as ready, with no warning (ESC-159)', async () => {
    fakeBackend({ claude_locate: () => located, engines: () => [info({ version: '2.1.999' })] });
    render(SettingsModal, { tab: 'claude' });
    const version = await screen.findByText('2.1.999');
    expect(version).toHaveAttribute('data-state', 'ready');
    expect(version).not.toHaveClass('warn');
    expect(version).not.toHaveClass('bad');
    expect(version).not.toHaveAttribute('aria-describedby');
    expect(screen.queryByText(/testé|tested/i)).toBeNull();
  });

  it('shows no version while the path typed is not the saved one, nor for a claude not found', async () => {
    const backend = fakeBackend({ claude_locate: () => located, engines: () => [info({})] });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('2.1.301')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: "Chemin de l'exécutable" }), 'C:\autre\claude.exe');
    await waitFor(() => expect(screen.queryByText('2.1.301')).toBeNull());
    expect(backend.called('engines').length).toBeGreaterThan(0);
  });

  it('says it in English too', async () => {
    setLang('en');
    fakeBackend({ claude_locate: () => located, engines: () => [info({ state: 'notConnected' })] });
    render(SettingsModal, { tab: 'claude' });
    expect(
      await screen.findByText('No account signed in: add or sign in an account of this harness in the Accounts tab.'),
    ).toBeInTheDocument();
    cleanup();
    fakeBackend({ claude_locate: () => located, engines: () => [info({ version: null })] });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('Unknown version: claude didn’t tell it.')).toBeInTheDocument();
  });

  it('hides the settings of what no harness does', () => {
    const labels = ['Les agents tiennent une liste de tâches', 'Remote control pour tous les agents'];
    const view = 'Vue des nouveaux agents';
    fakeBackend({ claude_locate: () => located });
    render(SettingsModal, { tab: 'claude' });
    for (const name of labels) expect(screen.getByRole('switch', { name })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: view })).toBeInTheDocument();
    cleanup();
    app.engines = [BARE_ENGINE];
    fakeBackend({ claude_locate: () => located });
    render(SettingsModal, { tab: 'claude' });
    for (const name of labels) expect(screen.queryByRole('switch', { name })).toBeNull();
    expect(screen.queryByRole('group', { name: view })).toBeNull();
  });
});

describe('SettingsModal, Claude Code updated by Escouade (ESC-133)', () => {
  beforeEach(() => resetApp());
  const located = { path: 'C:\\Users\\me\\.local\\bin\\claude.exe', wsl: null, nat: false, canWsl: true };
  const info = (over: Partial<EngineInfo> = {}): EngineInfo => ({
    ...CLAUDE_ENGINE,
    located,
    version: '2.1.302',
    state: 'ready',
    note: null,
    ...over,
  });
  const status = (over: Partial<ClaudeUpdateStatus>): ClaudeUpdateStatus => ({
    state: 'waiting',
    version: null,
    from: null,
    line: null,
    at: null,
    ...over,
  });
  const SWITCH = 'Mettre à jour Claude Code automatiquement';

  it('says what the last update did, its error in a line, or that the user turned it off in Claude Code', async () => {
    let answer = status({ state: 'updated', version: '2.1.302', from: '2.1.301', at: 1 });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { tab: 'claude' });
    expect(screen.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'true');
    expect(await screen.findByText('Mis à jour en 2.1.302 (depuis la 2.1.301).')).toBeInTheDocument();
    cleanup();
    answer = status({ state: 'upToDate', version: '2.1.302', at: 1 });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('À jour (2.1.302).')).toBeInTheDocument();
    cleanup();
    answer = status({ state: 'failed', line: 'Error: EACCES: permission denied', at: 1 });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('Échec de la mise à jour : Error: EACCES: permission denied')).toHaveClass('bad');
    cleanup();
    answer = status({ state: 'userOff', line: 'DISABLE_AUTOUPDATER' });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { tab: 'claude' });
    expect(
      await screen.findByText('Coupée dans Claude Code lui-même (DISABLE_AUTOUPDATER) : Escouade ne le met pas à jour non plus.'),
    ).toBeInTheDocument();
  });

  it('is turned off by its switch, saved with the settings, and says that nothing updates Claude Code then', async () => {
    const backend = fakeBackend({
      claude_locate: () => located,
      engines: () => [info()],
      claude_update_status: () => status({ state: 'upToDate', version: '2.1.302' }),
      save_settings: () => [],
    });
    render(SettingsModal, { tab: 'claude' });
    expect(await screen.findByText('À jour (2.1.302).')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: SWITCH }));
    expect(
      screen.getByText(
        'Coupée : Claude Code n’est plus mis à jour, ni par Escouade ni par lui-même. Lance claude update de temps en temps.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(backend.called('save_settings').at(-1)!.args.settings.claudeSelfUpdate).toBe(false);
  });

  it('says that Claude Code is gone, or its installation cut, with the repair', async () => {
    const cut =
      'L’installation de Claude Code a été interrompue (il en reste /opt/homebrew/bin/.claude-Xa3k9) : relance `npm install -g @anthropic-ai/claude-code` dans un terminal.';
    fakeBackend({
      claude_locate: () => ({ path: null, wsl: null, nat: false, canWsl: true }),
      engines: () => [info({ located: { ...located, path: null }, version: null, state: 'absent', note: cut })],
      claude_update_status: () => status({ state: 'missing', line: cut }),
    });
    render(SettingsModal, { tab: 'claude' });
    await waitFor(() => expect(screen.getAllByText(cut).length).toBe(2));
  });
});

describe('SettingsModal, the « Harness » tab (ESC-29)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });
  const located = { path: 'C:\Users\me\.local\bin\claude.exe', wsl: null, nat: false, canWsl: true };
  const claude = (over: Partial<EngineInfo> = {}): EngineInfo => ({
    ...CLAUDE_ENGINE,
    located,
    version: '2.1.301',
    state: 'notConnected',
    note: null,
    ...over,
  });
  const bare = (over: Partial<EngineInfo> = {}): EngineInfo => ({
    ...BARE_ENGINE,
    located: { path: 'C:\bin\codex.exe', wsl: null, nat: false, canWsl: true },
    version: '0.163.0',
    state: 'ready',
    note: null,
    ...over,
  });

  it('lists each harness with its executable and its version, and leaves signing in to the Accounts tab', async () => {
    fakeBackend({ claude_locate: () => located, engines: () => [claude(), bare()] });
    render(SettingsModal, { tab: 'claude' });
    const own = await screen.findByRole('region', { name: 'Claude Code' });
    expect(within(own).getByRole('textbox', { name: "Chemin de l'exécutable" })).toBeInTheDocument();
    expect(await within(own).findByText('2.1.301')).toHaveAttribute('data-state', 'notConnected');
    expect(
      within(own).getByText(/Aucun compte connecté : ajoute ou connecte un compte de ce harness dans l’onglet Comptes\.$/),
    ).toBeInTheDocument();
    const other = screen.getByRole('region', { name: 'Harness nu' });
    expect(within(other).getByText('Trouvé : C:\bin\codex.exe')).toBeInTheDocument();
    expect(within(other).getByText('0.163.0')).toHaveAttribute('data-state', 'ready');
    // An account signs in from Réglages › Comptes (ESC-128): the tab has no state line nor « Se connecter… » any more.
    expect(screen.queryByRole('button', { name: 'Se connecter…' })).toBeNull();
    expect(screen.queryByText('État')).toBeNull();
  });

  it('gives the command that installs a harness that is not found', async () => {
    fakeBackend({
      claude_locate: () => located,
      engines: () => [
        claude({ state: 'ready' }),
        bare({ located: { path: null, wsl: null, nat: false, canWsl: true }, version: null, state: 'absent' }),
      ],
    });
    render(SettingsModal, { tab: 'claude' });
    const other = await screen.findByRole('region', { name: 'Harness nu' });
    expect(await within(other).findByText('Introuvable')).toHaveAttribute('data-engine-state', 'absent');
    expect(within(other).getByText('npm install -g @openai/codex')).toBeInTheDocument();
    expect(await within(screen.getByRole('region', { name: 'Claude Code' })).findByText('2.1.301')).toHaveAttribute('data-state', 'ready');
  });

  it('says it in English too', async () => {
    setLang('en');
    fakeBackend({ claude_locate: () => located, engines: () => [claude(), bare()] });
    render(SettingsModal, { tab: 'claude' });
    const own = await screen.findByRole('region', { name: 'Claude Code' });
    expect(
      await within(own).findByText(/No account signed in: add or sign in an account of this harness in the Accounts tab\.$/),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Sign in…' })).toBeNull();
    expect(within(screen.getByRole('region', { name: 'Harness nu' })).getByText('Found: C:\bin\codex.exe')).toBeInTheDocument();
  });
});
