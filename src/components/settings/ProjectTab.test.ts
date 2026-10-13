import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const save = () => userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));

describe('ProjectTab', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site', agentsUseEscouade: true })] });
    app.modal = { kind: 'settings' };
  });

  it('lets the project’s agents use Escouade, off by default, and says what it means', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const allowed = screen.getByRole('switch', { name: 'Les agents peuvent utiliser Escouade' });
    expect(allowed).toHaveAttribute('aria-checked', 'false');
    expect(
      screen.getByText(
        'Ils lisent les tickets et les agents, et peuvent créer des tickets ou lancer d’autres agents, ce qui dépense du quota. S’applique au prochain démarrage de leur process.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(allowed);
    expect(allowed).toHaveAttribute('aria-checked', 'true');
    await save();
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', agentsUseEscouade: true });
    expect(app.projects[0].agentsUseEscouade).toBe(true);
  });

  it('shows the project’s own setting, and turns it off', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p2' });
    const allowed = screen.getByRole('switch', { name: 'Les agents peuvent utiliser Escouade' });
    expect(allowed).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(allowed);
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p2', agentsUseEscouade: false });
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

  describe('« Branche de départ des nouveaux agents »', () => {
    const branches = () => screen.getByRole('group', { name: 'Branche de départ des nouveaux agents' });
    const chip = (name: string) => within(branches()).getByRole('button', { name });

    it('is the branch the folder is on until one is chosen, which may then be fetched first', async () => {
      const backend = fakeBackend({ git_branches: () => ['feat/login', 'main', 'release'] });
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      expect(await within(branches()).findByRole('button', { name: '⎇ main' })).toHaveAttribute('aria-pressed', 'false');
      expect(chip('Branche courante')).toHaveAttribute('aria-pressed', 'true');
      // Fetching is of a branch chosen: the folder's own starts from the folder's commit.
      expect(screen.queryByRole('switch', { name: 'Récupérer avant de créer' })).toBeNull();
      await userEvent.click(chip('⎇ main'));
      expect(chip('⎇ main')).toHaveAttribute('aria-pressed', 'true');
      expect(chip('Branche courante')).toHaveAttribute('aria-pressed', 'false');
      const fetch = screen.getByRole('switch', { name: 'Récupérer avant de créer' });
      expect(fetch).toHaveAttribute('aria-checked', 'false');
      expect(screen.getByText(/^git fetch de main avant chaque nouvel agent/)).toBeInTheDocument();
      await userEvent.click(fetch);
      await save();
      expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', agentBaseBranch: 'main', agentBaseFetch: true });
      expect(app.projects[0].agentBaseBranch).toBe('main');
    });

    it('goes back to the branch the folder is on', async () => {
      resetApp({ projects: [project({ agentBaseBranch: 'release', agentBaseFetch: true })] });
      app.modal = { kind: 'settings' };
      const backend = fakeBackend({ git_branches: () => ['main', 'release'] });
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      // Shown as chosen before the branches are read.
      expect(chip('⎇ release')).toHaveAttribute('aria-pressed', 'true');
      expect(screen.getByRole('switch', { name: 'Récupérer avant de créer' })).toHaveAttribute('aria-checked', 'true');
      await userEvent.click(chip('Branche courante'));
      expect(screen.queryByRole('switch', { name: 'Récupérer avant de créer' })).toBeNull();
      await save();
      expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', agentBaseBranch: '' });
    });

    it('keeps among the choices a branch that is gone, which the next agent will tell', async () => {
      resetApp({ projects: [project({ agentBaseBranch: 'gone' })] });
      app.modal = { kind: 'settings' };
      fakeBackend({ git_branches: () => ['main'] });
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      await within(branches()).findByRole('button', { name: '⎇ main' });
      expect(
        within(branches())
          .getAllByRole('button')
          .map((b) => b.textContent),
      ).toEqual(['Branche courante', '⎇ gone', '⎇ main']);
      expect(chip('⎇ gone')).toHaveAttribute('aria-pressed', 'true');
    });

    it('is written in English', async () => {
      setLang('en');
      fakeBackend({ git_branches: () => ['main'] });
      render(SettingsModal, { tab: 'projects', projectId: 'p1' });
      const group = screen.getByRole('group', { name: 'New agents’ starting branch' });
      expect(within(group).getByRole('button', { name: 'Current branch' })).toHaveAttribute('aria-pressed', 'true');
      await userEvent.click(await within(group).findByRole('button', { name: '⎇ main' }));
      expect(screen.getByRole('switch', { name: 'Fetch before creating' })).toBeInTheDocument();
      expect(screen.getByText(/^Runs git fetch on main before each new agent/)).toBeInTheDocument();
    });
  });

  it('says it in English', () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'MCP server' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Agents can use Escouade' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'They read the tickets and the agents, and can create tickets or start other agents, which uses quota. Applies the next time their process starts.',
      ),
    ).toBeInTheDocument();
  });
});
