import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { setLang } from '../../lib/i18n';
import { mcp } from '../../lib/mcp.svelte';
import { app } from '../../lib/state.svelte';
import type { Account, McpActivityEntry, McpDeclaration, McpStatus } from '../../lib/types';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import McpSection from './McpSection.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\Users\\ada\\.escouade\\claude\\pro', claudePath: '', active: true };

const TOKEN = 'k3Zr9-Q_tokenOfTheServer0123456789abcdefghi';
const ADD = `mcp add --scope user --transport http escouade http://127.0.0.1:47123/mcp --header "Authorization: Bearer ${TOKEN}"`;

const ok = (account: string): McpDeclaration => ({ account, ok: true, error: null, command: `claude ${ADD}` });
const failed = (account: string, error: string): McpDeclaration => ({
  account,
  ok: false,
  error,
  command: `$old = $env:CLAUDE_CONFIG_DIR; $env:CLAUDE_CONFIG_DIR = 'C:\\Users\\ada\\.escouade\\claude\\pro'; claude ${ADD}; $env:CLAUDE_CONFIG_DIR = $old`,
});
const RUNNING: McpStatus = { running: true, port: 47123, error: null };

const call = (over: Partial<McpActivityEntry> = {}): McpActivityEntry => ({
  at: new Date(2026, 9, 10, 14, 32, 5).getTime(),
  caller: 'Claude (hors Escouade)',
  tool: 'create_ticket',
  summary: 'project: demo, title: Ajouter la page',
  outcome: 'ok',
  message: null,
  ...over,
});

type Handlers = Record<string, (a: any) => unknown>;
const backend = (over: Handlers = {}) =>
  fakeBackend({
    mcp_status: () => ({ running: false, port: 0, error: null }),
    mcp_declare_status: () => [],
    mcp_activity: () => [],
    mcp_set_enabled: () => null,
    mcp_clear_activity: () => null,
    ...over,
  });

const switchEl = () => screen.getByRole('switch', { name: 'Claude peut piloter Escouade' });

