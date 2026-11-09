import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, branchInfo, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

describe('GitPage', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
  });

  it('has no switch of its own for Escouade’s tools: the app’s one gives them to every agent (ESC-91)', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    expect(screen.queryByRole('switch', { name: 'Les agents peuvent utiliser Escouade' })).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Serveur MCP' })).not.toBeInTheDocument();
    // A change of the project saves nothing of the kind.
    await userEvent.click(screen.getByRole('switch', { name: 'Un worktree par agent' }));
    await afterGesture();
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project').at(-1)!.args.project).not.toHaveProperty('agentsUseEscouade');
  });

  it('makes a new agent’s worktree at its first message, under its name, until switched off (ESC-156)', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const toggle = screen.getByRole('switch', { name: 'Créer le worktree au premier message, au nom de l’agent' });
    // On for a project that does not say (saved before it).
    expect(toggle).toBeChecked();
    expect(
      screen.getByText(
        'Le premier message nomme l’agent, puis sa branche et son dossier prennent ce nom avant la préparation. Éteint, le worktree est créé tout de suite, en agent-N.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(toggle);
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', worktreeAtFirstMessage: false });
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
    render(SettingsModal, { page: 'git', projectId: 'p1' });
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
    expect(named).toHaveTextContent('origin/hotfix');
    expect(screen.getByText(/^Ils partent de origin\/hotfix,/)).toBeInTheDocument();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({
      id: 'p1',
      worktreeBase: { kind: 'branch', branch: 'origin/hotfix' },
    });
  });

  it('shows the branch the project starts from, and saves the folder’s again when chosen', async () => {
    resetApp({ projects: [project({ worktreeBase: { kind: 'branch', branch: 'release' } })] });
    app.modal = { kind: 'settings' };
    const backend = fakeBackend();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const group = screen.getByRole('group', { name: 'Branche de départ des nouveaux worktrees' });
    expect(within(group).getByRole('button', { name: /release/ })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(group).getByRole('button', { name: 'Branche actuelle du dossier' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({ worktreeBase: { kind: 'head' } });
  });

  it('cleans up the worktrees archived agents left, and says which it kept and why (ESC-143)', async () => {
    let done: (r: unknown) => void = () => {};
    const backend = fakeBackend({ clean_worktrees: () => new Promise((r) => (done = r)) });
    render(SettingsModal, { page: 'git', projectId: 'p1' });
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
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: 'Nettoyer' }));
    expect(await screen.findByRole('status')).toHaveTextContent('Aucun worktree à supprimer.');
  });

  it('sets where the worktrees go and how their branches are named, and says where the next one goes', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
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
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project).toMatchObject({
      id: 'p1',
      worktreeDir: '../demo-api-worktrees',
      agentBranchPrefix: 'gg/',
      ticketBranchPrefix: '',
    });
    expect(app.projects[0]).toMatchObject({ worktreeDir: '../demo-api-worktrees', agentBranchPrefix: 'gg/', ticketBranchPrefix: '' });
  });

  it('refuses a worktrees folder or a branch prefix that cannot be, and saves nothing until it is fixed', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const dir = screen.getByRole('textbox', { name: 'Dossier des worktrees' });
    await userEvent.type(dir, '../demo-api');
    expect(dir).toHaveAttribute('aria-invalid', 'true');
    expect(dir).toHaveAccessibleDescription(/C’est le dossier du projet lui-même\./);
    // Left as it is: said on its row, not saved.
    await afterGesture();
    expect(backend.called('update_project')).toHaveLength(0);
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
    const tickets = screen.getByRole('textbox', { name: 'Préfixe des branches des tickets' });
    await userEvent.type(tickets, ' x');
    expect(tickets).toHaveAttribute('aria-invalid', 'true');
    expect(tickets).toHaveAccessibleDescription(/Aucun nom de branche ne commence ainsi.*« \\ » ou « @\{ »/);
    await afterGesture();
    expect(backend.called('update_project')).toHaveLength(0);
    expect(app.projects[0].ticketBranchPrefix).toBe('ticket/');
  });
});
