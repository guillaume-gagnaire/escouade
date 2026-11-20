import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import App from '../../App.svelte';
import { inbox } from '../../lib/inbox/inbox.svelte';
import { handleShortcut } from '../../lib/shortcuts';
import { app } from '../../lib/state.svelte';
import type { InitialState, Settings, UiEvent, WindowAsk } from '../../lib/types';
import { agent, CLAUDE_ENGINE, fakeBackend, project, resetApp, SETTINGS } from '../../test/ipc';
import { windowAsk } from '../../test/inbox';

// The box « À toi » in the window (ESC-510): its tile and count in the rail only while the MCP server is on, turned on and
// off without a restart; its count agreeing with « N en attente » of the status bar; Ctrl Maj J.

let channel: { onmessage: (e: UiEvent) => void } | null = null;
const emit = (e: UiEvent) => channel!.onmessage(e);

async function start(settings: Partial<Settings>, asks: () => WindowAsk[]) {
  const initial: InitialState = {
    projects: [project()],
    agents: [agent({ id: 'a1', status: 'waiting', name: 'tests-e2e' }), agent({ id: 'a2', status: 'running', name: 'design-tableau' })],
    ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
    settings: { ...SETTINGS, ...settings },
    usage: { todayCost: 0, accounts: [], current: { claude: 'principal' } },
    git: {},
    shells: [],
    terminals: [],
    tickets: [],
    claudeFound: true,
    version: '0.1.0',
    models: [],
    engines: [CLAUDE_ENGINE],
  };
  const backend = fakeBackend({
    subscribe: (args: any) => {
      channel = args.channel;
      return initial;
    },
    get_conversation: () => [],
    git_files: () => [],
    git_branches: () => ['main'],
    inbox_list: () =>
      app.settings.mcpEnabled ? { enabled: true, asks: [...asks()], open: asks().length } : { enabled: false, asks: [], open: 0 },
  });
  resetApp();
  app.ready = false;
  render(App);
  await waitFor(() => expect(app.ready).toBe(true));
  return backend;
}

const tile = () => screen.queryByTestId('inbox-tile');

describe('the box « À toi » in the window', () => {
  it('shows nothing of the box while the MCP server is off, and its tile once it is on, without a restart', async () => {
    const asks = [windowAsk(), windowAsk({ id: 'a-2', state: 'snoozed' })];
    const backend = await start({ mcpEnabled: false }, () => asks);
    expect(tile()).toBeNull();
    // Ctrl Maj J does nothing, and the box's view never opens.
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'J', ctrlKey: true, shiftKey: true }), false)).toBe(false);
    app.openInbox();
    expect(app.ui.view).toBe('project');
    expect(backend.called('inbox_list')).toHaveLength(0);

    app.settings.mcpEnabled = true;
    flushSync();
    await waitFor(() => expect(tile()).not.toBeNull());
    // One request open (the other put off) and one agent stopped: 2, amber.
    const count = await screen.findByTestId('inbox-count');
    expect(count).toHaveTextContent('2');
    expect(count).toHaveClass('wt');
    // The status bar says the same stopped agent in « en attente ».
    expect(screen.getByText('en attente').closest('button')).toHaveTextContent('1 en attente');
    expect(tile()).toHaveAccessibleName('À toi, 2 choses t’attendent');

    // Opened from its tile, then by Ctrl Maj J closed again.
    await userEvent.click(tile()!);
    expect(app.ui.view).toBe('inbox');
    expect(await screen.findByTestId('inbox-queue')).toBeInTheDocument();
    expect(backend.called('inbox_track').at(-1)?.args).toEqual({ event: 'opened', from: 'tile', scope: 'all', kind: null });
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'J', ctrlKey: true, shiftKey: true }), false)).toBe(true);
    expect(app.ui.view).toBe('project');
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'J', ctrlKey: true, shiftKey: true }), false)).toBe(true);
    expect(app.ui.view).toBe('inbox');

    // Turned off with the box open: it closes, its tile and its count go; turned on again, they come back.
    app.settings.mcpEnabled = false;
    flushSync();
    await waitFor(() => expect(tile()).toBeNull());
    expect(app.ui.view).toBe('project');
    expect(screen.queryByTestId('inbox-queue')).toBeNull();
    expect(inbox.asks).toEqual([]);
    app.settings.mcpEnabled = true;
    flushSync();
    await waitFor(() => expect(screen.getByTestId('inbox-count')).toHaveTextContent('2'));
  });

  it('reads the box again when the core tells it changed, and counts a request left since', async () => {
    const asks = [windowAsk()];
    await start({ mcpEnabled: true }, () => asks);
    expect(await screen.findByTestId('inbox-count')).toHaveTextContent('2');
    asks.push(windowAsk({ id: 'a-3', title: 'Une autre' }));
    emit({ type: 'inbox', projectId: 'p1' });
    await waitFor(() => expect(screen.getByTestId('inbox-count')).toHaveTextContent('3'));
    // Said to a screen reader without taking the focus.
    await userEvent.click(tile()!);
    const status = within(screen.getByTestId('inbox')).getAllByRole('status')[0];
    expect(status).toHaveTextContent('Nouvelle demande de design-tableau : Une autre');
    expect(document.activeElement).not.toBe(status);
  });

  it('counts the stopped agents in the queue as « en attente » does, and leads to their conversation', async () => {
    await start({ mcpEnabled: true }, () => []);
    await userEvent.click(await screen.findByTestId('inbox-tile'));
    const stopped = await screen.findAllByTestId('inbox-stopped');
    expect(stopped).toHaveLength(1);
    expect(stopped[0]).toHaveTextContent('tests-e2e');
    await userEvent.click(stopped[0]);
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir la conversation' }));
    expect(app.ui.view).toBe('project');
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });
});