describe('McpSection', () => {
  beforeEach(() => {
    resetApp();
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO] };
  });

  it('is the « Escouade dans Claude » group of the « Claude Code » tab, off by default, with its help and its note', async () => {
    backend();
    app.modal = { kind: 'settings', tab: 'claude' };
    render(SettingsModal, { tab: 'claude' });
    expect(screen.getByRole('tab', { name: 'Claude Code' })).toHaveAttribute('aria-selected', 'true');
    expect(await screen.findByRole('heading', { name: 'Escouade dans Claude' })).toBeInTheDocument();
    expect(switchEl()).toHaveAttribute('aria-checked', 'false');
    expect(
      screen.getByText(
        'Claude Code (dans un terminal, un autre outil ou un agent d’Escouade) lit tes projets, agents et tickets, et peut créer des tickets ou lancer des agents.',
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'Le jeton est écrit en clair dans la config de Claude Code (~/.claude.json pour le compte Principal). Désactiver change le jeton.',
      ),
    ).toBeInTheDocument();
    // Off: nothing is said of a declaration.
    expect(screen.queryByText(/Déclaré dans Claude/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Déclaration dans Claude/)).not.toBeInTheDocument();
    // The log has its own group, after.
    expect(screen.getByRole('heading', { name: 'Activité MCP' })).toBeInTheDocument();
  });

  it('turns on at once, not through the modal’s draft, and says what it declares as the backend tells it', async () => {
    const be = backend();
    app.modal = { kind: 'settings', tab: 'claude' };
    render(SettingsModal, { tab: 'claude' });
    await userEvent.click(switchEl());
    expect(be.called('mcp_set_enabled').map((c) => c.args)).toEqual([{ enabled: true }]);
    await waitFor(() => expect(switchEl()).toHaveAttribute('aria-checked', 'true'));
    expect(app.settings.mcpEnabled).toBe(true);
    // Saved already: the tab is not a change left for « Enregistrer ».
    expect(screen.getByRole('tab', { name: 'Claude Code' })).not.toHaveClass('changed');
    // Not told yet.
    expect(screen.getByRole('status')).toHaveTextContent('Déclaration dans Claude…');
    mcp.take({ type: 'mcpStatus', status: RUNNING });
    mcp.take({ type: 'mcpDeclared', declared: [ok('principal'), ok('pro')] });
    expect(await screen.findByText('Déclaré dans Claude · compte Principal, compte Pro')).toBeInTheDocument();
    expect(screen.queryByText(/Déclaration dans Claude…/)).not.toBeInTheDocument();
  });

  it('turns off at once, and no longer says what it declared', async () => {
    app.settings.mcpEnabled = true;
    mcp.declared = [ok('principal')];
    const be = backend({ mcp_declare_status: () => [ok('principal')] });
    render(McpSection);
    expect(await screen.findByText('Déclaré dans Claude · compte Principal')).toBeInTheDocument();
    expect(switchEl()).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(switchEl());
    expect(be.called('mcp_set_enabled').map((c) => c.args)).toEqual([{ enabled: false }]);
    await waitFor(() => expect(switchEl()).toHaveAttribute('aria-checked', 'false'));
    expect(app.settings.mcpEnabled).toBe(false);
    mcp.take({ type: 'mcpDeclared', declared: [] });
    expect(screen.queryByText(/Déclaré dans Claude/)).not.toBeInTheDocument();
  });

  it('leaves the switch where it was, and says why, when the backend refuses', async () => {
    backend({
      mcp_set_enabled: () => {
        throw 'Le jeton du serveur MCP n’a pas pu être gardé';
      },
    });
    render(McpSection);
    await userEvent.click(switchEl());
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Le jeton du serveur MCP n’a pas pu être gardé']));
    expect(switchEl()).toHaveAttribute('aria-checked', 'false');
    expect(app.settings.mcpEnabled).toBe(false);
    // Free to try again.
    expect(switchEl()).toBeEnabled();
  });

  it('tells the account it could not declare it in, why, and the command to run, the token hidden on screen and whole when copied', async () => {
    app.settings.mcpEnabled = true;
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [ok('principal'), failed('pro', 'Failed to write to config')],
    });
    render(McpSection);
    expect(await screen.findByText('Déclaré dans Claude · compte Principal')).toBeInTheDocument();
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('Pas déclaré pour le compte Pro : Failed to write to config');
    // The command to run by hand is shown, but its token is not.
    const shown = within(alert.parentElement!).getByText(/mcp add --scope user --transport http escouade/);
    expect(shown).toHaveTextContent('Bearer ••••••••');
    expect(document.body.textContent).not.toContain(TOKEN);
    expect(shown).toHaveTextContent("$env:CLAUDE_CONFIG_DIR = 'C:\\Users\\ada\\.escouade\\claude\\pro'");
    // Copied whole.
    await userEvent.click(screen.getByRole('button', { name: 'Copier la commande' }));
    expect(writeText).toHaveBeenCalledTimes(1);
    const copied = (writeText.mock.calls as unknown as string[][])[0][0];
    expect(copied).toContain(`Bearer ${TOKEN}`);
    expect(copied).toContain('$env:CLAUDE_CONFIG_DIR');
    expect(app.toasts.map((t) => t.text)).toEqual(['Commande copiée']);
    // The one that went is not offered a command.
    expect(screen.getAllByRole('button', { name: 'Copier la commande' })).toHaveLength(1);
  });

  it('tells each failed account apart, and names Principal as the interface does', async () => {
    app.settings.mcpEnabled = true;
    app.settings.accounts = [{ ...PRINCIPAL, name: 'Main' }, PRO];
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [failed('principal', 'locked'), failed('pro', 'no claude')],
    });
    render(McpSection);
    const alerts = await screen.findAllByRole('alert');
    expect(alerts.map((a) => a.textContent)).toEqual([
      'Pas déclaré pour le compte Principal : locked',
      'Pas déclaré pour le compte Pro : no claude',
    ]);
    expect(screen.queryByText(/Déclaré dans Claude/)).not.toBeInTheDocument();
  });

  it('says why the server itself did not start', async () => {
    app.settings.mcpEnabled = true;
    backend({ mcp_status: () => ({ running: false, port: 0, error: 'Aucun port libre entre 47000 et 47999 pour le serveur MCP' }) });
    render(McpSection);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Le serveur n’a pas démarré : Aucun port libre entre 47000 et 47999 pour le serveur MCP',
    );
    expect(screen.queryByText(/Déclaration dans Claude/)).not.toBeInTheDocument();
  });

  it('is written in English when the interface is', async () => {
    setLang('en');
    app.settings.mcpEnabled = true;
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [ok('principal'), failed('pro', 'locked')],
      mcp_activity: () => [call({ outcome: 'refused', message: 'The autopilot is paused' })],
    });
    render(McpSection);
    expect(await screen.findByText('Declared in Claude · Main account')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('Not declared for the Pro account: locked');
    expect(screen.getByRole('switch', { name: 'Claude can drive Escouade' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy the command' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Escouade in Claude' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'MCP activity' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Clear' })).toBeInTheDocument();
    expect(await screen.findByText('Refused')).toBeInTheDocument();
    expect(
      screen.getByText(/The token is written in plain text in Claude Code’s config \(~\/\.claude\.json for the Main account\)/),
    ).toBeInTheDocument();
  });

  describe('the activity', () => {
    const items = () => screen.getAllByRole('listitem');

    it('has nothing to clear while empty', async () => {
      backend();
      render(McpSection);
      expect(await screen.findByText('Aucun appel pour l’instant.')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Effacer' })).toBeDisabled();
    });

    it('lists the calls, the most recent first, with their time, caller, tool, summary and result', async () => {
      backend({
        mcp_activity: () => [
          call({ at: new Date(2026, 9, 10, 14, 30).getTime(), tool: 'list_projects', summary: '', outcome: 'ok' }),
          call({
            at: new Date(2026, 9, 10, 14, 31).getTime(),
            caller: 'refacto-auth',
            tool: 'start_ticket',
            summary: 'ticket: DEM-2',
            outcome: 'refused',
            message: 'Le pilote auto est en pause jusqu’à 15:00',
          }),
          call({
            at: new Date(2026, 9, 10, 14, 32).getTime(),
            caller: 'Client inconnu',
            tool: '',
            summary: 'POST /mcp',
            outcome: 'refused',
            message: 'Requête refusée : jeton inconnu',
          }),
          call({ at: new Date(2026, 9, 10, 14, 33).getTime(), tool: 'create_ticket', outcome: 'error', message: 'Le titre est vide' }),
        ],
      });
      render(McpSection);
      const log = await screen.findByRole('list', { name: 'Appels au serveur MCP, les plus récents en tête' });
      await waitFor(() => expect(within(log).getAllByRole('listitem')).toHaveLength(4));
      const rows = within(log).getAllByRole('listitem');
      expect(rows.map((r) => r.querySelector('.tool')?.textContent)).toEqual(['create_ticket', '—', 'start_ticket', 'list_projects']);
      const first = rows[0];
      expect(first).toHaveTextContent('14:33');
      expect(first).toHaveTextContent('Claude (hors Escouade)');
      expect(first).toHaveTextContent('project: demo, title: Ajouter la page');
      expect(first).toHaveTextContent('Erreur');
      expect(first).toHaveTextContent('Le titre est vide');
      // A refusal says by whom, on what and why; in the colour of an alert.
      expect(rows[2]).toHaveTextContent('14:31');
      expect(rows[2]).toHaveTextContent('refacto-auth');
      expect(rows[2]).toHaveTextContent('ticket: DEM-2');
      expect(rows[2]).toHaveTextContent('Refusé');
      expect(rows[2]).toHaveTextContent('Le pilote auto est en pause jusqu’à 15:00');
      expect(rows[1]).toHaveTextContent('Client inconnu');
      expect(rows[1]).toHaveTextContent('Requête refusée : jeton inconnu');
      expect(rows[0]).toHaveClass('bad');
      expect(rows[1]).toHaveClass('bad');
      expect(rows[2]).toHaveClass('bad');
      expect(rows[3]).not.toHaveClass('bad');
      expect(rows[3]).toHaveTextContent('Fait');
      expect(screen.getByRole('button', { name: 'Effacer' })).toBeEnabled();
    });

    it('shows a call as it comes, above the others', async () => {
      backend({ mcp_activity: () => [call({ tool: 'list_projects', summary: '' })] });
      render(McpSection);
      await waitFor(() => expect(items()).toHaveLength(1));
      mcp.take({
        type: 'mcpActivity',
        entry: call({ at: new Date(2026, 9, 10, 14, 40).getTime(), tool: 'get_ticket', summary: 'ticket: DEM-1' }),
      });
      await waitFor(() => expect(items()).toHaveLength(2));
      expect(items().map((r) => r.querySelector('.tool')?.textContent)).toEqual(['get_ticket', 'list_projects']);
    });

    it('clears the log, in the window and in the backend', async () => {
      const be = backend({ mcp_activity: () => [call(), call({ tool: 'get_ticket' })] });
      render(McpSection);
      await waitFor(() => expect(items()).toHaveLength(2));
      await userEvent.click(screen.getByRole('button', { name: 'Effacer' }));
      expect(be.called('mcp_clear_activity')).toHaveLength(1);
      expect(await screen.findByText('Aucun appel pour l’instant.')).toBeInTheDocument();
      expect(screen.queryAllByRole('listitem')).toHaveLength(0);
    });

    it('keeps the calls that came before it was opened', async () => {
      // The window was told of them from its start; the settings open long after.
      mcp.take({ type: 'mcpActivity', entry: call({ tool: 'get_ticket' }) });
      backend({
        mcp_activity: () => [call({ tool: 'get_ticket' }), call({ at: new Date(2026, 9, 10, 14, 35).getTime(), tool: 'list_agents' })],
      });
      render(McpSection);
      await waitFor(() => expect(items()).toHaveLength(2));
      expect(items().map((r) => r.querySelector('.tool')?.textContent)).toEqual(['list_agents', 'get_ticket']);
    });
  });
});
