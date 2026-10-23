import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, branchInfo, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const save = () => userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));

describe('ProjectTab', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
  });

  it('has no switch of its own for Escouade’s tools: the app’s one gives them to every agent (ESC-91)', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.queryByRole('switch', { name: 'Les agents peuvent utiliser Escouade' })).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Serveur MCP' })).not.toBeInTheDocument();
    // A change of the project saves nothing of the kind.
    await userEvent.click(screen.getByRole('switch', { name: 'Un worktree par agent' }));
    await save();
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project')[0].args.project).not.toHaveProperty('agentsUseEscouade');
  });

  it('makes a new agent’s worktree at its first message, under its name, until switched off (ESC-156)', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const toggle = screen.getByRole('switch', { name: 'Créer le worktree au premier message, au nom de l’agent' });
    // On for a project that does not say (saved before it).
    expect(toggle).toBeChecked();
    expect(
      screen.getByText(
        'Le premier message nomme l’agent, puis sa branche et son dossier prennent ce nom avant la préparation. Éteint, le worktree est créé tout de suite, en agent-N.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(toggle);
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', worktreeAtFirstMessage: false });
  });

  it('starts the new worktrees from the folder’s branch, the board’s target or a branch chosen (ESC-160)', async () => {
    const backend = fakeBackend({
      branch_list: () => [
        branchInfo({ name: 'main', current: true }),
        branchInfo({ name: 'release' }),
        branchInfo({ name: 'origin/hotfix', remote: true }),
      ],
    });
    app.git = { p1: gitInfo({ branch: 'main' }) };
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const group = screen.getByRole('group', { name: 'Branche de départ des nouveaux worktrees' });
    const chip = (name: RegExp | string) => within(group).getByRole('button', { name });
    // The folder's branch for a project that does not say (saved before it).
    expect(chip('Branche actuelle du dossier')).toHaveAttribute('aria-pressed', 'true');
    expect(
      screen.getByText('Ils partent de la branche sur laquelle est le dossier du projet au moment de leur création.'),
    ).toBeInTheDocument();
    await userEvent.click(chip('Branche cible du Kanban'));
    expect(chip('Branche cible du Kanban')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText(/^Ils partent de main, mise à jour depuis le distant/)).toBeInTheDocument();
    // A branch, chosen in the picker: any, a remote one too.
    await userEvent.click(chip('Une branche…'));
    const picker = await screen.findByRole('dialog', { name: 'Branche de départ' });
    await within(picker).findByText('origin/hotfix');
    await userEvent.click(within(picker).getByText('origin/hotfix'));
    expect(screen.queryByRole('dialog', { name: 'Branche de départ' })).toBeNull();
    const named = chip('Branche de départ : origin/hotfix. Choisir une autre branche');
    expect(named).toHaveAttribute('aria-pressed', 'true');
    expect(named).toHaveTextContent('⎇ origin/hotfix ▾');
    expect(screen.getByText(/^Ils partent de origin\/hotfix,/)).toBeInTheDocument();
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({
      id: 'p1',
      worktreeBase: { kind: 'branch', branch: 'origin/hotfix' },
    });
  });

  it('shows the branch the project starts from, and saves the folder’s again when chosen', async () => {
    resetApp({ projects: [project({ worktreeBase: { kind: 'branch', branch: 'release' } })] });
    app.modal = { kind: 'settings' };
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const group = screen.getByRole('group', { name: 'Branche de départ des nouveaux worktrees' });
    expect(within(group).getByRole('button', { name: /release/ })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(group).getByRole('button', { name: 'Branche actuelle du dossier' }));
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ worktreeBase: { kind: 'head' } });
  });

  it('opens the setup assistant again on the project (ESC-94)', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p2' });
    expect(screen.getByText('Revoir les réglages du projet étape par étape, ou y copier ceux d’un autre projet.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir l’assistant…' }));
    expect(app.modal).toEqual({ kind: 'newProject', projectId: 'p2' });
  });

  it('cleans up the worktrees archived agents left, and says which it kept and why (ESC-143)', async () => {
    let done: (r: unknown) => void = () => {};
    const backend = fakeBackend({ clean_worktrees: () => new Promise((r) => (done = r)) });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(
      screen.getByText(
        'Supprime ceux des agents archivés, après leurs commandes de démontage, sauf ceux qui gardent des changements non commités ou des commits ni mergés ni poussés.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Nettoyer' }));
    expect(screen.getByRole('button', { name: 'Nettoyage…' })).toBeDisabled();
    expect(backend.called('clean_worktrees')[0].args).toEqual({ projectId: 'p1' });
    done({ removed: 2, kept: [{ agentId: 'a1', name: 'api', reason: 'worktree gardé : 1 fichier(s) non commité(s)' }] });
    const status = await screen.findByRole('status');
    expect(status).toHaveTextContent('2 worktrees supprimés. 1 gardé :');
    expect(status).toHaveTextContent('api · worktree gardé : 1 fichier(s) non commité(s)');
    expect(screen.getByRole('button', { name: 'Nettoyer' })).toBeEnabled();
  });

  it('says when no worktree was left to clean up', async () => {
    fakeBackend({ clean_worktrees: () => ({ removed: 0, kept: [] }) });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Nettoyer' }));
    expect(await screen.findByRole('status')).toHaveTextContent('Aucun worktree à supprimer.');
  });

  describe('remote control', () => {
    const LABEL = 'Remote control pour les agents de ce projet';
    const REASON = 'Activé pour tous les projets dans Réglages › Harness';
    const tab = (name: string) => screen.getByRole('tab', { name });
    const headings = () => screen.getAllByRole('heading').map((h) => h.textContent);
    /** The modal on « Claude Code », the setting for all agents switched on or off, then on the project’s tab. */
    const toProjects = async (global: boolean, [allLabel, projectsTab] = ['Remote control pour tous les agents', 'Projets']) => {
      render(SettingsModal, { tab: 'claude' });
      const all = screen.getByRole('switch', { name: allLabel });
      if (global !== (all.getAttribute('aria-checked') === 'true')) await userEvent.click(all);
      await userEvent.click(tab(projectsTab));
    };

    it('is hidden while no harness has Remote Control', () => {
      fakeBackend();
      app.engines = [BARE_ENGINE];
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      expect(screen.queryByRole('switch', { name: LABEL })).toBeNull();
      expect(headings()).not.toContain('Remote control');
    });

    it('has its own group after the project’s identity, off by default, and says what it does', () => {
      fakeBackend();
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      const names = headings();
      expect(names.indexOf('Remote control')).toBe(names.indexOf('Identité') + 1);
      expect(names.indexOf('Confidentialité')).toBe(names.indexOf('Remote control') + 1);
      expect(names.indexOf('Git')).toBe(names.indexOf('Confidentialité') + 1);
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'false');
      expect(remote).toBeEnabled();
      expect(screen.getByText(/^Chaque agent de ce projet, existant ou nouveau/)).toHaveTextContent(
        'Chaque agent de ce projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ».',
      );
      expect(screen.queryByText(REASON)).not.toBeInTheDocument();
    });

    it('is saved with the project when switched on, and shows what the project has', async () => {
      const backend = fakeBackend();
      resetApp({ projects: [project({ remoteControl: true }), project({ id: 'p2', name: 'site' })] });
      app.modal = { kind: 'settings' };
      render(SettingsModal, { tab: 'projects', projectId: 'p2' });
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'false');
      await userEvent.click(screen.getByRole('switch', { name: LABEL }));
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
      expect(tab('Projets')).toHaveClass('changed');
      expect(backend.called('update_project')).toHaveLength(0);
      await save();
      expect(backend.called('update_project')).toHaveLength(1);
      expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p2', remoteControl: true });
      expect(app.projects[1].remoteControl).toBe(true);
      // The other project kept its own value.
      expect(app.projects[0].remoteControl).toBe(true);
    });

    it('is switched on by a project that has it, and saves nothing when put back', async () => {
      const backend = fakeBackend();
      resetApp({ projects: [project({ remoteControl: true })] });
      app.modal = { kind: 'settings' };
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'true');
      await userEvent.click(remote);
      await userEvent.click(remote);
      expect(tab('Projets')).not.toHaveClass('changed');
      await save();
      expect(backend.called('update_project')).toHaveLength(0);
    });

    it('is shown on and disabled while the setting for all agents is on in the modal, with the reason in view', async () => {
      const backend = fakeBackend({ save_settings: () => [] });
      await toProjects(true);
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'true');
      expect(remote).toBeDisabled();
      expect(screen.getByText(REASON)).toBeVisible();
      expect(remote).toHaveAccessibleDescription(REASON);
      // A click changes nothing, and nothing of the project is saved.
      await userEvent.click(remote);
      expect(remote).toHaveAttribute('aria-checked', 'true');
      expect(tab('Projets')).not.toHaveClass('changed');
      await save();
      expect(backend.called('save_settings')[0].args.settings).toMatchObject({ remoteControlAll: true });
      expect(backend.called('update_project')).toHaveLength(0);
      expect(app.projects[0].remoteControl).toBe(false);
    });

    it('finds its own value and is active again once the setting for all agents is switched off', async () => {
      fakeBackend();
      await toProjects(true);
      expect(screen.getByRole('switch', { name: LABEL })).toBeDisabled();
      await userEvent.click(tab('Harness'));
      await userEvent.click(screen.getByRole('switch', { name: 'Remote control pour tous les agents' }));
      await userEvent.click(tab('Projets'));
      const remote = screen.getByRole('switch', { name: LABEL });
      expect(remote).toHaveAttribute('aria-checked', 'false');
      expect(remote).toBeEnabled();
      expect(screen.queryByText(REASON)).not.toBeInTheDocument();
      expect(remote).not.toHaveAccessibleDescription(REASON);
    });

    it('keeps the project’s own value under the setting for all agents, on until the project turns it off', async () => {
      const backend = fakeBackend({ save_settings: () => [] });
      resetApp({ projects: [project({ remoteControl: true })] });
      app.modal = { kind: 'settings' };
      await toProjects(true);
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByRole('switch', { name: LABEL })).toBeDisabled();
      await userEvent.click(tab('Harness'));
      await userEvent.click(screen.getByRole('switch', { name: 'Remote control pour tous les agents' }));
      await userEvent.click(tab('Projets'));
      expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByRole('switch', { name: LABEL })).toBeEnabled();
      expect(backend.called('update_project')).toHaveLength(0);
    });

    it('says it in English', async () => {
      setLang('en');
      fakeBackend();
      await toProjects(true, ['Remote control for all agents', 'Projects']);
      const remote = screen.getByRole('switch', { name: 'Remote control for this project’s agents' });
      expect(remote).toBeDisabled();
      expect(remote).toHaveAttribute('aria-checked', 'true');
      expect(screen.getByText('On for all projects in Settings › Harness')).toBeVisible();
      expect(screen.getByText(/^Every agent of this project/)).toHaveTextContent(
        'Every agent of this project, existing or new (ticket agents included), is reachable from claude.ai and the Claude app. Escouade starts each one’s claude process right away, and it stays running. The session appears on claude.ai as “project · agent” (for a ticket agent, the name comes from the ticket). In Claude Code view, an interface that is already open only picks it up at its next launch. To set an agent apart: right-click, “Turn off remote control”.',
      );
      expect(screen.getAllByRole('heading').map((h) => h.textContent)).toContain('Remote control');
    });
  });

  it('sets where the worktrees go and how their branches are named, and says where the next one goes', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const dir = screen.getByRole('textbox', { name: 'Dossier des worktrees' });
    expect(dir).toHaveValue('');
    expect(dir).toHaveAttribute('placeholder', '.claude/worktrees');
    expect(dir).toHaveAccessibleDescription(/Le prochain : C:\\code\\demo-api\\\.claude\\worktrees\\<nom>/);
    const agents = screen.getByRole('textbox', { name: 'Préfixe des branches des agents' });
    const tickets = screen.getByRole('textbox', { name: 'Préfixe des branches des tickets' });
    expect(agents).toHaveValue('escouade/');
    expect(tickets).toHaveValue('ticket/');
    expect(screen.getByText('Branche d’un nouvel agent : escouade/<nom>. Vide : son nom seul.')).toBeInTheDocument();
    await userEvent.type(dir, '../demo-api-worktrees');
    expect(screen.getByText('Le prochain : C:\\code\\demo-api-worktrees\\<nom>')).toBeInTheDocument();
    await userEvent.clear(agents);
    await userEvent.type(agents, 'gg/');
    expect(screen.getByText('Branche d’un nouvel agent : gg/<nom>. Vide : son nom seul.')).toBeInTheDocument();
    await userEvent.clear(tickets);
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({
      id: 'p1',
      worktreeDir: '../demo-api-worktrees',
      agentBranchPrefix: 'gg/',
      ticketBranchPrefix: '',
    });
    expect(app.projects[0]).toMatchObject({ worktreeDir: '../demo-api-worktrees', agentBranchPrefix: 'gg/', ticketBranchPrefix: '' });
  });

  it('refuses a worktrees folder or a branch prefix that cannot be, and saves nothing until it is fixed', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const saveButton = screen.getByRole('button', { name: 'Enregistrer' });
    const dir = screen.getByRole('textbox', { name: 'Dossier des worktrees' });
    await userEvent.type(dir, '../demo-api');
    expect(dir).toHaveAttribute('aria-invalid', 'true');
    expect(dir).toHaveAccessibleDescription(/C’est le dossier du projet lui-même\./);
    expect(screen.getByText('Le dossier des worktrees ou un préfixe de branche de « demo-api » n’est pas valide.')).toBeInTheDocument();
    expect(saveButton).toBeDisabled();
    await userEvent.clear(dir);
    await userEvent.type(dir, '..');
    expect(dir).toHaveAccessibleDescription(/Ce dossier contient celui du projet : choisis-en un à côté/);
    await userEvent.clear(dir);
    await userEvent.type(dir, 'D:wt');
    expect(dir).toHaveAccessibleDescription(/Écris un chemin relatif au dossier du projet, ou un chemin complet \(C:\\…\)\./);
    await userEvent.clear(dir);
    await userEvent.type(dir, '.git/wt');
    expect(dir).toHaveAccessibleDescription(/Ce dossier est dans un dossier \.git\./);
    await userEvent.clear(dir);
    expect(dir).toHaveAttribute('aria-invalid', 'false');
    expect(saveButton).toBeEnabled();
    const tickets = screen.getByRole('textbox', { name: 'Préfixe des branches des tickets' });
    await userEvent.type(tickets, ' x');
    expect(tickets).toHaveAttribute('aria-invalid', 'true');
    expect(tickets).toHaveAccessibleDescription(/Aucun nom de branche ne commence ainsi.*« \\ » ou « @\{ »/);
    expect(saveButton).toBeDisabled();
    await userEvent.click(saveButton);
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('says in the sensitive zone that closing the project keeps its history, and offers to erase it with the question', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(
      screen.getByText(
        "Le projet quitte l'application : ses agents, ses terminaux et ses lancements s'arrêtent. Son historique (conversations, tickets, réglages) est gardé : rouvrir ce dossier le retrouve. Les fichiers et les worktrees sur le disque ne sont pas touchés.",
      ),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      option: { label: 'Effacer aussi son historique (conversations, tickets, réglages)', value: false },
    });
  });

  it('says it in English', () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'Identity' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'The project leaves the app: its agents, terminals and launches stop. Its history (conversations, tickets, settings) is kept: adding this folder again brings it back. The files and worktrees on disk are not touched.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'MCP server' })).not.toBeInTheDocument();
    expect(screen.queryByRole('switch', { name: 'Agents can use Escouade' })).not.toBeInTheDocument();
  });
});

