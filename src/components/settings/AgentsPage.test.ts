import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { setLang } from '../../lib/i18n';
import { mcp } from '../../lib/mcp.svelte';
import { app } from '../../lib/state.svelte';
import type { Account, EngineDecl, McpActivityEntry, McpDeclaration, McpStatus } from '../../lib/types';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp, SETTINGS, usableEngines } from '../../test/ipc';
import { afterGesture } from '../../test/settings';
import SettingsModal from '../modals/SettingsModal.svelte';
import McpActivity from './McpActivity.svelte';
import McpSection from './McpSection.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\Users\\ada\\.escouade\\claude\\pro', claudePath: '', active: true };

const TOKEN = 'k3Zr9-Q_tokenOfTheServer0123456789abcdefghi';
const ADD = (token: string) =>
  `mcp add --scope user --transport http escouade http://127.0.0.1:47123/mcp --header "Authorization: Bearer ${token}"`;
const WITH_FOLDER = (add: string) =>
  `$old = $env:CLAUDE_CONFIG_DIR; $env:CLAUDE_CONFIG_DIR = 'C:\\Users\\ada\\.escouade\\claude\\pro'; claude ${add}; $env:CLAUDE_CONFIG_DIR = $old`;

// As the backend sends them: the commands have their token hidden. The real one is asked for when it is copied.
const ok = (account: string): McpDeclaration => ({
  account,
  engine: 'claude',
  ok: true,
  error: null,
  command: `claude ${ADD('••••••••')}`,
  file: 'C:\\Users\\ada\\.claude.json',
  paste: false,
});
const failed = (account: string, error: string): McpDeclaration => ({
  account,
  engine: 'claude',
  ok: false,
  error,
  command: WITH_FOLDER(ADD('••••••••')),
  file: 'C:\\Users\\ada\\.escouade\\claude\\pro\\.claude.json',
  paste: false,
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

const switchEl = () => screen.getByRole('switch', { name: 'Les agents peuvent piloter Escouade' });
/** The declarations listed under the switch, each as it reads. */
const rows = () => within(screen.getByRole('list', { name: 'Déclarations dans les harness' })).getAllByRole('listitem');

describe('Agents › « Piloter Escouade » (McpSection)', () => {
  beforeEach(() => {
    resetApp();
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO] };
  });

  it('is the group « Piloter Escouade » of the page Agents (from the 2.0 tab « Serveur MCP »), off by default, with its help and its note', async () => {
    backend();
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    expect(within(screen.getByRole('navigation')).getByRole('button', { name: 'Agents' })).toHaveAttribute('aria-current', 'page');
    expect(await screen.findByRole('heading', { name: 'Piloter Escouade' })).toBeInTheDocument();
    expect(switchEl()).toHaveAttribute('aria-checked', 'false');
    // One switch: none of a harness of its own (ESC-119).
    expect(screen.getAllByRole('switch').filter((s) => /piloter Escouade/.test(s.getAttribute('aria-label') ?? ''))).toHaveLength(1);
    expect(
      screen.getByText(
        'Les agents d’Escouade, et chaque harness installé lancé hors d’Escouade (Claude Code, Codex, OpenCode, Copilot CLI, dans un terminal ou leur app), lisent tes projets, agents et tickets, et peuvent créer des tickets ou lancer des agents, ce qui dépense du quota, valider des tickets (tests, fusion, push, pull request) ou en supprimer. Un agent déjà lancé reçoit les outils au prochain démarrage de son process.',
      ),
    ).toBeInTheDocument();
    // Which harnesses and files get the entry, and the token in plain text in each.
    expect(
      screen.getByText(
        'Escouade déclare son serveur dans la config de chaque compte actif de chaque harness installé : ~/.claude.json (Claude Code, par claude mcp add), ~/.codex/config.toml (Codex), ~/.config/opencode/opencode.json (OpenCode), ~/.copilot/mcp-config.json (Copilot CLI) pour le compte Principal, le reste du fichier et ses commentaires gardés. Le jeton y est écrit en clair. Désactiver change le jeton, puis retire l’entrée partout. Un harness installé ou un compte ajouté plus tard est déclaré aussitôt.',
      ),
    ).toBeInTheDocument();
    // Off: nothing is said of a declaration.
    expect(screen.queryByRole('list', { name: 'Déclarations dans les harness' })).not.toBeInTheDocument();
    expect(screen.queryByText(/Déclaration dans/)).not.toBeInTheDocument();
    // The log is in the page's « Avancé », folded, named there; unfolded, under its title.
    expect(screen.queryByRole('heading', { name: 'Activité MCP' })).toBeNull();
    await userEvent.click(within(document.getElementById('settings-panel')!).getByRole('button', { name: /^Avancé/ }));
    expect(screen.getByRole('heading', { name: 'Activité MCP' })).toBeInTheDocument();
  });

  it('turns on at once, not through the modal’s draft, and says what it declares as the backend tells it', async () => {
    const be = backend();
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    await userEvent.click(switchEl());
    expect(be.called('mcp_set_enabled').map((c) => c.args)).toEqual([{ enabled: true }]);
    await waitFor(() => expect(switchEl()).toHaveAttribute('aria-checked', 'true'));
    expect(app.settings.mcpEnabled).toBe(true);
    // Saved already: the tab is not a change left for « Enregistrer ».
    // Not told yet.
    expect(screen.getByRole('status')).toHaveTextContent('Déclaration dans les harness installés…');
    mcp.take({ type: 'mcpStatus', status: RUNNING });
    mcp.take({ type: 'mcpDeclared', declared: [ok('principal'), ok('pro')] });
    await waitFor(() => expect(rows()).toHaveLength(2));
    expect(rows().map((r) => r.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'Claude · compte Principal Déclaré C:\\Users\\ada\\.claude.json',
      'Claude · compte Pro Déclaré C:\\Users\\ada\\.claude.json',
    ]);
    expect(screen.queryByText(/Déclaration dans/)).not.toBeInTheDocument();
  });

  it('says how many agents already running get the tools at their next start, and where to restart them', async () => {
    app.settings.mcpEnabled = true;
    mcp.declared = [ok('principal')];
    backend({ mcp_status: () => RUNNING, mcp_declare_status: () => [ok('principal')] });
    app.agents = { a1: agent({ id: 'a1', escouadeNext: true }), a2: agent({ id: 'a2', escouadeNext: false }) };
    render(McpSection);
    expect(
      await screen.findByText(
        '1 agent déjà lancé aura les outils au prochain démarrage de son process : « Relancer » sur sa conversation.',
      ),
    ).toBeInTheDocument();
    app.agents.a2 = agent({ id: 'a2', escouadeNext: true });
    expect(
      await screen.findByText(
        '2 agents déjà lancés auront les outils au prochain démarrage de leur process : « Relancer » sur leur conversation.',
      ),
    ).toBeInTheDocument();
    // All restarted: nothing left to say.
    app.agents = {};
    await waitFor(() => expect(screen.queryByText(/« Relancer » sur/)).not.toBeInTheDocument());
    setLang('en');
    app.agents = { a1: agent({ id: 'a1', escouadeNext: true }), a2: agent({ id: 'a2', escouadeNext: true }) };
    expect(
      await screen.findByText(
        '2 agents already running get the tools the next time their process starts: “Restart” on their conversation.',
      ),
    ).toBeInTheDocument();
    setLang('fr');
  });

  it('turns off at once, and no longer says what it declared', async () => {
    app.settings.mcpEnabled = true;
    mcp.declared = [ok('principal')];
    const be = backend({ mcp_declare_status: () => [ok('principal')] });
    render(McpSection);
    expect(await screen.findByText('Claude · compte Principal')).toBeInTheDocument();
    expect(switchEl()).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(switchEl());
    expect(be.called('mcp_set_enabled').map((c) => c.args)).toEqual([{ enabled: false }]);
    await waitFor(() => expect(switchEl()).toHaveAttribute('aria-checked', 'false'));
    expect(app.settings.mcpEnabled).toBe(false);
    mcp.take({ type: 'mcpDeclared', declared: [] });
    expect(screen.queryByText('Claude · compte Principal')).not.toBeInTheDocument();
  });

  it('shows the switch busy while the change is saved', async () => {
    let done: (v: null) => void = () => {};
    backend({ mcp_set_enabled: () => new Promise((r) => (done = r)) });
    render(McpSection);
    await userEvent.click(switchEl());
    expect(switchEl()).toBeDisabled();
    done(null);
    await waitFor(() => expect(switchEl()).toBeEnabled());
    expect(switchEl()).toHaveAttribute('aria-checked', 'true');
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
    const be = backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [ok('principal'), failed('pro', 'Failed to write to config')],
      // Asked for when it is copied: the real command, the real token.
      mcp_manual_command: () => WITH_FOLDER(ADD(TOKEN)),
    });
    render(McpSection);
    expect(await screen.findByText('Claude · compte Principal')).toBeInTheDocument();
    expect(rows()[0]).toHaveTextContent('Déclaré');
    expect(rows()[1]).toHaveTextContent('Pas déclaré');
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('Pas déclaré pour le compte Pro : Failed to write to config');
    // The command to run by hand is shown, but its token is not.
    const shown = within(alert.parentElement!).getByText(/mcp add --scope user --transport http escouade/);
    expect(shown).toHaveTextContent('Bearer ••••••••');
    expect(document.body.textContent).not.toContain(TOKEN);
    expect(shown).toHaveTextContent("$env:CLAUDE_CONFIG_DIR = 'C:\\Users\\ada\\.escouade\\claude\\pro'");
    // The shell it is written for is said.
    expect(within(alert.parentElement!).getByText('À lancer à la main dans PowerShell :')).toBeInTheDocument();
    // The real command is not in the window until it is asked for.
    expect(be.called('mcp_manual_command')).toHaveLength(0);
    // Copied whole.
    await userEvent.click(screen.getByRole('button', { name: 'Copier la commande' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1));
    expect(be.called('mcp_manual_command').map((c) => c.args)).toEqual([{ account: 'pro' }]);
    const copied = (writeText.mock.calls as unknown as string[][])[0][0];
    expect(copied).toContain(`Bearer ${TOKEN}`);
    expect(copied).toContain('$env:CLAUDE_CONFIG_DIR');
    expect(app.toasts.map((t) => t.text)).toEqual(['Commande copiée']);
    // The one that went is not offered a command.
    expect(screen.getAllByRole('button', { name: 'Copier la commande' })).toHaveLength(1);
    // Still not on the screen.
    expect(document.body.textContent).not.toContain(TOKEN);
  });

  it('copies nothing, and says why, when the backend cannot give the command', async () => {
    app.settings.mcpEnabled = true;
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [failed('pro', 'locked')],
      mcp_manual_command: () => {
        throw 'Le serveur MCP ne tourne pas';
      },
    });
    render(McpSection);
    await userEvent.click(await screen.findByRole('button', { name: 'Copier la commande' }));
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Le serveur MCP ne tourne pas']));
    expect(writeText).not.toHaveBeenCalled();
  });

  it('hides a token in what Claude Code said, whatever the backend sent', async () => {
    app.settings.mcpEnabled = true;
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [failed('pro', `Invalid header Authorization: Bearer ${TOKEN}`)],
    });
    render(McpSection);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Pas déclaré pour le compte Pro : Invalid header Authorization: Bearer ••••••••',
    );
    expect(document.body.textContent).not.toContain(TOKEN);
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
    expect(screen.queryByText('Déclaré')).not.toBeInTheDocument();
  });

  it('says why the server itself did not start', async () => {
    app.settings.mcpEnabled = true;
    backend({ mcp_status: () => ({ running: false, port: 0, error: 'Aucun port libre entre 47000 et 47999 pour le serveur MCP' }) });
    render(McpSection);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Le serveur n’a pas démarré : Aucun port libre entre 47000 et 47999 pour le serveur MCP',
    );
    expect(screen.queryByText(/Déclaration dans/)).not.toBeInTheDocument();
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
    render(McpActivity);
    expect(await screen.findByText('Claude · Main account')).toBeInTheDocument();
    expect(screen.getByText('Declared')).toBeInTheDocument();
    expect(screen.getByText('Not declared')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Declarations in the harnesses' })).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('Not declared for the Pro account: locked');
    expect(screen.getByRole('switch', { name: 'Agents can drive Escouade' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'Escouade’s agents, and every harness installed started outside Escouade (Claude Code, Codex, OpenCode, Copilot CLI, in a terminal or their app), read your projects, agents and tickets, and can create tickets or start agents, which uses quota, approve tickets (tests, merge, push, pull request) or delete them. An agent already running gets the tools the next time its process starts.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy the command' })).toBeInTheDocument();
    expect(screen.getByText('Run it by hand in PowerShell:')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Drive Escouade' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'MCP activity' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Clear' })).toBeInTheDocument();
    expect(await screen.findByText('Refused')).toBeInTheDocument();
    expect(
      screen.getByText(
        /~\/\.copilot\/mcp-config\.json \(Copilot CLI\) for the Main account, the rest of the file and its comments kept\. The token is written there in plain text\./,
      ),
    ).toBeInTheDocument();
  });

  describe('the activity', () => {
    const items = () => screen.getAllByRole('listitem');

    it('has nothing to clear while empty', async () => {
      backend();
      render(McpActivity);
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
      render(McpActivity);
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
      // A log that scrolls is reached with the keyboard.
      expect(log).toHaveAttribute('tabindex', '0');
      expect(screen.getByRole('button', { name: 'Effacer' })).toBeEnabled();
    });

    it('shows a call as it comes, above the others', async () => {
      backend({ mcp_activity: () => [call({ tool: 'list_projects', summary: '' })] });
      render(McpActivity);
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
      render(McpActivity);
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
      render(McpActivity);
      await waitFor(() => expect(items()).toHaveLength(2));
      expect(items().map((r) => r.querySelector('.tool')?.textContent)).toEqual(['list_agents', 'get_ticket']);
    });
  });
});

