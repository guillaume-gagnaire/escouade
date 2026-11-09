import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { SAVE_PAUSE_MS, settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { BRAND, PROJECT_COLORS } from '../../lib/theme';
import type { ClaudeUpdateStatus, EngineInfo, RunCommand } from '../../lib/types';
import { BARE_ENGINE, board, branchInfo, CLAUDE_ENGINE, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import SettingsModal from './SettingsModal.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const SHELLS = [
  { id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' },
  { id: 'bash', label: 'Git Bash', path: 'bash.exe' },
];

/** The bar of the pages. */
const nav = () => screen.getByRole('navigation');
/** An entry of the bar: a page, or a project. */
const tab = (name: string | RegExp) => within(nav()).getByRole('button', { name });
const panel = () => document.getElementById('settings-panel')!;
/** What is in a group of the settings, under its heading. */
const group = (name: string) => within(within(panel()).getByRole('heading', { name }).closest('section')!);
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

  it('opens on the page asked for, or on the page a 2.0 tab leads to, the project unfolded on its pages', () => {
    fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'tickets', projectId: 'p2' });
    expect(tab('Tickets importés')).toHaveAttribute('aria-current', 'page');
    expect(tab('site')).toHaveAttribute('aria-expanded', 'true');
    expect(tab('demo-api')).toHaveAttribute('aria-expanded', 'false');
    expect(settingsForm.page).toBe('tickets');
    expect(settingsForm.projectId).toBe('p2');
    cleanup();
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    expect(tab('Lancement')).toHaveAttribute('aria-current', 'page');
    expect(settingsForm.page).toBe('launch');
    cleanup();
    render(SettingsModal, { page: 'advanced' });
    expect(tab('Avancé')).toHaveAttribute('aria-current', 'page');
    expect(settingsForm.page).toBe('advanced');
    // On a page of Escouade, no project is unfolded.
    expect(within(nav()).queryByRole('button', { name: 'Projet' })).toBeNull();
  });

  it('dit sous le chemin de l’exécutable où est son claude, dans WSL aussi', async () => {
    fakeBackend({
      claude_locate: ({ path }: { path: string }) =>
        path === ''
          ? { path: null, wsl: null, nat: false, canWsl: true }
          : { path: '/home/moi/.local/bin/claude', wsl: 'Ubuntu', nat: true, canWsl: true },
    });
    render(SettingsModal, { page: 'accounts' });
    expect(
      await screen.findByText(
        'Introuvable. Indique le chemin de claude.exe, ou de Claude Code installé dans WSL : ~/.local/bin/claude, /home/<utilisateur>/…, ou \\\\wsl.localhost\\<distribution>\\….',
      ),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: "Chemin de l'exécutable" }), '~/.local/bin/claude');
    expect(await screen.findByText('Trouvé dans WSL (Ubuntu) : /home/moi/.local/bin/claude')).toBeInTheDocument();
    expect(screen.getByText(/^Le réseau de WSL est en NAT : le serveur MCP d’Escouade/)).toBeInTheDocument();
  });

  it('shows a page at a time, from the bar: Escouade, the projects, Tous les projets, À propos and the version', async () => {
    fakeBackend();
    render(SettingsModal);
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    const entries = () =>
      within(nav())
        .getAllByRole('button')
        .map((t) => t.textContent?.trim());
    expect(entries()).toEqual([
      'Général',
      'Agents',
      'Comptes et harness',
      'Services connectés',
      'Voix',
      'Notifications',
      'Plugins',
      'Avancé',
      'demo-api',
      'site',
      'Tous les projets',
      'À propos1.3.1',
    ]);
    // An icon each for Escouade's pages, drawn; a square of its colour for each project.
    expect(
      within(nav())
        .getAllByRole('button')
        .map((t) => t.querySelector('svg')?.dataset.icon ?? (t.querySelector('.sq') ? 'sq' : null)),
    ).toEqual(['sliders', 'agents', 'user', 'link', 'mic', 'bell', 'puzzle', 'term', 'sq', 'sq', 'folder', 'info']);
    expect([...nav().querySelectorAll('.s-h')].map((g) => g.textContent)).toEqual(['Escouade', 'Projets']);
    // The search (R8).
    expect(within(nav()).getByRole('searchbox', { name: 'Rechercher un réglage' })).toBeEnabled();
    expect(tab('Comptes et harness')).toHaveAttribute('aria-current', 'page');
    expect(within(dialog).getByRole('heading', { level: 1, name: 'Comptes et harness' })).toBeInTheDocument();
    expect(within(dialog).getByText('Les harness de ce poste, leurs comptes, et ce qui se passe à la limite')).toBeInTheDocument();
    expect(within(panel()).getByRole('region', { name: 'Claude Code' })).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Quand un quota est épuisé' })).toBeInTheDocument();

    await userEvent.click(tab('Avancé'));
    expect(tab('Avancé')).toHaveAttribute('aria-current', 'page');
    expect(tab('Comptes et harness')).not.toHaveAttribute('aria-current');
    expect(within(panel()).getByRole('heading', { name: 'Réseau' })).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Claude Code' })).not.toBeInTheDocument();
    // Each page starts at its top, whatever the one before was scrolled to.
    panel().scrollTop = 300;
    await userEvent.click(tab('Général'));
    expect(panel().scrollTop).toBe(0);
    // A project opens on « Projet », unfolded on its pages; another page of it keeps the project.
    await userEvent.click(tab('site'));
    expect(settingsForm.page).toBe('project');
    expect(settingsForm.projectId).toBe('p2');
    expect(entries().slice(8, 19)).toEqual([
      'demo-api',
      'site',
      'Projet',
      'Git et worktrees',
      'Lancement',
      'Kanban',
      'Tickets importés',
      'Équipe',
      'Confidentialité',
      'Tous les projets',
      'À propos1.3.1',
    ]);
    expect(within(dialog).getByRole('heading', { level: 1, name: 'site' })).toBeInTheDocument();
    await userEvent.click(tab('Kanban'));
    expect(within(dialog).getByRole('heading', { level: 1, name: 'Kanban' })).toBeInTheDocument();
    // Another project, on the same page.
    await userEvent.click(tab('demo-api'));
    expect(settingsForm.page).toBe('board');
    expect(settingsForm.projectId).toBe('p1');
    expect(tab('site')).toHaveAttribute('aria-expanded', 'false');
  });

  it('goes through the bar with the arrows, Home and End, unfolding a project gone through', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'plugins' });
    tab('Plugins').focus();
    await userEvent.keyboard('{ArrowDown}');
    expect(tab('Avancé')).toHaveAttribute('aria-current', 'page');
    expect(tab('Avancé')).toHaveFocus();
    // On a project: it opens, on « Projet », the focus on its name.
    await userEvent.keyboard('{ArrowDown}');
    expect(settingsForm.projectId).toBe('p1');
    expect(tab('Projet')).toHaveAttribute('aria-current', 'page');
    await expect.poll(() => document.activeElement).toBe(tab('demo-api'));
    await userEvent.keyboard('{ArrowDown}');
    await userEvent.keyboard('{ArrowDown}');
    expect(tab('Git et worktrees')).toHaveAttribute('aria-current', 'page');
    await expect.poll(() => document.activeElement).toBe(tab('Git et worktrees'));
    await userEvent.keyboard('{End}');
    expect(tab(/^À propos/)).toHaveAttribute('aria-current', 'page');
    await expect.poll(() => document.activeElement).toBe(tab(/^À propos/));
    // Up through « Tous les projets » to the last project, opened on « Projet ».
    await userEvent.keyboard('{ArrowUp}');
    await userEvent.keyboard('{ArrowUp}');
    await expect.poll(() => document.activeElement).toBe(tab('site'));
    expect(settingsForm.projectId).toBe('p2');
    expect(settingsForm.page).toBe('project');
    await userEvent.keyboard('{Home}');
    expect(tab('Général')).toHaveAttribute('aria-current', 'page');
    await expect.poll(() => document.activeElement).toBe(tab('Général'));
    // The first stays where it is.
    await userEvent.keyboard('{ArrowUp}');
    expect(tab('Général')).toHaveFocus();
    // One entry in the Tab order: the one the keyboard is on.
    expect(
      within(nav())
        .getAllByRole('button')
        .filter((b) => b.tabIndex === 0),
    ).toEqual([tab('Général')]);
  });

  it('tints the window by what the page sets: Escouade’s teal, or the project’s colour as in the draft', async () => {
    fakeBackend();
    app.projects[1].color = PROJECT_COLORS[3];
    const tint = () => document.documentElement.style.getPropertyValue('--p');
    const { unmount } = render(SettingsModal, { page: 'general' });
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    expect(tint()).toBe(BRAND);
    expect(dialog).toHaveAttribute('data-scope', 'app');
    await userEvent.click(tab('site'));
    expect(tint()).toBe(PROJECT_COLORS[3]);
    expect(dialog).toHaveAttribute('data-scope', 'project');
    // Its tile beside its name.
    expect(dialog.querySelector('.s-head .tile')).toHaveTextContent('Si');
    // A colour chosen in the draft tints it at once.
    settingsForm.projects.p2.color = PROJECT_COLORS[5];
    flushSync();
    expect(tint()).toBe(PROJECT_COLORS[5]);
    await userEvent.click(tab('Tous les projets'));
    expect(tint()).toBe(BRAND);
    expect(dialog.querySelector('.s-head .tile')).toBeNull();
    // Closed, the app's own tint comes back: the colour of the project on screen.
    await userEvent.click(tab('site'));
    app.ui.view = 'project';
    app.ui.activeProject = 'p1';
    unmount();
    expect(tint()).toBe(app.projects[0].color);
  });

  it('saves a text when it loses the focus and a switch at once, each once, with no « Enregistrer » (ESC-380)', async () => {
    const backend = fakeBackend({ save_settings: () => [{ id: 'pwsh', label: 'PowerShell', path: 'pwsh.exe' }] });
    render(SettingsModal);
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    expect(within(dialog).queryByRole('button', { name: 'Enregistrer' })).toBeNull();
    expect(within(dialog).queryByRole('button', { name: 'Annuler' })).toBeNull();
    await userEvent.click(tab('Avancé'));
    await userEvent.type(screen.getByPlaceholderText('aucun'), 'http://proxy:3128');
    // Typing: not yet.
    expect(backend.called('save_settings')).toHaveLength(0);
    // Left for another page: saved.
    await userEvent.click(tab('Notifications'));
    await settingsForm.settled();
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ proxyUrl: 'http://proxy:3128', sound: true });
    const sound = screen.getByRole('switch', { name: 'Son activé' });
    expect(sound).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(sound);
    await settingsForm.settled();
    expect(backend.called('save_settings')).toHaveLength(2);
    expect(backend.called('save_settings')[1].args.settings).toMatchObject({ proxyUrl: 'http://proxy:3128', sound: false });
    await afterGesture();
    expect(backend.called('save_settings')).toHaveLength(2);
    // The window stays open, nothing to confirm.
    expect(app.modal).not.toBeNull();
    expect(app.shells.map((s) => s.id)).toEqual(['pwsh']);
  });

  it('saves a text after a pause in the typing, without leaving the field (ESC-380)', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal);
    await userEvent.click(tab('Avancé'));
    const proxy = screen.getByPlaceholderText('aucun');
    await userEvent.type(proxy, 'http://proxy:3128');
    expect(proxy).toHaveFocus();
    expect(backend.called('save_settings')).toHaveLength(0);
    await waitFor(() => expect(backend.called('save_settings')).toHaveLength(1), { timeout: SAVE_PAUSE_MS + 1000 });
    expect(backend.called('save_settings')[0].args.settings.proxyUrl).toBe('http://proxy:3128');
    expect(proxy).toHaveFocus();
  });

  it('saves what waits when the window closes, and asks nothing (ESC-380)', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    app.modal = { kind: 'settings' };
    const view = render(SettingsModal);
    await userEvent.click(tab('Avancé'));
    await userEvent.type(screen.getByPlaceholderText('aucun'), 'http://proxy:3128');
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    expect(backend.called('save_settings')).toHaveLength(0);
    // What App does once the modal is gone.
    view.unmount();
    await waitFor(() => expect(backend.called('save_settings')).toHaveLength(1));
    expect(app.settings.proxyUrl).toBe('http://proxy:3128');
  });

  it('turns TLS verification off for a proxy with a certificate of its own', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal);
    await userEvent.click(tab('Avancé'));
    const insecure = screen.getByRole('switch', { name: 'Ignorer la vérification des certificats TLS' });
    expect(insecure).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(insecure);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ insecureTls: true });
    expect(app.settings.insecureTls).toBe(true);
  });

  it('chooses what to be told about, everything on by default', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    await userEvent.click(tab('Notifications'));
    const group = within(panel()).getByRole('heading', { name: 'Me prévenir pour' }).closest('section')!;
    const labels = ['Questions et autorisations', 'Tâches terminées', 'Erreurs', 'Tickets (prêt à tester, bloqué)'];
    // Ticked, as the preview has them.
    expect(
      within(group)
        .getAllByRole('checkbox')
        .map((s) => s.closest('label')!.textContent!.trim()),
    ).toEqual(labels);
    for (const label of labels) expect(within(group).getByRole('checkbox', { name: label })).toBeChecked();
    // Off, a type is silent outside the window; what blinks or flashes still does.
    expect(group).toHaveTextContent(
      /Désactivé : ni notification système ni carillon pour ce type ; l’onglet, la carte et (la barre des tâches|le Dock) signalent toujours l’agent\./,
    );

    await userEvent.click(within(group).getByRole('checkbox', { name: 'Tâches terminées' }));
    expect(within(group).getByRole('checkbox', { name: 'Tâches terminées' })).not.toBeChecked();
    expect(within(group).getByRole('checkbox', { name: 'Erreurs' })).toBeChecked();
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings.notifyFor).toEqual({
      ...app.settings.notifyFor,
      questions: true,
      done: false,
      errors: true,
      tickets: true,
    });
    expect(app.settings.notifyFor.done).toBe(false);
  });

  it('sets each news of the code hosts on its own (ESC-68)', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    await userEvent.click(tab('Notifications'));
    const hosts = within(panel()).getByRole('heading', { name: 'Pull requests et déploiements' }).closest('section')!;
    const names = within(hosts)
      .getAllByRole('checkbox')
      .map((c) => c.closest('label')?.textContent?.trim());
    expect(names).toEqual([
      'CI en échec',
      'Changements demandés',
      'PR approuvée',
      'PR mergée',
      'Conflit',
      'Relecture demandée',
      'Déploiement en production réussi',
      'Déploiement en échec',
    ]);
    await userEvent.click(within(hosts).getByRole('checkbox', { name: 'PR approuvée' }));
    await userEvent.click(within(hosts).getByRole('checkbox', { name: 'Déploiement en production réussi' }));
    await afterGesture();
    const saved = backend.called('save_settings').at(-1)!.args.settings.notifyFor;
    expect(saved.prApproved).toBe(false);
    expect(saved.deployOk).toBe(false);
    expect(saved.prCiFailed).toBe(true);
    expect(saved.deployFailed).toBe(true);
  });

  it('has no defaults of new agents for every harness any more: each account has its own (ESC-128)', async () => {
    const backend = backendSaving();
    render(SettingsModal);
    const resume = group('Quand un quota est épuisé').getByRole('switch', { name: 'Reprise automatique après la limite d’usage' });
    expect(resume).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(resume);
    expect(screen.queryByRole('group', { name: 'Modèle par défaut' })).not.toBeInTheDocument();
    expect(screen.queryByRole('group', { name: 'Effort par défaut' })).not.toBeInTheDocument();
    expect(screen.queryByRole('group', { name: 'Mode de permission par défaut' })).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Nouveaux agents' })).not.toBeInTheDocument();
    await userEvent.click(tab('Agents'));
    await userEvent.click(within(panel()).getByRole('button', { name: /^Avancé/ }));
    const idle = screen.getByRole('spinbutton', { name: /Arrêter les processus des agents inactifs/ });
    await userEvent.clear(idle);
    await afterGesture();
    // Emptied: said on its row, the number saved before stays (ESC-380).
    expect(idle).toHaveAttribute('aria-invalid', 'true');
    expect(idle).toHaveAccessibleDescription(`Un nombre entier, 0 ou plus : ${app.settings.idleStopMinutes} reste enregistré.`);
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({
      autoResume: false,
      idleStopMinutes: app.settings.idleStopMinutes,
    });
    await userEvent.type(idle, '0');
    await afterGesture();
    expect(idle).toHaveAttribute('aria-invalid', 'false');
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ idleStopMinutes: 0 });
    expect(backend.called('save_settings').at(-1)!.args.settings).not.toHaveProperty('defaultModel');
  });

  it('opens new agents in Escouade or in their harness’s terminal (ESC-34)', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const view = group('Nouveaux agents').getByRole('group', { name: 'Vue des nouveaux agents' });
    const escouade = within(view).getByRole('button', { name: 'Escouade' });
    const harness = within(view).getByRole('button', { name: 'Terminal du harness' });
    expect(escouade).toHaveAttribute('aria-pressed', 'true');
    expect(harness).toHaveAttribute('aria-pressed', 'false');
    // Its help says that a ticket's agent, and one whose harness has no such view, stay in Escouade's.
    expect(screen.getByText(/ceux d’un harness sans vue terminal, restent dans la vue d’Escouade/)).toBeInTheDocument();
    await userEvent.click(harness);
    expect(harness).toHaveAttribute('aria-pressed', 'true');
    expect(escouade).toHaveAttribute('aria-pressed', 'false');
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ terminalView: true });
    expect(app.settings.terminalView).toBe(true);
  });

  it('saves the delay before a message sent during a turn', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const delay = group('Nouveaux agents').getByRole('group', { name: 'Délai avant l’envoi pendant un tour' });
    expect(screen.getByText(/tu peux le modifier ou l’annuler d’ici là/)).toHaveTextContent(/Cursor et OpenCode/);
    expect(
      within(delay)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Désactivé', '5 s', '10 s', '20 s', '30 s']);
    await userEvent.click(within(delay).getByRole('button', { name: '20 s' }));
    expect(within(delay).getByRole('button', { name: '20 s' })).toHaveAttribute('aria-pressed', 'true');
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ midTurnDelaySecs: 20 });
    expect(app.settings.midTurnDelaySecs).toBe(20);
  });

  it('lets the agents keep a task list, from « Nouveaux agents », on by default', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const processes = group('Nouveaux agents');
    const todo = processes.getByRole('switch', { name: 'Les agents tiennent une liste de tâches' });
    expect(todo).toHaveAttribute('aria-checked', 'true');
    // Next to the other settings of the processes, with its help.
    expect(processes.getByRole('group', { name: 'Vue des nouveaux agents' })).toBeInTheDocument();
    // The help says what the setting does, and that the model is the one to use the tool: the banner is not promised.
    const help = processes.getByText(/Donne aux agents l’outil de liste de tâches de Claude Code/);
    expect(help).toHaveTextContent(/s’ils la tiennent, Escouade montre leur avancée dans le bandeau « Plan »/);
    expect(help).toHaveTextContent(/le modèle n’est pas obligé de s’en servir/);
    expect(help).not.toHaveTextContent(/Escouade peut alors montrer/);
    await userEvent.click(todo);
    expect(todo).toHaveAttribute('aria-checked', 'false');
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ todoTools: false });
    expect(app.settings.todoTools).toBe(false);
  });

  describe('the attachments, in « Avancé »', () => {
    const maxField = () => screen.getByRole('spinbutton', { name: 'Taille maximale d’un fichier joint (Mo)' });
    const warnField = () => screen.getByRole('spinbutton', { name: 'Avertir pour un fichier joint de plus de (Mo)' });
    const keepField = () => screen.getByRole('spinbutton', { name: 'Purger les copies après (jours)' });

    it('shows the five settings with their defaults: no maximum, a warning at 200 Mo, the original file, a big text by its path', () => {
      fakeBackend();
      render(SettingsModal, { page: 'advanced' });
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
      render(SettingsModal, { page: 'advanced' });
      const attachments = group('Pièces jointes');
      await userEvent.clear(maxField());
      await userEvent.type(maxField(), '500');
      await userEvent.clear(warnField());
      await userEvent.type(warnField(), '0');
      await userEvent.type(keepField(), '30');
      await userEvent.click(attachments.getByRole('switch', { name: 'Joindre le fichier d’origine' }));
      await userEvent.click(attachments.getByRole('button', { name: 'Refusé' }));
      await afterGesture();
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({
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
      render(SettingsModal, { page: 'advanced' });
      const attachments = group('Attachments');
      expect(attachments.getByRole('spinbutton', { name: 'Largest attached file (MB)' })).toBeInTheDocument();
      expect(attachments.getByRole('spinbutton', { name: 'Warn for an attached file over (MB)' })).toBeInTheDocument();
      expect(attachments.getByRole('spinbutton', { name: 'Purge copies after (days)' })).toBeInTheDocument();
      expect(attachments.getByRole('switch', { name: 'Attach the original file' })).toBeInTheDocument();
      expect(attachments.getByRole('button', { name: 'Attached by its path' })).toBeInTheDocument();
      expect(attachments.getByText('MB, 0 = no limit')).toBeInTheDocument();
    });
  });

  it('saves the task list at each switch, back on when switched on again', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const todo = screen.getByRole('switch', { name: 'Les agents tiennent une liste de tâches' });
    await userEvent.click(todo);
    await userEvent.click(todo);
    expect(todo).toHaveAttribute('aria-checked', 'true');
    await afterGesture();
    expect(backend.called('save_settings').map((c) => c.args.settings.todoTools)).toEqual([false, true]);
    expect(app.settings.todoTools).toBe(true);
  });

  it('makes every agent reachable by remote control from Agents, before « Piloter Escouade », off until the user switches it on', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const remote = group('Remote control');
    const all = remote.getByRole('switch', { name: 'Remote control pour tous les agents' });
    expect(remote.getByText('Claude Code seulement : les autres harness n’ont pas de remote control.')).toBeInTheDocument();
    expect(all).toHaveAttribute('aria-checked', 'false');
    expect(remote.getByText(/^Chaque agent de chaque projet, existant ou nouveau \(agents des tickets compris\)/)).toHaveTextContent(
      'Chaque agent de chaque projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ».',
    );
    // On the page of « Piloter Escouade », its group after.
    expect(within(panel()).getByRole('heading', { name: 'Piloter Escouade' })).toBeInTheDocument();
    await userEvent.click(all);
    expect(all).toHaveAttribute('aria-checked', 'true');
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ remoteControlAll: true });
    expect(app.settings.remoteControlAll).toBe(true);
  });

  it('saves the remote control of all agents at each switch, off again when switched off', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const all = screen.getByRole('switch', { name: 'Remote control pour tous les agents' });
    await userEvent.click(all);
    await userEvent.click(all);
    await afterGesture();
    expect(backend.called('save_settings').map((c) => c.args.settings.remoteControlAll)).toEqual([true, false]);
    expect(app.settings.remoteControlAll).toBe(false);
  });

  it('lets a ticket’s agent stopped by the limit go on on another account, off until the user switches it on', async () => {
    const backend = backendSaving();
    app.settings.accounts = [
      { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal, { page: 'accounts' });
    const move = group('Quand un quota est épuisé').getByRole('switch', {
      name: 'Reprendre sur un autre compte un agent de ticket arrêté par la limite',
    });
    // Off by default (not verified with a second real account), beside the automatic resume it is the alternative to.
    expect(move).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText('Sinon il attend la remise à zéro de son compte')).toBeInTheDocument();
    await userEvent.click(move);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ switchOnLimit: true });
  });

  it('has no such switch with a single account, nothing to go on on', () => {
    backendSaving();
    render(SettingsModal);
    expect(screen.getByRole('switch', { name: 'Reprise automatique après la limite d’usage' })).toBeInTheDocument();
    cleanup();
    render(SettingsModal, { page: 'accounts' });
    expect(screen.queryByRole('switch', { name: /Reprendre sur un autre compte/ })).not.toBeInTheDocument();
  });

  it('lets an agent go on on the next harness once every account of its own is spent, off by default, with a model per harness (ESC-88)', async () => {
    const backend = backendSaving();
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, name: 'Codex' }];
    app.enginesFound = ['claude', 'codex'];
    render(SettingsModal, { page: 'accounts' });
    const next = screen.getByRole('switch', { name: 'Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés' });
    expect(next).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(next);
    // Codex's best model by default, in « Avancé »; another one chosen.
    await userEvent.click(within(panel()).getByRole('button', { name: /^Avancé/ }));
    const codex = screen.getByRole('combobox', { name: 'Modèle de passation · Codex' }) as HTMLSelectElement;
    expect(codex.selectedOptions[0].text).toBe('Le meilleur (GPT-5.5)');
    await userEvent.selectOptions(codex, 'gpt-5.5-mini');
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({
      handoverOnLimit: true,
      handoverModels: { codex: 'gpt-5.5-mini' },
    });
  });

  it('has no such switch with a single harness', () => {
    backendSaving();
    render(SettingsModal, { page: 'accounts' });
    expect(screen.queryByRole('switch', { name: /Continuer sur le harness suivant/ })).not.toBeInTheDocument();
  });

  it('saves nothing when nothing is changed, opened, gone through and closed', async () => {
    const backend = fakeBackend();
    const view = render(SettingsModal);
    await userEvent.click(tab('Avancé'));
    await userEvent.click(tab('Notifications'));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
    view.unmount();
    await settingsForm.settled();
    // The pages it showed only read the harnesses, what the MCP server is doing and the vectors' model.
    expect(
      backend.calls.filter(
        (c) =>
          ![
            'engines',
            'mcp_status',
            'mcp_declare_status',
            'mcp_activity',
            'claude_update_status',
            'vectors_model',
            'account_status',
            'closed_projects',
            // What the seed holds, for a harness not installed (R4).
            'plugins_list',
            'plugin_seed_offers',
            'data_dir',
            // The amber dots of the bar (R8): the code hosts' accounts as last checked, each plugin's process.
            'forges',
            'plugin_run',
          ].includes(c.cmd),
      ),
    ).toEqual([]);
  });

  it('opens on the page and the project asked for, and switches project', async () => {
    backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p2' });
    expect(tab('Kanban')).toHaveAttribute('aria-current', 'page');
    await expect.poll(() => document.activeElement).toBe(tab('Kanban'));
    expect(tab('site')).toHaveAttribute('aria-expanded', 'true');
    expect(within(nav()).getByRole('group', { name: 'site' })).toContainElement(tab('Kanban'));
    expect(screen.getByRole('radio', { name: /Ouvrir une pull request/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(tab('demo-api'));
    expect(tab('Kanban')).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toHaveAttribute('aria-checked', 'true');
    // No project unfolded where the settings are the app's.
    await userEvent.click(tab('Avancé'));
    expect(within(nav()).queryByRole('group')).not.toBeInTheDocument();
  });

  it('sets what validating a ticket does, saved with the rest', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Branche cible : main. Choisir une autre branche' }));
    await userEvent.click(await within(await screen.findByRole('dialog', { name: 'Branche cible' })).findByText('release'));
    expect(screen.getByRole('button', { name: 'Branche cible : release. Choisir une autre branche' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    expect(screen.queryByRole('switch', { name: 'PR en brouillon' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('radio', { name: /Ouvrir une pull request/ }));
    // The merge strategy goes; « Mise à jour de la branche » (ESC-61) has a Rebase of its own.
    expect(screen.queryByRole('group', { name: 'Stratégie' })).not.toBeInTheDocument();
    expect(within(screen.getByRole('group', { name: 'Mise à jour de la branche' })).getByRole('button', { name: 'Merge' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    await userEvent.click(screen.getByRole('switch', { name: 'PR en brouillon' }));
    await afterGesture();
    expect(backend.called('board_set').at(-1)!.args).toMatchObject({
      projectId: 'p1',
      settings: { action: 'pr', draft: true, target: 'release', strategy: 'rebase' },
    });
    expect(app.projects[0].board).toMatchObject({ action: 'pr', target: 'release' });
  });

  it('chooses the target branch in the branch picker, without the agents’ and the tickets’ branches (ESC-144)', async () => {
    backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
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

  it('no longer says where the agents start: the project’s settings do (ESC-160)', async () => {
    backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    expect(screen.queryByRole('switch', { name: 'Les agents partent de la branche cible' })).toBeNull();
    expect(screen.queryByText(/^Chaque nouvel agent à worktree part de/)).toBeNull();
  });

  it('hides the target branch when the work is only pushed or kept', async () => {
    backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    await userEvent.click(screen.getByRole('radio', { name: /Laisser en l'état/ }));
    expect(screen.queryByText('Branche cible')).not.toBeInTheDocument();
  });

  it('keeps the tests switch off until a command is given, and previews the commit message', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    const tests = screen.getByRole('switch', { name: 'Relancer les tests avant' });
    expect(tests).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Commande de tests' }), 'npm test');
    expect(tests).toBeEnabled();
    await userEvent.click(tests);
    expect(screen.getByText('feat: limiter les tentatives de connexion [DEM-42]')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Message de commit généré' }));
    expect(screen.getByText('DEM-42 Limiter les tentatives de connexion')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: "Supprimer le worktree d'un ticket terminé ou supprimé" }));
    await afterGesture();
    expect(backend.called('board_set').at(-1)!.args.settings).toMatchObject({
      testCommand: 'npm test',
      testsFirst: true,
      conventional: false,
      cleanup: false,
    });
  });

  it('sets the conflict policy, the autopilot, the agents in parallel and their model', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    await userEvent.click(within(screen.getByRole('group', { name: 'En cas de conflit' })).getByRole('button', { name: "L'agent résout" }));
    await userEvent.click(screen.getByRole('switch', { name: 'Attribuer les tickets automatiquement' }));
    await userEvent.click(within(screen.getByRole('group', { name: 'En parallèle' })).getByRole('button', { name: '4' }));
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Modèle' }), 'opus');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Mode' }), 'plan');
    await afterGesture();
    expect(backend.called('board_set').at(-1)!.args.settings).toMatchObject({
      conflict: 'agent',
      autopilot: false,
      maxParallel: 4,
      model: 'opus',
      mode: 'plan',
    });
  });

  it('pauses the autopilot past a quota, for every project, beside « Changer de compte à » and no more in Kanban (R4)', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    expect(screen.queryByRole('group', { name: /au-delà du quota/ })).toBeNull();
    await userEvent.click(tab('Comptes et harness'));
    const limit = group('Quand un quota est épuisé');
    // The two thresholds one after the other, then the resumes.
    expect(limit.getAllByRole('group').map((g) => g.getAttribute('aria-label'))).toEqual([
      'Changer de compte à',
      'Pause du pilote auto au-delà du quota',
    ]);
    expect(limit.getAllByRole('switch').map((g) => g.getAttribute('aria-label'))).toEqual(['Reprise automatique après la limite d’usage']);
    const pause = limit.getByRole('group', { name: 'Pause du pilote auto au-delà du quota' });
    expect(
      within(pause)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['80 %', '90 %', '95 %', '100 %']);
    expect(within(pause).getByRole('button', { name: '100 %' })).toHaveAttribute('aria-pressed', 'true');
    expect(
      screen.getByText(
        'Le pilote auto ne démarre aucun ticket tant que la fenêtre de 5 h ou la fenêtre hebdomadaire dépasse ce seuil. Un ticket lancé à la main part quand même.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText('pour tous les projets')).toBeInTheDocument();
    await userEvent.click(within(pause).getByRole('button', { name: '90 %' }));
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ quotaPause: 90 });
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
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    await afterGesture();
    expect(app.toasts.at(-1)?.text).toContain('disque plein');
    expect(app.modal).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Rebase' })).toHaveAttribute('aria-pressed', 'true');
    expect(app.projects[0].board.strategy).toBe('squash');
  });

  it('sets a project’s name, color and worktrees', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'project', projectId: 'p2' });
    expect(screen.getByText('C:\\code\\demo-api')).toBeInTheDocument();
    const name = screen.getByRole('textbox', { name: 'Nom du projet' });
    await userEvent.clear(name);
    await userEvent.type(name, 'vitrine');
    const colors = screen.getByRole('group', { name: 'Couleur' });
    expect(within(colors).getAllByRole('button')).toHaveLength(PROJECT_COLORS.length);
    await userEvent.click(within(colors).getByRole('button', { name: 'Couleur 4' }));
    expect(within(colors).getByRole('button', { name: 'Couleur 4' })).toHaveAttribute('aria-pressed', 'true');
    // The worktrees are on « Git et worktrees », the same draft.
    await userEvent.click(tab('Git et worktrees'));
    await userEvent.click(screen.getByRole('switch', { name: 'Un worktree par agent' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({
      id: 'p2',
      name: 'vitrine',
      color: PROJECT_COLORS[3],
      worktreePerAgent: true,
    });
    expect(app.projects[1].name).toBe('vitrine');
  });

  it('edits, adds and saves the project’s launch commands', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    const front = screen.getByRole('group', { name: 'Commande 1' });
    expect(within(front).getByLabelText('Nom')).toHaveValue('Front');
    expect(within(front).getByLabelText('Shell')).toHaveValue('pwsh');
    expect(within(front).getByLabelText(/Sous-dossier/)).toHaveValue('web');

    await userEvent.click(screen.getByRole('button', { name: 'Ajouter une commande' }));
    const api = screen.getByRole('group', { name: 'Commande 2' });
    // A new command runs with the first detected shell, in the project's folder.
    expect(within(api).getByLabelText('Shell')).toHaveValue('pwsh');
    await userEvent.type(within(api).getByLabelText('Nom'), 'API');
    await userEvent.type(within(api).getByLabelText('Commande'), 'cargo run');
    await userEvent.selectOptions(within(api).getByLabelText('Shell'), 'bash');
    await afterGesture();

    const saved = backend.called('update_project').at(-1)!.args.project.runCommands;
    expect(saved).toEqual([FRONT, { id: expect.any(String), name: 'API', command: 'cargo run', shell: 'bash', cwd: '' }]);
    expect(app.projects[0].runCommands).toEqual(saved);
  });

  it('says on its row a command without a name or a line, and a project without a name, and saves neither (ESC-380)', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    const command = within(screen.getByRole('group', { name: 'Commande 1' })).getByLabelText('Commande');
    await userEvent.clear(command);
    await afterGesture();
    expect(command).toHaveAttribute('aria-invalid', 'true');
    expect(command).toHaveAccessibleDescription(
      'Chaque commande demande un nom et une ligne de commande : les commandes enregistrées restent celles d’avant tant qu’il en manque.',
    );
    expect(backend.called('update_project')).toHaveLength(0);
    expect(app.projects[0].runCommands).toEqual([FRONT]);
    await userEvent.type(command, 'npm start');
    await afterGesture();
    expect(command).toHaveAttribute('aria-invalid', 'false');
    expect(backend.called('update_project')).toHaveLength(1);
    expect(app.projects[0].runCommands[0].command).toBe('npm start');
    await userEvent.click(tab('Projet'));
    const name = screen.getByRole('textbox', { name: 'Nom du projet' });
    await userEvent.clear(name);
    await afterGesture();
    expect(name).toHaveAccessibleDescription('Un projet garde un nom : « demo-api » reste enregistré.');
    expect(backend.called('update_project')).toHaveLength(1);
    expect(app.projects[0].name).toBe('demo-api');
  });

  it('removes a command, stopping it if it ran', async () => {
    const backend = backendSaving();
    app.launches.c1 = { status: 'running', ptyId: 't1', name: 'Front', stopping: false, code: null, startedAt: 1 };
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    await userEvent.click(within(screen.getByRole('group', { name: 'Commande 1' })).getByRole('button', { name: 'Supprimer' }));
    expect(screen.getByText("Aucune commande pour l'instant.")).toBeInTheDocument();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.runCommands).toEqual([]);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    expect(app.launches.c1).toBeUndefined();
  });

  it('chooses who writes the commits, the agent by default', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Commit' });
    expect(within(choice).getByRole('button', { name: "Rédigé par l'agent" })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('Direct : Escouade propose un message, tu le relis et tu commites toi-même.')).toBeInTheDocument();
    await userEvent.click(within(choice).getByRole('button', { name: 'Direct, avec un message proposé' }));
    expect(within(choice).getByRole('button', { name: 'Direct, avec un message proposé' })).toHaveAttribute('aria-pressed', 'true');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', commitMode: 'direct' });
    expect(app.projects[0].commitMode).toBe('direct');
  });

  it('chooses where the launches’ pages open, in the agent by default (ESC-311)', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Ouvrir les pages des lancements' });
    expect(within(choice).getByRole('button', { name: 'Dans l’agent' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(choice).getByRole('button', { name: 'Dans une fenêtre à part' })).toHaveAttribute('aria-pressed', 'false');
    await userEvent.click(within(choice).getByRole('button', { name: 'Dans le navigateur du système' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', pagesOpen: 'system' });
    expect(app.projects[0].pagesOpen).toBe('system');
  });

  it('offers no preferred account with a single account', () => {
    backendSaving();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    expect(screen.queryByRole('group', { name: 'Compte préféré' })).not.toBeInTheDocument();
  });

  it('chooses the account a project’s agents and tickets go to, « Automatique » by default', async () => {
    const backend = backendSaving();
    app.settings.accounts = [
      ...app.settings.accounts,
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
      { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false },
    ];
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Compte préféré' });
    // The account switched off is not offered.
    expect(
      within(choice)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Automatique', 'Principal', 'Pro']);
    expect(within(choice).getByRole('button', { name: 'Automatique' })).toHaveAttribute('aria-pressed', 'true');
    expect(choice.closest('.sr')).toHaveTextContent('Le compte sur lequel partent les nouveaux agents et les tickets de ce projet.');
    await userEvent.click(within(choice).getByRole('button', { name: 'Pro' }));
    expect(within(choice).getByRole('button', { name: 'Pro' })).toHaveAttribute('aria-pressed', 'true');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', account: 'pro' });
    expect(app.projects[0].account).toBe('pro');
  });

  it('goes back to « Automatique », and shows an account switched off since that the project still prefers', async () => {
    const backend = backendSaving();
    app.projects[0].account = 'team';
    app.settings.accounts = [
      ...app.settings.accounts,
      { id: 'team', name: 'Équipe', configDir: 'C:\\claude\\team', claudePath: '', active: false },
    ];
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Compte préféré' });
    expect(within(choice).getByRole('button', { name: 'Équipe (inactif)' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(choice).getByRole('button', { name: 'Automatique' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', account: '' });
  });

  it('names the preferred account in English', async () => {
    setLang('en');
    backendSaving();
    app.settings.accounts = [
      ...app.settings.accounts,
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const choice = screen.getByRole('group', { name: 'Preferred account' });
    expect(
      within(choice)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Automatic', 'Main', 'Pro']);
    expect(choice.closest('.sr')).toHaveTextContent('The account this project’s new agents and tickets go to.');
  });

  it('saves the files copied into new worktrees', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const field = screen.getByRole('textbox', { name: /Fichiers copiés dans les worktrees/ });
    expect(field).toHaveValue('.env*');
    // Only what git ignores is copied: a copy never shows as a change to commit.
    expect(field).toHaveAccessibleDescription(/^Seuls ceux que git ignore/);
    await userEvent.type(field, '{Enter}**/.env.local');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.worktreeCopy).toEqual(['.env*', '**/.env.local']);
  });

  it('sets the commands run in new worktrees, by hand or as Claude suggests them', async () => {
    let answer!: (v: unknown) => void;
    const backend = backendSaving({ suggest_worktree_steps: () => new Promise((r) => (answer = r)) });
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter une commande de démontage' }));
    const down = screen.getByRole('group', { name: 'Commande de démontage 1' });
    expect(within(down).getByLabelText('Shell')).toHaveValue('pwsh');
    await userEvent.type(within(down).getByLabelText('Commande'), 'docker compose down');
    const worktrees = group('Worktrees');
    await userEvent.click(worktrees.getByRole('button', { name: 'Remplir automatiquement' }));
    expect(worktrees.getByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
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
        "Proposition de Claude : relis chaque commande en entier. Elles remplacent les deux listes et, une fois prises, tournent seules : la préparation à l'ouverture de chaque nouveau worktree, le démontage avant sa suppression.",
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
    expect(worktrees.getByRole('button', { name: 'Remplir automatiquement' })).toBeEnabled();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({
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
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    await userEvent.click(group('Worktrees').getByRole('button', { name: 'Remplir automatiquement' }));
    const proposal = await group('Worktrees').findByRole('region', { name: 'Commandes de worktree proposées' });
    // The reading's button was given up: the proposal takes the focus, to be read from the keyboard.
    expect(proposal).toHaveFocus();
    // Still there when coming back to the project, which keeps the focus.
    await userEvent.click(tab('Avancé'));
    await userEvent.click(tab('demo-api'));
    await userEvent.click(tab('Git et worktrees'));
    expect(tab('Git et worktrees')).toHaveFocus();
    const worktrees = group('Worktrees');
    const back = within(worktrees.getByRole('region', { name: 'Commandes de worktree proposées' }));
    expect(back.getByText('npm ci')).toBeInTheDocument();
    await userEvent.click(back.getByRole('button', { name: 'Ignorer' }));
    expect(worktrees.getByRole('button', { name: 'Remplir automatiquement' })).toHaveFocus();
    expect(worktrees.queryByRole('region', { name: 'Commandes de worktree proposées' })).toBeNull();
    expect(screen.queryByRole('group', { name: 'Commande de préparation 1' })).toBeNull();
    await afterGesture();
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('reorders the commands run in new worktrees with their buttons, or Alt+↑ and Alt+↓ on one of them', async () => {
    const steps = ['npm ci', 'npm run gen', 'npm run db'].map((command, i) => ({ id: `s${i}`, command, shell: 'pwsh', cwd: '' }));
    app.projects = [project({ worktreeSetup: steps })];
    const backend = backendSaving();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
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
    await afterGesture();
    expect(
      backend
        .called('update_project')
        .at(-1)!
        .args.project.worktreeSetup.map((s: { id: string }) => s.id),
    ).toEqual(['s2', 's0', 's1']);
  });

  it('sets the launch commands as Claude suggests them, in place of those of the draft', async () => {
    let answer!: (v: unknown) => void;
    const backend = backendSaving({ suggest_run_commands: () => new Promise((r) => (answer = r)) });
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    const launch = group('Lancement');
    expect(
      launch.getByText(
        'Claude lit le projet (manifestes, scripts, docker-compose, README…) sans rien modifier et propose les commandes à lancer. Relis-les avant de remplacer les tiennes.',
      ),
    ).toBeInTheDocument();
    // Not before it is asked for.
    expect(backend.called('suggest_run_commands')).toHaveLength(0);
    await userEvent.click(launch.getByRole('button', { name: 'Remplir automatiquement' }));
    expect(launch.getByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
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
      .toBe('2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant de remplacer les commandes.');
    // Read in full first, in place of nothing yet.
    const proposal = within(await launch.findByRole('region', { name: 'Commandes de lancement proposées' }));
    expect(proposal.getByText('API')).toBeInTheDocument();
    expect(proposal.getByText('cargo watch -x run')).toBeInTheDocument();
    expect(proposal.getByText('le dossier du projet')).toBeInTheDocument();
    expect(proposal.getByText('npm run dev')).toBeInTheDocument();
    expect(screen.getByDisplayValue('Front')).toBeInTheDocument();
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
    expect(launch.getByRole('button', { name: 'Remplir automatiquement' })).toBeEnabled();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.runCommands).toEqual([
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
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    await userEvent.click(group('Lancement').getByRole('button', { name: 'Remplir automatiquement' }));
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
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    // The proposal is styled by its own component (settings/Proposal): no rule of the page around it (a launch
    // command's box, a step of a worktree) reaches what it lists, and none of its rules reaches the page.
    const apart = async (groupName: string, name: string, ofPage: string) => {
      await userEvent.click(group(groupName).getByRole('button', { name: 'Remplir automatiquement' }));
      const proposal = await screen.findByRole('region', { name });
      const scopeOf = (e: Element) => [...e.classList].find((c) => c.startsWith('svelte-'));
      const page = scopeOf(panel().querySelector(ofPage)!)!;
      const own = scopeOf(proposal)!;
      expect(page).not.toBe(own);
      expect(proposal.querySelectorAll(`.${page}`)).toHaveLength(0);
      expect([...panel().querySelectorAll(`.${own}`)].every((e) => proposal.contains(e))).toBe(true);
    };
    await apart('Worktrees', 'Commandes de worktree proposées', '.sub-h');
    await userEvent.click(tab('Lancement'));
    await apart('Lancement', 'Commandes de lancement proposées', '.cmds');
  });

  it('tells the two « ✦ Remplir automatiquement » buttons apart by the text of their own row', async () => {
    backendSaving();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const button = { name: 'Remplir automatiquement' };
    expect(group('Worktrees').getByRole('button', button)).toHaveAccessibleDescription(/lockfiles.*propose les commandes\. Relis-les/);
    cleanup();
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    expect(group('Lancement').getByRole('button', button)).toHaveAccessibleDescription(/docker-compose.*commandes à lancer\. Relis-les/);
  });

  it('keeps the launch commands when Claude proposes none readable, and says so', async () => {
    backendSaving({ suggest_run_commands: () => Promise.reject("Claude n'a pas proposé de commandes lisibles.") });
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    await userEvent.click(group('Lancement').getByRole('button', { name: 'Remplir automatiquement' }));
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ text: "Claude n'a pas proposé de commandes lisibles.", kind: 'error' });
    expect(screen.getByDisplayValue('Front')).toBeInTheDocument();
    expect(group('Lancement').getByRole('button', { name: 'Remplir automatiquement' })).toBeEnabled();
  });

  it('reads the project at once when it is opened to propose the launch commands, but not when it comes back', async () => {
    let answer!: (v: unknown) => void;
    const backend = backendSaving({ suggest_run_commands: () => new Promise((r) => (answer = r)) });
    const { unmount } = render(SettingsModal, { page: 'launch', projectId: 'p1', suggest: true });
    expect(backend.called('suggest_run_commands')[0].args).toEqual({ projectId: 'p1' });
    expect(group('Lancement').getByRole('button', { name: 'Claude lit le projet…' })).toBeDisabled();
    answer({ commands: [{ id: 's1', name: 'Web', command: 'npm start', shell: 'bash', cwd: '' }], refused: 0 });
    const proposal = await group('Lancement').findByRole('region', { name: 'Commandes de lancement proposées' });
    await userEvent.click(within(proposal).getByRole('button', { name: 'Remplacer les commandes' }));
    expect(await screen.findByDisplayValue('Web')).toBeInTheDocument();
    unmount();
    // Back from a modal it opened (the project's closing): the draft as it was, nothing asked again.
    render(SettingsModal, { page: 'launch', projectId: 'p1', suggest: true, resume: true });
    expect(backend.called('suggest_run_commands')).toHaveLength(1);
    expect(screen.getByDisplayValue('Web')).toBeInTheDocument();
  });

  it('asks before closing the project', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'project', projectId: 'p2' });
    // In « Avancé », folded at the foot of « Projet », which says what it hides (no « Zone sensible » any more).
    expect(screen.queryByRole('button', { name: 'Fermer le projet…' })).toBeNull();
    const fold = within(panel()).getByRole('button', { name: /^Avancé/ });
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(fold).toHaveTextContent('AvancéFermer le projet');
    await userEvent.click(fold);
    expect(fold).toHaveAttribute('aria-expanded', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Fermer « site » ?', confirm: 'Fermer le projet' });
  });

  it('says there is no project to set when none is open', async () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(SettingsModal, { page: 'project' });
    expect(screen.getByText('Aucun projet ouvert.')).toBeInTheDocument();
    // No project in the bar, and still an entry to reach it from the keyboard.
    expect(nav().querySelectorAll('.s-h')).toHaveLength(1);
    await expect.poll(() => document.activeElement).toBe(tab('Général'));
    // « Services connectés » is Escouade's: it needs no project.
    await userEvent.click(tab('Services connectés'));
    expect(screen.queryByText('Aucun projet ouvert.')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Comptes connectés' })).toBeInTheDocument();
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

    it('lists them in « Tous les projets », after what a new project starts with', async () => {
      const backend = fakeBackend({ closed_projects: () => [OLD] });
      render(SettingsModal, { page: 'allProjects', projectId: 'p2' });
      await screen.findByRole('heading', { name: 'Projets fermés' });
      expect(group('Projets fermés').getByText('ancien')).toBeInTheDocument();
      expect(backend.called('closed_projects')).toHaveLength(1);
      const headings = within(panel())
        .getAllByRole('heading')
        .map((h) => h.textContent);
      expect(headings.slice(-2)).toEqual(['Nouveaux projets', 'Projets fermés']);
      // A page of a project does not carry them.
      await userEvent.click(tab('site'));
      expect(screen.queryByRole('heading', { name: 'Projets fermés' })).not.toBeInTheDocument();
    });

    it('lists them when no project is open', async () => {
      resetApp({ projects: [] });
      fakeBackend({ closed_projects: () => [OLD] });
      render(SettingsModal, { page: 'project' });
      expect(screen.getByText('Aucun projet ouvert.')).toBeInTheDocument();
      await userEvent.click(tab('Tous les projets'));
      await screen.findByRole('heading', { name: 'Projets fermés' });
      expect(group('Projets fermés').getByText('ancien')).toBeInTheDocument();
      expect(group('Projets fermés').getByRole('button', { name: "Effacer l'historique…" })).toBeInTheDocument();
    });

    it('shows no group when none was closed', async () => {
      const backend = fakeBackend({ closed_projects: () => [] });
      render(SettingsModal, { page: 'allProjects' });
      await waitFor(() => expect(backend.called('closed_projects')).toHaveLength(1));
      await new Promise((r) => setTimeout(r));
      expect(screen.queryByRole('heading', { name: 'Projets fermés' })).not.toBeInTheDocument();
    });
  });

  it('checks for an update on demand', async () => {
    const backend = fakeBackend({ update_check: () => ({ kind: 'none' }) });
    render(SettingsModal, { page: 'general' });
    expect(screen.getByText('Escouade 1.3.1', { selector: '#settings-panel *' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Aucune mise à jour disponible.');
    // Asked by the user: the backend is told, so that a failure is never kept quiet.
    expect(backend.called('update_check').map((c) => c.args)).toEqual([{ manual: true }]);
  });

  it('tells why a check on demand failed, instead of saying there is nothing new', async () => {
    fakeBackend({
      update_check: () => ({ kind: 'failed', trouble: { message: '403 Forbidden de github.com', show: true, page: null } }),
    });
    render(SettingsModal, { page: 'general' });
    const button = screen.getByRole('button', { name: 'Rechercher une mise à jour' });
    await userEvent.click(button);
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Vérification des mises à jour impossible : 403 Forbidden de github.com');
    // The check is over once its button is back.
    await expect.poll(() => (button as HTMLButtonElement).disabled).toBe(false);
    expect(app.toasts.map((t) => t.text)).toEqual(['Vérification des mises à jour impossible : 403 Forbidden de github.com']);
  });

  it('installs updates by itself unless told not to', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'general' });
    const auto = screen.getByRole('switch', { name: 'Installer les mises à jour automatiquement' });
    expect(auto).toHaveAttribute('aria-checked', 'true');
    expect(
      screen.getByText('Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré.'),
    ).toBeInTheDocument();
    await userEvent.click(auto);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ autoUpdate: false });
    expect(app.settings.autoUpdate).toBe(false);
  });

  it('receives the betas only once ticked, saved at once, a check following (ESC-374)', async () => {
    const backend = fakeBackend({ save_settings: () => [], update_check: () => ({ kind: 'none' }) });
    render(SettingsModal, { page: 'general' });
    const beta = screen.getByRole('switch', { name: 'Recevoir les versions bêta' });
    expect(beta).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText(/Les bêtas arrivent avant tout le monde et peuvent avoir des défauts/)).toBeInTheDocument();
    await userEvent.click(beta);
    // Saved without « Enregistrer »: the check that follows reads the saved settings.
    await waitFor(() => expect(backend.called('update_check')).toHaveLength(1));
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ betaUpdates: true });
    expect(backend.called('update_check')[0].args).toEqual({ manual: true });
    expect(app.settings.betaUpdates).toBe(true);
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Aucune mise à jour disponible.');
    // Unticked: saved, and a quiet check takes back a beta on offer.
    await userEvent.click(beta);
    await waitFor(() => expect(backend.called('update_check')).toHaveLength(2));
    expect(backend.called('save_settings')[1].args.settings).toMatchObject({ betaUpdates: false });
    expect(backend.called('update_check')[1].args).toEqual({ manual: false });
    expect(beta).toHaveAttribute('aria-checked', 'false');
  });

  it('says « bêta » beside the version of an app on a beta, and of a beta found', async () => {
    app.version = '2.2.0-beta.1';
    fakeBackend({
      update_check: () => ({ kind: 'found', update: { id: 3, version: '2.2.0-beta.2', notes: '' } }),
      update_download: () => new Promise(() => {}),
    });
    render(SettingsModal, { page: 'general' });
    expect(screen.getByText('Escouade 2.2.0-beta.1 (bêta)', { selector: '#settings-panel *' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => screen.queryByText('Escouade 2.2.0-beta.2 (bêta) disponible')).not.toBeNull();
  });

  it('shares the usage statistics only once switched on', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    app.settings = { ...app.settings, telemetry: null };
    render(SettingsModal, { page: 'general' });
    const share = screen.getByRole('switch', { name: 'Statistiques d’usage' });
    expect(share).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(share);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ telemetry: true });
    expect(app.settings.telemetry).toBe(true);
  });

  it('stops the usage statistics when switched off', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    app.settings = { ...app.settings, telemetry: true };
    render(SettingsModal, { page: 'general' });
    const share = screen.getByRole('switch', { name: 'Statistiques d’usage' });
    expect(share).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(share);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ telemetry: false });
  });

  it('opens the website, in the language of the interface, and the Discord server', async () => {
    const backend = fakeBackend({ open_url: () => null });
    render(SettingsModal, { page: 'about' });
    expect(screen.getByText('Site web')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir le site' }));
    await userEvent.click(screen.getByRole('button', { name: 'Rejoindre le Discord' }));
    expect(backend.called('open_url').map((c) => c.args.url)).toEqual(['https://escouade.dev/', 'https://discord.gg/m3KYtN8gxP']);
    cleanup();
    setLang('en');
    render(SettingsModal, { page: 'about' });
    await userEvent.click(screen.getByRole('button', { name: 'Open the website' }));
    expect(backend.called('open_url').at(-1)?.args.url).toBe('https://escouade.dev/en/');
    setLang('fr');
  });

  it('offers a coffee, on the Stripe page of the language of the interface, in the browser', async () => {
    const backend = fakeBackend({ open_url: () => null });
    render(SettingsModal, { page: 'about' });
    expect(screen.getByText('Soutenir Escouade')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Offrir un café' }));
    expect(backend.called('open_url').map((c) => c.args.url)).toEqual(['https://buy.stripe.com/8x2fZh20z7rM87067200001']);
    // Said once, in its row: nothing opens by itself.
    expect(screen.queryAllByRole('dialog')).toHaveLength(1);
    cleanup();
    setLang('en');
    render(SettingsModal, { page: 'about' });
    expect(screen.getByText('Support Escouade')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Buy a coffee' }));
    expect(backend.called('open_url').at(-1)?.args.url).toBe('https://buy.stripe.com/00w7sLcFdaDYaf8cvq00000');
    setLang('fr');
  });

  it('says where the local data is, and opens its folder', async () => {
    const backend = fakeBackend({ data_dir: () => 'C:\\Users\\moi\\.escouade', open_path: () => null });
    render(SettingsModal, { page: 'advanced' });
    expect(await screen.findByText('~/.escouade')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir le dossier' }));
    expect(backend.called('open_path').map((c) => c.args.path)).toEqual(['C:\\Users\\moi\\.escouade']);
  });

  it('keeps the focus inside, from the tab it opens on', async () => {
    fakeBackend();
    const outside = document.createElement('button');
    document.body.prepend(outside);
    render(SettingsModal, { page: 'board', projectId: 'p1' });
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    await expect.poll(() => document.activeElement).toBe(tab('Kanban'));
    await userEvent.tab({ shift: true });
    expect(dialog.contains(document.activeElement)).toBe(true);
    outside.remove();
  });

  it('opens on the group a link asks for, on its page only (« Kanban (Workflow) »)', async () => {
    fakeBackend();
    const scrolled = vi.spyOn(Element.prototype, 'scrollIntoView');
    render(SettingsModal, { page: 'board', projectId: 'p1', at: 'workflow' });
    expect(tab('Kanban')).toHaveAttribute('aria-current', 'page');
    await expect.poll(() => scrolled.mock.contexts.length).toBeGreaterThan(0);
    const target = scrolled.mock.contexts.at(-1) as HTMLElement;
    expect(within(target).getByRole('heading', { name: 'Workflow' })).toBeInTheDocument();
    // Another page starts at its top.
    scrolled.mockClear();
    await userEvent.click(tab('Lancement'));
    expect(scrolled).not.toHaveBeenCalled();
    expect(panel().scrollTop).toBe(0);
    scrolled.mockRestore();
  });

  it('comes back with its draft as it was', async () => {
    fakeBackend();
    settingsForm.open({ page: 'advanced' });
    settingsForm.settings.proxyUrl = 'http://proxy:3128';
    settingsForm.page = 'project';
    render(SettingsModal, { page: 'advanced', resume: true });
    expect(tab('Avancé')).toHaveAttribute('aria-current', 'page');
    expect(screen.getByPlaceholderText('aucun')).toHaveValue('http://proxy:3128');
  });

  it('does not close when a selection dragged from a field ends outside', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'advanced' });
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
    render(SettingsModal, { page: 'board' });
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

  it('names its pages and what they set in English, and follows a change of language while it is open', async () => {
    fakeBackend();
    setLang('fr');
    render(SettingsModal);
    expect(screen.getByRole('dialog', { name: 'Réglages' })).toBeInTheDocument();
    expect(screen.getByText('Les harness de ce poste, leurs comptes, et ce qui se passe à la limite')).toBeInTheDocument();
    setLang('en');
    flushSync();
    const dialog = screen.getByRole('dialog', { name: 'Settings' });
    expect(
      within(nav())
        .getAllByRole('button')
        .map((x) => x.textContent?.trim()),
    ).toEqual([
      'General',
      'Agents',
      'Accounts and harnesses',
      'Connected services',
      'Voice',
      'Notifications',
      'Plugins',
      'Advanced',
      'demo-api',
      'site',
      'All projects',
      'About1.3.1',
    ]);
    expect([...nav().querySelectorAll('.s-h')].map((g) => g.textContent)).toEqual(['Escouade', 'Projects']);
    expect(within(nav()).getByRole('searchbox', { name: 'Search settings' })).toBeInTheDocument();
    expect(within(dialog).getByText('This machine’s harnesses, their accounts, and what happens at the limit')).toBeInTheDocument();
    expect(within(dialog).queryByRole('button', { name: 'Save' })).toBeNull();
    expect(group('When a quota runs out').getByRole('group', { name: 'Pause the autopilot above quota' })).toBeInTheDocument();
    expect(screen.getByText('for all projects')).toBeInTheDocument();
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    const path = screen.getByRole('textbox', { name: 'Executable path' });
    expect(path).toHaveAttribute('placeholder', 'claude (found in PATH)');
    app.claudePathFound = false;
    flushSync();
    expect(path).toHaveAttribute('placeholder', 'not found — enter the path to claude.exe');
    expect(screen.getByRole('switch', { name: 'Auto-resume after the usage limit' })).toBeInTheDocument();
    await userEvent.click(tab('Agents'));
    await userEvent.click(within(panel()).getByRole('button', { name: /^Advanced/ }));
    expect(screen.getByRole('spinbutton', { name: 'Stop idle agent processes after (minutes)' })).toBeInTheDocument();
    await userEvent.type(screen.getByRole('spinbutton', { name: /Stop idle/ }), '0');
  });

  it('names the task list setting in English', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'agents' });
    const todo = screen.getByRole('switch', { name: 'Agents keep a task list' });
    expect(todo).toHaveAttribute('aria-checked', 'true');
    const help = screen.getByText(/Gives agents Claude Code’s task list tool/);
    expect(help).toHaveTextContent(/if they keep a list, Escouade shows their progress in the “Plan” banner/);
    expect(help).toHaveTextContent(/the model does not have to use it/);
    expect(help).not.toHaveTextContent(/so Escouade can show/);
    expect(screen.getByRole('heading', { name: 'New agents' })).toBeInTheDocument();
  });

  it('names the remote control setting for all agents in English', async () => {
    const backend = backendSaving();
    render(SettingsModal, { page: 'agents' });
    const remote = group('Remote control');
    const all = remote.getByRole('switch', { name: 'Remote control for all agents' });
    expect(remote.getByText('Claude Code only: the other harnesses have no remote control.')).toBeInTheDocument();
    expect(all).toHaveAttribute('aria-checked', 'false');
    expect(remote.getByText(/^Every agent of every project/)).toHaveTextContent(
      'Every agent of every project, existing or new (ticket agents included), is reachable from claude.ai and the Claude app. Escouade starts each one’s claude process right away, and it stays running. The session appears on claude.ai as “project · agent” (for a ticket agent, the name comes from the ticket). In Claude Code view, an interface that is already open only picks it up at its next launch. To set an agent apart: right-click, “Turn off remote control”.',
    );
    await userEvent.click(all);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ remoteControlAll: true });
  });

  it('says of the executable path whether it is found, not whether the current account’s Claude Code is', async () => {
    fakeBackend();
    // An account with a `claude` of its own is the current one: Claude Code is found, the settings' path is not.
    app.claudeFound = true;
    app.claudePathFound = false;
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
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
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
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
    render(SettingsModal, { page: 'accounts' });
    expect(await screen.findByText('Not found. Give the path to claude.')).toBeInTheDocument();
    cleanup();
    fakeBackend();
    render(SettingsModal, { page: 'accounts' });
    await new Promise((r) => setTimeout(r, 400));
    expect(screen.queryByText(/^Found|^Not found/)).toBeNull();
  });

  it('says the switch to another account in English', () => {
    fakeBackend();
    app.settings.accounts = [
      { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
      { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true },
    ];
    render(SettingsModal, { page: 'accounts' });
    expect(group('When a quota runs out').getByRole('heading', { name: 'When a quota runs out' })).toBeInTheDocument();
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
    ).toEqual(['Notify me about', 'Pull requests and deployments', 'Channels', 'Sound']);
    expect(
      group('Notify me about')
        .getAllByRole('checkbox')
        .map((s) => s.closest('label')!.textContent!.trim()),
    ).toEqual(['Questions and permissions', 'Finished tasks', 'Errors', 'Tickets (ready to review, blocked)']);
    expect(panel()).toHaveTextContent(
      /Off: no system notification or chime for this type; the tab, the card, and the (taskbar|Dock) still flag the agent\./,
    );
    expect(panel()).toHaveTextContent(/(Windows|macOS) notifications when the app is not in the foreground/);
    expect(screen.getByRole('button', { name: 'Test the sound' })).toBeInTheDocument();

    await userEvent.click(tab('Advanced'));
    expect(panel()).toHaveTextContent('Detected: PowerShell 7, Git Bash');
    expect(screen.getByRole('textbox', { name: /WSL distribution/ })).toHaveAccessibleName('WSL distribution');

    expect(screen.getByPlaceholderText('none')).toBeInTheDocument();
    expect(panel()).toHaveTextContent('NO_PROXY, separated by commas');
    expect(screen.getByRole('switch', { name: 'Skip TLS certificate verification' })).toHaveAttribute('aria-checked', 'false');

    expect(within(panel()).getByRole('heading', { name: 'Local data' })).toBeInTheDocument();
    expect(
      within(panel())
        .getAllByRole('heading')
        .map((h) => h.textContent),
    ).toEqual(['Network', 'Terminals', 'Attachments', 'Search by meaning', 'Local data']);

    await userEvent.click(tab('General'));
    expect(screen.getByRole('button', { name: 'Check for updates' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Install updates automatically' })).toBeInTheDocument();

    await userEvent.click(tab(/^About/));
    expect(screen.getByRole('button', { name: 'Open the website' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Check for updates' })).toBeNull();
  });

  it('says a project’s settings in English, the worktree commands and the launch commands included', async () => {
    app.projects = [project({ runCommands: [FRONT], worktreeSetup: [{ id: 'w1', command: 'npm ci', shell: 'ghost', cwd: '' }] })];
    backendSaving({
      suggest_run_commands: () => ({ commands: [{ id: 's1', name: 'API', command: 'cargo run', shell: 'bash', cwd: '' }], refused: 2 }),
    });
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    const h2 = () =>
      within(panel())
        .queryAllByRole('heading', { level: 2 })
        .map((h) => h.textContent);
    expect(h2()).toEqual(['Identity']);
    // Folded at its foot, named with what it hides.
    expect(within(panel()).getByRole('button', { name: /^Advanced/ })).toHaveTextContent('AdvancedClose the project');
    // The assistant is a row of the identity (2.0 design).
    expect(screen.getByRole('button', { name: 'Open the assistant…' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Project name' })).toHaveValue('demo-api');
    expect(screen.getByRole('button', { name: 'Color 4' })).toBeInTheDocument();
    await userEvent.click(tab('Git and worktrees'));
    expect(h2()).toEqual(['Git', 'Worktrees']);
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

    await userEvent.click(tab('Launch'));
    expect(h2()).toEqual(['Launch']);
    const launch = group('Launch');
    expect(launch.getByText('Command 1', { selector: 'legend' })).toBeInTheDocument();
    expect(launch.getByLabelText(/^Subfolder/)).toHaveValue('web');
    expect(launch.getByLabelText('Subfolder (blank = project folder)')).toHaveValue('web');
    await userEvent.click(launch.getByRole('button', { name: 'Fill in automatically' }));
    // A plural, and what was left out said in the singular and the plural of the count.
    await expect
      .poll(() => app.toasts.at(-1)?.text)
      .toBe('1 command suggested, 2 left out (invisible characters or too long): review it before replacing the commands.');
    const proposal = within(await launch.findByRole('region', { name: 'Suggested launch commands' }));
    expect(proposal.getByText('the project folder')).toBeInTheDocument();
    expect(proposal.getByRole('button', { name: 'Ignore' })).toBeInTheDocument();
    expect(proposal.getByRole('button', { name: 'Replace the commands' })).toBeInTheDocument();

    await userEvent.click(tab('Team'));
    expect(h2()).toEqual(['Shared Kanban', 'Shared settings', 'Shared memory']);
    await userEvent.click(tab('Privacy'));
    expect(h2()).toEqual(['Remote control', 'Plugins in this project']);
    await userEvent.click(tab('Kanban'));
    expect(h2().at(-1)).toBe('Workflow');
    await userEvent.click(tab('Project'));
    await userEvent.click(within(panel()).getByRole('button', { name: /^Advanced/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Close the project…' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Close “demo-api”?' });
  });

  it('says what keeps the settings from being saved in English', async () => {
    backendSaving();
    render(SettingsModal, { page: 'project', projectId: 'p1' });
    await userEvent.clear(screen.getByRole('textbox', { name: 'Project name' }));
    expect(screen.getByText('A project needs a name: “demo-api” stays saved.')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Project name' }), 'api');
    await userEvent.click(tab('Launch'));
    await userEvent.clear(within(screen.getByRole('group', { name: 'Command 1' })).getByLabelText('Command'));
    expect(
      screen.getByText('Each command needs a name and a command line: the saved commands stay as they were until none is missing one.'),
    ).toBeInTheDocument();
  });

  it('says there is no project open in English', () => {
    resetApp({ projects: [] });
    fakeBackend();
    render(SettingsModal, { page: 'project' });
    expect(screen.getByText('No project open.')).toBeInTheDocument();
  });

  it('tells the update check in English, and saves a switch without a toast', async () => {
    const backend = fakeBackend({ save_settings: () => [], update_check: () => ({ kind: 'none' }) });
    render(SettingsModal, { page: 'general' });
    await userEvent.click(screen.getByRole('button', { name: 'Check for updates' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('No update available.');
    await userEvent.click(screen.getByRole('switch', { name: 'Install updates automatically' }));
    await afterGesture();
    expect(backend.called('save_settings')).toHaveLength(1);
    expect(app.toasts.at(-1)?.text).toBe('No update available.');
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
    render(SettingsModal, { page: 'accounts' });
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
    render(SettingsModal, { page: 'accounts' });
    expect(await screen.findByText(refusal)).toBeInTheDocument();
    expect(screen.getByText('2.0.14')).toHaveClass('bad');
    expect(screen.getByText('2.0.14')).toHaveAccessibleDescription(refusal);
    cleanup();
    answer = info({ state: 'notConnected' });
    fakeBackend({ claude_locate: () => located, engines: () => [answer] });
    render(SettingsModal, { page: 'accounts' });
    expect(
      await screen.findByText('Aucun compte connecté : connecte l’un de ses comptes ci-dessous, ou ajoutes-en un.'),
    ).toBeInTheDocument();
  });

  it('shows a version newer than the tested ones as ready, with no warning (ESC-159)', async () => {
    fakeBackend({ claude_locate: () => located, engines: () => [info({ version: '2.1.999' })] });
    render(SettingsModal, { page: 'accounts' });
    const version = await screen.findByText('2.1.999');
    expect(version).toHaveAttribute('data-state', 'ready');
    expect(version).not.toHaveClass('warn');
    expect(version).not.toHaveClass('bad');
    expect(version).not.toHaveAttribute('aria-describedby');
    expect(screen.queryByText(/testé|tested/i)).toBeNull();
  });

  it('shows no version while the path typed is not the saved one, nor for a claude not found', async () => {
    const backend = fakeBackend({ claude_locate: () => located, engines: () => [info({})] });
    render(SettingsModal, { page: 'accounts' });
    expect(await screen.findByText('2.1.301')).toBeInTheDocument();
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    await userEvent.type(screen.getByRole('textbox', { name: "Chemin de l'exécutable" }), 'C:\autre\claude.exe');
    await waitFor(() => expect(screen.queryByText('2.1.301')).toBeNull());
    expect(backend.called('engines').length).toBeGreaterThan(0);
  });

  it('says it in English too', async () => {
    setLang('en');
    fakeBackend({ claude_locate: () => located, engines: () => [info({ state: 'notConnected' })] });
    render(SettingsModal, { page: 'accounts' });
    expect(await screen.findByText('No account signed in: sign in one of its accounts below, or add one.')).toBeInTheDocument();
    cleanup();
    fakeBackend({ claude_locate: () => located, engines: () => [info({ version: null })] });
    render(SettingsModal, { page: 'accounts' });
    expect(await screen.findByText('Unknown version: claude didn’t tell it.')).toBeInTheDocument();
  });

  it('hides the settings of what no harness does', () => {
    const labels = ['Les agents tiennent une liste de tâches', 'Remote control pour tous les agents'];
    const view = 'Vue des nouveaux agents';
    fakeBackend({ claude_locate: () => located });
    render(SettingsModal, { page: 'agents' });
    for (const name of labels) expect(screen.getByRole('switch', { name })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: view })).toBeInTheDocument();
    cleanup();
    app.engines = [BARE_ENGINE];
    fakeBackend({ claude_locate: () => located });
    render(SettingsModal, { page: 'agents' });
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
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    expect(screen.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'true');
    expect(await screen.findByText('Mis à jour en 2.1.302 (depuis la 2.1.301).')).toBeInTheDocument();
    cleanup();
    answer = status({ state: 'upToDate', version: '2.1.302', at: 1 });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    expect(await screen.findByText('À jour (2.1.302).')).toBeInTheDocument();
    cleanup();
    answer = status({ state: 'failed', line: 'Error: EACCES: permission denied', at: 1 });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    expect(await screen.findByText('Échec de la mise à jour : Error: EACCES: permission denied')).toHaveClass('bad');
    cleanup();
    answer = status({ state: 'userOff', line: 'DISABLE_AUTOUPDATER' });
    fakeBackend({ claude_locate: () => located, engines: () => [info()], claude_update_status: () => answer });
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
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
    render(SettingsModal, { page: 'accounts' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    expect(await screen.findByText('À jour (2.1.302).')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: SWITCH }));
    expect(
      screen.getByText(
        'Coupée : Claude Code n’est plus mis à jour, ni par Escouade ni par lui-même. Lance claude update de temps en temps.',
      ),
    ).toBeInTheDocument();
    await afterGesture();
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
    render(SettingsModal, { page: 'accounts' });
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

  it('lists each harness with its state and its version beside its name, its accounts under it, its path in « Avancé »', async () => {
    fakeBackend({ claude_locate: () => located, engines: () => [claude(), bare()] });
    render(SettingsModal, { page: 'accounts' });
    const own = await screen.findByRole('region', { name: 'Claude Code' });
    expect(within(own).getByRole('heading', { name: 'Claude Code' })).toBeInTheDocument();
    expect(within(own).getByRole('button', { name: 'Ajouter un compte Claude Code…' })).toBeInTheDocument();
    expect(within(own).getByRole('listitem', { name: 'Principal' })).toBeInTheDocument();
    expect(within(own).queryByRole('textbox', { name: "Chemin de l'exécutable" })).toBeNull();
    expect(await within(own).findByText('2.1.301')).toHaveAttribute('data-state', 'notConnected');
    expect(
      within(own).getByText(/Aucun compte connecté : connecte l’un de ses comptes ci-dessous, ou ajoutes-en un\.$/),
    ).toBeInTheDocument();
    const other = screen.getByRole('region', { name: 'Harness nu' });
    expect(within(other).getByText('0.163.0')).toHaveAttribute('data-state', 'ready');
    expect(within(other).getByText(/^Prêt/)).toBeInTheDocument();
    await userEvent.click(within(panel()).getByRole('button', { name: /^(Avancé|Advanced)/ }));
    expect(screen.getByRole('textbox', { name: "Chemin de l'exécutable" })).toBeInTheDocument();
  });

  it('says that the cost of a harness that does not tell it is estimated, and what it cannot do', async () => {
    const estimated = { ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, cost: 'estimated' as const } };
    app.engines = [CLAUDE_ENGINE, estimated];
    fakeBackend({ claude_locate: () => located, engines: () => [claude({ state: 'ready' }), bare({ caps: estimated.caps })] });
    render(SettingsModal, { page: 'accounts' });
    const other = await screen.findByRole('region', { name: 'Harness nu' });
    expect(within(other).getByText('Coût')).toBeInTheDocument();
    expect(
      within(other).getByText(
        'Estimé d’après les tokens et les tarifs publics : Harness nu ne donne pas le coût de ses tours. Il est écrit « ≈ » partout.',
      ),
    ).toBeInTheDocument();
    expect(within(other).getByText('Pas de PDF joint · pas de tâches de fond')).toBeInTheDocument();
    expect(within(screen.getByRole('region', { name: 'Claude Code' })).queryByText('Coût')).toBeNull();
  });

  it('gives the command that installs a harness that is not found', async () => {
    fakeBackend({
      claude_locate: () => located,
      engines: () => [
        claude({ state: 'ready' }),
        bare({ located: { path: null, wsl: null, nat: false, canWsl: true }, version: null, state: 'absent' }),
      ],
    });
    render(SettingsModal, { page: 'accounts' });
    const other = await screen.findByRole('region', { name: 'Harness nu' });
    expect(await within(other).findByText('Introuvable')).toHaveAttribute('data-engine-state', 'absent');
    expect(within(other).getByText('npm install -g @openai/codex')).toBeInTheDocument();
    expect(await within(screen.getByRole('region', { name: 'Claude Code' })).findByText('2.1.301')).toHaveAttribute('data-state', 'ready');
    // Its path is not set here (Claude Code's alone is): no « Indique le chemin de claude » under it (ESC-84).
    expect(within(other).queryByText(/chemin de claude/)).toBeNull();
  });

  it('says it in English too', async () => {
    setLang('en');
    fakeBackend({ claude_locate: () => located, engines: () => [claude(), bare()] });
    render(SettingsModal, { page: 'accounts' });
    const own = await screen.findByRole('region', { name: 'Claude Code' });
    expect(await within(own).findByText(/No account signed in: sign in one of its accounts below, or add one\.$/)).toBeInTheDocument();
    expect(within(screen.getByRole('region', { name: 'Harness nu' })).getByText(/^Ready/)).toBeInTheDocument();
  });
});

describe('SettingsModal, Général › Mises à jour (choice 13 of the 2.0 design)', () => {
  beforeEach(() => resetApp());

  it('keeps a download of an update that failed while it lasts, with the way through the browser', async () => {
    fakeBackend();
    render(SettingsModal, { page: 'general' });
    expect(screen.queryByText(/^Mise à jour /)).toBeNull();
    cleanup();
    app.updateTrouble = { version: '1.9.0', message: 'le proxy a refusé le fichier (403)', page: 'https://example.com/v1.9.0' };
    render(SettingsModal, { page: 'general' });
    expect(screen.getByText('Mise à jour 1.9.0')).toBeInTheDocument();
    expect(screen.getByText('Téléchargement impossible : le proxy a refusé le fichier (403)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Télécharger dans le navigateur' })).toBeInTheDocument();
    cleanup();
    app.updateTrouble = { version: '1.9.0', message: 'délai dépassé', page: null };
    render(SettingsModal, { page: 'general' });
    expect(screen.getByText('Téléchargement impossible : délai dépassé')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Télécharger dans le navigateur' })).toBeNull();
  });

  /** The backend with an update found (each test its own version: the updater keeps the one it found), its download held until `ready()`. */
  function found(version: string, over: Record<string, (a: any) => unknown> = {}) {
    let ready!: (answer: unknown) => void;
    const download = new Promise((r) => (ready = r));
    const backend = fakeBackend({
      update_check: () => ({ kind: 'found', update: { id: 1, version, notes: `- Nouveauté de la ${version}` } }),
      update_download: () => download,
      update_close: () => null,
      ...over,
    });
    return { backend, ready: (answer: unknown = { kind: 'ready' }) => ready(answer) };
  }
  /** The row of the update in « À propos ». */
  const updateRow = () => screen.getByText(/^Escouade \S+ (disponible|prête)$/).closest('.sr') as HTMLElement;

  it('shows an update found by the button while it downloads, then ready, and says so to a screen reader (ESC-250)', async () => {
    const { ready } = found('2.5.0');
    render(SettingsModal, { page: 'general' });
    const live = screen.getByTestId('update-live');
    expect(live).toHaveAttribute('aria-live', 'polite');
    expect(live).toHaveTextContent('');
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    expect(await screen.findByText('Escouade 2.5.0 disponible')).toBeInTheDocument();
    expect(within(updateRow()).getByText('Téléchargement…')).toBeInTheDocument();
    expect(live).toHaveTextContent('Escouade 2.5.0 disponible : téléchargement…');
    // Found: no « Aucune mise à jour » toast.
    expect(app.toasts).toEqual([]);
    ready();
    expect(await screen.findByText('Escouade 2.5.0 prête')).toBeInTheDocument();
    expect(screen.queryByText('Escouade 2.5.0 disponible')).toBeNull();
    expect(live).toHaveTextContent('Escouade 2.5.0 prête : redémarre pour l’installer.');
  });

  it('shows an update the automatic check found, ready, its restart through the update’s window and its notes', async () => {
    const backend = fakeBackend();
    app.update = { version: '2.6.0', notes: '- Une rangée dans À propos', ready: true };
    render(SettingsModal, { page: 'general' });
    const row = within(updateRow());
    expect(row.getByText('Escouade 2.6.0 prête')).toBeInTheDocument();
    // The notes, folded until asked for.
    expect(screen.queryByText('Une rangée dans À propos')).toBeNull();
    await userEvent.click(row.getByRole('button', { name: 'Voir les nouveautés' }));
    expect(screen.getByText('Une rangée dans À propos')).toBeInTheDocument();
    expect(row.getByRole('button', { name: 'Masquer les nouveautés' })).toHaveAttribute('aria-expanded', 'true');
    // As the status bar: the update’s window (unsaved files, agents at work), which comes back here once put off.
    await userEvent.click(row.getByRole('button', { name: 'Redémarrer maintenant' }));
    expect(app.modal).toEqual({ kind: 'update', back: 'general' });
    expect(backend.called('update_restart')).toEqual([]);
  });

  it('says the restart the backend planned, put off from here as from the status bar', async () => {
    const backend = fakeBackend({ update_postpone: () => null });
    app.update = { version: '2.6.1', notes: '', ready: true };
    app.now = 1_000_000;
    app.restartAt = 1_000_000 + 9_500;
    render(SettingsModal, { page: 'general' });
    const row = within(updateRow());
    expect(row.getByText('Redémarrage dans 10 s')).toBeInTheDocument();
    // No notes: nothing to unfold.
    expect(row.queryByRole('button', { name: 'Voir les nouveautés' })).toBeNull();
    await userEvent.click(row.getByRole('button', { name: 'Plus tard' }));
    expect(app.restartAt).toBeNull();
    expect(backend.called('update_postpone')).toHaveLength(1);
    expect(row.queryByText(/^Redémarrage dans/)).toBeNull();
  });

  it('finds and offers an update with the automatic updates turned off', async () => {
    found('2.7.0');
    render(SettingsModal, { page: 'general' });
    await userEvent.click(screen.getByRole('switch', { name: 'Installer les mises à jour automatiquement' }));
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    expect(await screen.findByText('Escouade 2.7.0 disponible')).toBeInTheDocument();
  });

  it('shows nothing found with the toast alone, and a failed check with its error alone', async () => {
    fakeBackend({ update_check: () => ({ kind: 'none' }) });
    render(SettingsModal, { page: 'general' });
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Aucune mise à jour disponible.');
    expect(screen.queryByText(/^Escouade \S+ (disponible|prête)$/)).toBeNull();
    fakeBackend({ update_check: () => ({ kind: 'failed', trouble: { message: 'délai dépassé', show: true, page: null } }) });
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Vérification des mises à jour impossible : délai dépassé');
    expect(screen.queryByText(/^Escouade \S+ (disponible|prête)$/)).toBeNull();
    expect(screen.getByTestId('update-live')).toHaveTextContent('');
  });

  it('gives way to the download that failed, never both rows, and to a newer version found meanwhile', async () => {
    const { ready } = found('2.8.0');
    render(SettingsModal, { page: 'general' });
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    expect(await screen.findByText('Escouade 2.8.0 disponible')).toBeInTheDocument();
    ready({ kind: 'failed', trouble: { message: 'le proxy a refusé le fichier (403)', show: false, page: null } });
    expect(await screen.findByText('Mise à jour 2.8.0')).toBeInTheDocument();
    expect(screen.queryByText(/^Escouade \S+ (disponible|prête)$/)).toBeNull();
    // Looked for again: the failure stays said while the download is tried again.
    found('2.8.0');
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    await expect.poll(() => app.update?.version).toBe('2.8.0');
    expect(screen.getByText('Mise à jour 2.8.0')).toBeInTheDocument();
    expect(screen.queryByText('Escouade 2.8.0 disponible')).toBeNull();
    // A newer one found meanwhile takes the row.
    const newer = found('2.9.0');
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher une mise à jour' }));
    expect(await screen.findByText('Escouade 2.9.0 disponible')).toBeInTheDocument();
    expect(screen.queryByText('Mise à jour 2.8.0')).toBeNull();
    newer.ready();
    expect(await screen.findByText('Escouade 2.9.0 prête')).toBeInTheDocument();
    expect(screen.queryByText(/2\.8\.0/)).toBeNull();
  });
});