describe('ProjectTab: « Projet privé » (ESC-139)', () => {
  const LABEL = 'Projet privé';
  const DEFAULT = 'Nouveaux projets privés';
  const tab = (name: string) => screen.getByRole('tab', { name });

  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site', private: true })] });
    app.modal = { kind: 'settings' };
  });

  it('is off by default, says what it changes, and is saved with the project', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const own = screen.getByRole('switch', { name: LABEL });
    expect(own).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByRole('heading', { name: 'Confidentialité' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'Les agents des autres projets ne le voient pas par le serveur MCP d’Escouade : ni dans les listes, ni par son nom, ses tickets ou ses agents. Les siens ne voient que lui. Toi, Claude hors Escouade et les vues de l’app voyez tout.',
      ),
    ).toBeVisible();
    await userEvent.click(own);
    expect(tab('Projets')).toHaveClass('changed');
    await save();
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', private: true });
    expect(app.projects[0].private).toBe(true);
    expect(backend.called('save_settings')).toHaveLength(0);
  });

  it('says, harness by harness, whether its files are kept by a guard or a barrier (ESC-140)', () => {
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const list = screen.getByRole('list', { name: 'Ses fichiers' });
    const rows = within(list).getAllByRole('listitem');
    expect(rows).toHaveLength(4);
    const kind = (name: string) => rows.find((r) => r.textContent?.startsWith(name))!.querySelector('.kind')!.textContent;
    expect(kind('Claude Code')).toBe('Garde-fou');
    expect(kind('Codex')).toBe('Barrière');
    expect(kind('OpenCode')).toBe('Garde-fou');
    expect(kind('Copilot CLI')).toBe('Garde-fou');
    // What each cannot guarantee, said.
    expect(within(list).getByText(/sauf en mode Bypass, qui n’en a pas/)).toBeVisible();
    expect(within(list).getByText(/Rien n’est garanti en mode Bypass, ni en vue Copilot CLI/)).toBeVisible();
    expect(within(list).getByText(/pas à un script qui ouvre un fichier lui-même/)).toBeVisible();
    expect(screen.getByText(/Une barrière pour tous les harness demanderait un compte système par projet/)).toBeVisible();
  });

  it('shows a private project as it is, its lock among the projects of the settings', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p2' });
    expect(screen.getByRole('switch', { name: LABEL })).toHaveAttribute('aria-checked', 'true');
    const lock = 'Projet privé : invisible pour les agents des autres projets';
    expect(screen.getByRole('button', { name: /site/ })).toContainElement(screen.getByRole('img', { name: lock }));
    expect(screen.getAllByRole('img', { name: lock })).toHaveLength(1);
    // Put back as it was: nothing to save.
    await userEvent.click(screen.getByRole('switch', { name: LABEL }));
    await userEvent.click(screen.getByRole('switch', { name: LABEL }));
    await save();
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('sets the default of the new projects, off until switched on, also with no project open', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'Nouveaux projets' })).toBeInTheDocument();
    const byDefault = screen.getByRole('switch', { name: DEFAULT });
    expect(byDefault).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText('Chaque projet ajouté est privé dès le départ. Ceux qui existent gardent leur réglage.')).toBeVisible();
    await userEvent.click(byDefault);
    expect(tab('Projets')).toHaveClass('changed');
    await save();
    expect(backend.called('save_settings')[0].args.settings).toMatchObject({ privateProjects: true });
    expect(app.settings.privateProjects).toBe(true);
    // The projects that exist keep their own.
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('offers the default with no project open', () => {
    fakeBackend();
    resetApp({ projects: [] });
    render(SettingsModal, { tab: 'projects' });
    expect(screen.getByRole('switch', { name: DEFAULT })).toBeInTheDocument();
  });

  it('says it in English', () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByRole('switch', { name: 'Private project' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'New projects are private' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Private project: unseen by the agents of the other projects' })).toBeInTheDocument();
  });
});