describe('McpSection — every harness installed (ESC-119)', () => {
  const CODEX: EngineDecl = { ...BARE_ENGINE, id: 'codex', name: 'Codex', speaker: 'Codex' };
  const CODEX_PRO: Account = { ...PRO, id: 'codex-pro', name: 'Perso', engine: 'codex' };
  const entry = (token: string) =>
    `[mcp_servers.escouade]\nurl = "http://127.0.0.1:47123/mcp"\nhttp_headers = { Authorization = "Bearer ${token}" }`;
  const json = (token: string) =>
    `{\n  "mcpServers": {\n    "escouade": {\n      "type": "http",\n      "url": "http://127.0.0.1:47123/mcp",\n      "headers": { "Authorization": "Bearer ${token}" }\n    }\n  }\n}`;
  const of = (engine: McpDeclaration['engine'], account: string, file: string, command: string): McpDeclaration => ({
    account,
    engine,
    ok: true,
    error: null,
    command,
    file,
    paste: true,
  });

  beforeEach(() => {
    resetApp();
    app.engines = [
      app.engines[0],
      CODEX,
      { ...BARE_ENGINE, id: 'copilot', name: 'Copilot CLI', speaker: 'Copilot' },
      { ...BARE_ENGINE, id: 'opencode', name: 'OpenCode', speaker: 'OpenCode' },
    ];
    usableEngines('claude', 'codex', 'copilot', 'opencode');
    app.settings = { ...SETTINGS, accounts: [PRINCIPAL, PRO, CODEX_PRO], mcpEnabled: true };
  });

  it('lists each declaration, harness and account, with its file and whether it is there', async () => {
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [
        ok('principal'),
        of('codex', 'principal-codex', 'C:\\Users\\ada\\.codex\\config.toml', entry('••••••••')),
        of('codex', 'codex-pro', 'C:\\Users\\ada\\.escouade\\codex\\perso\\config.toml', entry('••••••••')),
        of('opencode', 'principal-opencode', 'C:\\Users\\ada\\.config\\opencode\\opencode.json', '{}'),
        of('copilot', 'principal-copilot', 'C:\\Users\\ada\\.copilot\\mcp-config.json', json('••••••••')),
      ],
    });
    render(McpSection);
    await waitFor(() => expect(rows()).toHaveLength(5));
    // The Principal of a harness not saved yet is named as Principal.
    expect(rows().map((r) => [r.querySelector('.where')?.textContent, r.querySelector('.file')?.textContent])).toEqual([
      ['Claude · compte Principal', 'C:\\Users\\ada\\.claude.json'],
      ['Codex · compte Principal', 'C:\\Users\\ada\\.codex\\config.toml'],
      ['Codex · compte Perso', 'C:\\Users\\ada\\.escouade\\codex\\perso\\config.toml'],
      ['OpenCode · compte Principal', 'C:\\Users\\ada\\.config\\opencode\\opencode.json'],
      ['Copilot · compte Principal', 'C:\\Users\\ada\\.copilot\\mcp-config.json'],
    ]);
    expect(screen.getAllByText('Déclaré')).toHaveLength(5);
    // Nothing to paste where it is declared.
    expect(screen.queryByRole('button', { name: /Copier/ })).not.toBeInTheDocument();
  });

  it('gives the entry to paste in the file of an account it failed in, its token hidden, the real one copied', async () => {
    const failed: McpDeclaration = {
      ...of('copilot', 'principal-copilot', 'C:\\Users\\ada\\.copilot\\mcp-config.json', json('••••••••')),
      ok: false,
      error: 'C:\\Users\\ada\\.copilot\\mcp-config.json n’est pas un JSON valide, il n’a pas été modifié',
    };
    const writeText = vi.fn(async (_: string) => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const be = backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [ok('principal'), failed],
      mcp_manual_command: () => json(TOKEN),
    });
    render(McpSection);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Pas déclaré pour le compte Principal : C:\\Users\\ada\\.copilot\\mcp-config.json n’est pas un JSON valide',
    );
    expect(screen.getByText('À ajouter à la main dans C:\\Users\\ada\\.copilot\\mcp-config.json :')).toBeInTheDocument();
    expect(document.querySelector('.cmd')?.textContent).toBe(json('••••••••'));
    expect(document.body.textContent).not.toContain(TOKEN);
    await userEvent.click(screen.getByRole('button', { name: 'Copier l’entrée' }));
    expect(be.called('mcp_manual_command').map((c) => c.args)).toEqual([{ account: 'principal-copilot' }]);
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(json(TOKEN)));
    expect(app.toasts.map((t) => t.text)).toEqual(['Entrée copiée']);
  });

  it('is written in English when the interface is', async () => {
    setLang('en');
    backend({
      mcp_status: () => RUNNING,
      mcp_declare_status: () => [ok('principal'), of('codex', 'codex-pro', 'C:\\x\\config.toml', entry('••••••••'))],
    });
    render(McpSection);
    expect(await screen.findByText('Codex · Perso account')).toBeInTheDocument();
    expect(screen.getAllByText('Declared')).toHaveLength(2);
  });
});