describe('ProjectTab: a private project’s accounts (ESC-141)', () => {
  const PRIVATE = 'Projet privé';
  const LIMIT = 'Comptes partagés à la limite';
  const pro = { id: 'pro', name: 'Pro', configDir: 'C:/acc/pro', claudePath: '', active: true };
  const principal = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };

  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.settings.accounts = [principal, pro];
    app.modal = { kind: 'settings' };
  });

  it('says nothing of the accounts of a project that is not private', () => {
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.queryByRole('switch', { name: LIMIT })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Ajouter un compte réservé…' })).toBeNull();
  });

  it('made private, says which shared accounts it still goes to and offers to reserve one', async () => {
    const backend = fakeBackend({
      account_update: ({ account }) => [principal, account],
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(screen.getByRole('switch', { name: PRIVATE }));
    expect(
      screen.getByText(
        'Ses agents peuvent partir sur « Principal » et « Pro », des comptes partagés avec d’autres projets : sessions, mémoire, historique et réglages globaux de Claude Code y sont communs.',
      ),
    ).toBeVisible();
    // Pro reserved to it alone, at once: what it shares is now Principal only, at the limit too unless allowed.
    await userEvent.click(screen.getByRole('button', { name: 'Réserver Pro à ce projet' }));
    expect(backend.called('account_update')[0].args.account).toMatchObject({ id: 'pro', projects: ['p1'] });
    expect(
      await screen.findByText(
        'Compte réservé à ce projet : « Pro ». Sessions, mémoire et historique de Claude Code n’y sont partagés avec aucun autre projet.',
      ),
    ).toBeVisible();
    expect(screen.queryByText(/^Ses agents peuvent partir sur/)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Ajouter un compte réservé…' })).toBeNull();
    // Allowed to go on on a shared account at the limit: Principal is shared again, and said so.
    const limit = screen.getByRole('switch', { name: LIMIT });
    expect(limit).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(limit);
    expect(screen.getByText(/Ses agents peuvent partir sur « Principal », un compte partagé/)).toBeVisible();
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', private: true, sharedOnLimit: true });
  });

  it('adds an account reserved to it, back to its settings as drafted', async () => {
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(screen.getByRole('switch', { name: PRIVATE }));
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter un compte réservé…' }));
    expect(app.modal).toEqual({ kind: 'account', engine: 'claude', reserveFor: 'p1' });
  });

  it('says the machine’s account reserved to it is still the harness’s outside Escouade', () => {
    fakeBackend();
    app.projects[0].private = true;
    app.settings.accounts = [
      { ...principal, projects: ['p1'] },
      { ...pro, projects: ['p2'] },
    ];
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(
      screen.getByText(
        'Compte réservé à ce projet : « Principal ». Sessions, mémoire et historique de Claude Code n’y sont partagés avec aucun autre projet. Le compte de la machine reste celui de Claude Code hors d’Escouade.',
      ),
    ).toBeVisible();
  });

  it('says it in English', async () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    await userEvent.click(screen.getByRole('switch', { name: 'Private project' }));
    expect(screen.getByRole('switch', { name: 'Shared accounts at the limit' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add a reserved account…' })).toBeInTheDocument();
    expect(screen.getByText(/Its agents may go to “Main” and “Pro”, accounts shared with other projects/)).toBeVisible();
  });
});

describe('ProjectTab: its default harness (ESC-29)', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.modal = { kind: 'settings' };
  });
  const LABEL = 'Harness par défaut';

  it('is offered only from two harnesses found on', () => {
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.queryByRole('group', { name: LABEL })).toBeNull();
  });

  it('is Claude Code until another is chosen, and saved with the project', async () => {
    app.enginesFound = ['claude', 'codex'];
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const group = screen.getByRole('group', { name: LABEL });
    expect(screen.getByText('Celui des nouveaux agents et des tickets du projet, sauf s’ils en choisissent un autre.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Claude Code', pressed: true })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Harness nu' }));
    expect(group).toBeInTheDocument();
    await save();
    expect(backend.called('update_project')[0].args.project.engine).toBe('codex');
  });
});