const IDLE = 'Arrêter les processus des agents inactifs après';
const IDLE_FIELD = 'Arrêter les processus des agents inactifs après (minutes)';
const REMOTE = 'Remote control pour tous les agents';
const panel = () => document.getElementById('settings-panel')!;
/** The choices of a setting, as its chips read. */
const choices = (name: string) =>
  within(within(panel()).getByRole('group', { name }))
    .getAllByRole('button')
    .map((b) => b.textContent?.trim());
const choice = (setting: string, name: string) =>
  within(within(panel()).getByRole('group', { name: setting })).getByRole('button', { name });

describe('the page « Agents »', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', page: 'agents' };
  });

  it('has its groups in the order of the study, the rare folded into « Avancé »', async () => {
    backend();
    render(SettingsModal, { page: 'agents' });
    expect(screen.getByText('Les nouveaux agents, les textes rédigés, le remote control et le pilotage d’Escouade')).toBeInTheDocument();
    expect(
      within(panel())
        .getAllByRole('heading')
        .map((h) => h.textContent),
    ).toEqual(['Nouveaux agents', 'Textes rédigés', 'Remote control', 'Piloter Escouade']);
    const fold = within(panel()).getByRole('button', { name: /^Avancé/ });
    expect(fold).toHaveTextContent(IDLE + ' · Activité MCP');
    expect(within(panel()).queryByText(IDLE)).toBeNull();
    await userEvent.click(fold);
    expect(within(panel()).getByText(IDLE)).toBeInTheDocument();
    expect(within(panel()).getByRole('heading', { name: 'Activité MCP' })).toBeInTheDocument();
  });

  it('says the remote control of every agent is its harness’s alone, and hides it with no harness that has one', () => {
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    backend();
    const view = render(SettingsModal, { page: 'agents' });
    expect(within(panel()).getByRole('switch', { name: REMOTE })).toBeInTheDocument();
    expect(panel()).toHaveTextContent('Claude Code seulement : les autres harness n’ont pas de remote control.');
    view.unmount();
    app.engines = [BARE_ENGINE];
    render(SettingsModal, { page: 'agents' });
    expect(within(panel()).queryByRole('heading', { name: 'Remote control' })).toBeNull();
  });

  it('saves the stop of the idle processes, from « Avancé »', async () => {
    const be = backend({ save_settings: () => [] });
    render(SettingsModal, { page: 'agents' });
    await userEvent.click(within(panel()).getByRole('button', { name: /^Avancé/ }));
    const field = within(panel()).getByRole('spinbutton', { name: IDLE_FIELD });
    await userEvent.clear(field);
    await userEvent.type(field, '45');
    await afterGesture();
    expect(be.called('save_settings').at(-1)!.args.settings).toMatchObject({ idleStopMinutes: 45 });
  });

  describe('« Modèle des textes rédigés »', () => {
    const WRITER = 'Modèle des textes rédigés';

    it('offers Claude Haiku alone with Claude Code alone, chosen by default', () => {
      fakeBackend();
      render(SettingsModal, { page: 'agents' });
      expect(within(panel()).getByRole('heading', { name: 'Textes rédigés' })).toBeInTheDocument();
      expect(choices(WRITER)).toEqual(['Claude Haiku']);
      expect(choice(WRITER, 'Claude Haiku')).toHaveAttribute('aria-pressed', 'true');
      expect(panel()).toHaveTextContent(
        'Seuls les harness utilisables sont proposés : programme installé, plugin actif et au moins un compte.',
      );
    });

    it('lists only the harnesses installed, and saves the one chosen', async () => {
      app.engines = [CLAUDE_ENGINE, BARE_ENGINE, { ...BARE_ENGINE, id: 'cursor', name: 'Absent', writer: 'Absent mini' }];
      usableEngines('claude', 'codex');
      const backend = fakeBackend({ save_settings: () => [] });
      render(SettingsModal, { page: 'agents' });
      expect(choices(WRITER)).toEqual(['Claude Haiku', 'Nu mini']);
      await userEvent.click(choice(WRITER, 'Nu mini'));
      expect(choice(WRITER, 'Nu mini')).toHaveAttribute('aria-pressed', 'true');
      await afterGesture();
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ writerEngine: 'codex' });
    });

    it('leaves out a harness whose plugin is off or that has no account, and goes to another (ESC-391)', () => {
      app.engines = [CLAUDE_ENGINE, BARE_ENGINE, { ...BARE_ENGINE, id: 'cursor', name: 'Sans compte', writer: 'Sans compte mini' }];
      usableEngines('claude', 'codex');
      app.enginesFound = ['claude', 'codex', 'cursor'];
      app.enginesOff = ['codex'];
      app.settings = { ...app.settings, writerEngine: 'codex' };
      fakeBackend();
      render(SettingsModal, { page: 'agents' });
      expect(choices(WRITER)).toEqual(['Claude Haiku']);
      expect(choice(WRITER, 'Claude Haiku')).toHaveAttribute('aria-pressed', 'true');
    });

    it('shows the harness the texts go to when the one chosen is not installed any more', () => {
      app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
      usableEngines('codex');
      app.settings = { ...app.settings, writerEngine: 'claude' };
      fakeBackend();
      render(SettingsModal, { page: 'agents' });
      expect(choices(WRITER)).toEqual(['Nu mini']);
      expect(choice(WRITER, 'Nu mini')).toHaveAttribute('aria-pressed', 'true');
    });

    it('chooses the model of a harness that takes the one given, among those of its account (ESC-45)', async () => {
      const OPENCODE = { ...BARE_ENGINE, id: 'opencode' as const, name: 'OpenCode', writer: 'OpenCode', writerTakesModel: true };
      app.engines = [CLAUDE_ENGINE, BARE_ENGINE, OPENCODE];
      usableEngines('claude', 'codex', 'opencode');
      app.engineModels = {
        // Codex picks its own: no menu for it.
        codex: [{ id: 'gpt-x', label: 'GPT-X', efforts: [], defaultEffort: null, images: false, default: true }],
        opencode: [
          { id: 'ollama/qwen', label: 'Qwen', efforts: [], defaultEffort: null, images: false, default: true, provider: 'Ollama (local)' },
          { id: 'paid/coder', label: 'Coder', efforts: [], defaultEffort: null, images: false, default: false, provider: 'Paid' },
        ],
      };
      const backend = fakeBackend({ save_settings: () => [] });
      render(SettingsModal, { page: 'agents' });
      expect(within(panel()).queryByRole('combobox', { name: 'Modèle' })).toBeNull();
      await userEvent.click(choice(WRITER, 'Nu mini'));
      expect(within(panel()).queryByRole('combobox', { name: 'Modèle' })).toBeNull();
      await userEvent.click(choice(WRITER, 'OpenCode'));
      const menu = within(panel()).getByRole('combobox', { name: 'Modèle' });
      expect([...menu.querySelectorAll('option')].map((o) => o.textContent)).toEqual([
        'Par défaut (Qwen)',
        'Qwen · Ollama (local)',
        'Coder · Paid',
      ]);
      expect(menu).toHaveValue('');
      expect(panel()).toHaveTextContent('sans lui, OpenCode en choisirait un lui-même');
      await userEvent.selectOptions(menu, 'paid/coder');
      await afterGesture();
      expect(backend.called('save_settings').at(-1)!.args.settings).toMatchObject({ writerEngine: 'opencode', writerModel: 'paid/coder' });
    });

    it('is not shown when no harness is installed', () => {
      app.enginesFound = [];
      fakeBackend();
      render(SettingsModal, { page: 'agents' });
      expect(within(panel()).queryByRole('group', { name: WRITER })).toBeNull();
    });

    it('reads in English', () => {
      fakeBackend();
      setLang('en');
      render(SettingsModal, { page: 'agents' });
      expect(within(panel()).getByRole('heading', { name: 'Written texts' })).toBeInTheDocument();
      expect(choices('Model for written texts')).toEqual(['Claude Haiku']);
      expect(panel()).toHaveTextContent(
        'Only the harnesses that can be used are offered: program installed, plugin on and at least one account.',
      );
    });
  });
});
