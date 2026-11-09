import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
  openAgentTerminal: async () => ({ id: 't1', projectId: '', name: 'x', shell: 'claude' }),
  getXTerm: () => undefined,
  roomOf: () => null,
  mountTerminal() {},
  disposeTerminal() {},
  openTerminal: async () => ({ id: 't', projectId: '', name: '', shell: '' }),
}));

import { choosesEngine, engineOptions, harnessAsks, harnessHeld, offered, unusable } from '../../lib/engines';
import { setLang } from '../../lib/i18n';
import type { PluginView } from '../../lib/plugins';
import { app } from '../../lib/state.svelte';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, project, resetApp, ticket, usableEngines } from '../../test/ipc';
import Conversation from '../Conversation.svelte';
import TicketCard from '../board/TicketCard.svelte';
import HarnessConsentModal from './HarnessConsentModal.svelte';

// A harness project by project (ESC-404): its menus leave out a harness turned off for the project, a private project's
// first agent on a harness asks first (what its plugin will get, its permissions), and the cards of an agent or a ticket
// whose harness is off for its project say so with « Activer ».

const CLAUDE_PLUGIN = 'dev.escouade.claude-code';
const claudePlugin = {
  id: CLAUDE_PLUGIN,
  name: 'Claude Code',
  asks: ['run', 'harness:claude -p'],
  manifest: { id: CLAUDE_PLUGIN, contributes: { harnesses: [{ id: 'claude' }] }, permissions: {} },
  choices: { enabled: true, projects: {}, revoked: [], settings: {} },
} as unknown as PluginView;

const OPEN = project({ id: 'p1', name: 'demo-api' });
const VAULT = project({ id: 'p2', name: 'coffre', private: true });

function backend(over: Record<string, (args: any) => unknown> = {}) {
  return fakeBackend({
    harness_plugin_of: () => CLAUDE_PLUGIN,
    plugins_list: () => [claudePlugin],
    harness_enable_in: () => {
      app.harnessScope = {};
      return null;
    },
    harness_scope: () => app.harnessScope,
    engines_declared: () => app.engines,
    engines_found_now: () => app.enginesFound,
    engines_off_now: () => [],
    create_agent: () => agent({ id: 'new', projectId: 'p2' }),
    get_conversation: () => [],
    ...over,
  });
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [OPEN, VAULT] });
  app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  usableEngines('claude', 'codex');
});

describe('the menus « Harness » of a project (ESC-404)', () => {
  it('leave out a harness turned off for the project, and keep it elsewhere', () => {
    app.harnessScope = { p1: { off: ['codex'], asks: [] } };
    expect(offered('p1').map((e) => e.id)).toEqual(['claude']);
    expect(offered('p2').map((e) => e.id)).toEqual(['claude', 'codex']);
    expect(offered().map((e) => e.id)).toEqual(['claude', 'codex']);
    expect(unusable('codex', 'p1')).toBe('projectOff');
    expect(choosesEngine(null, 'p1')).toBe(false);
    // The one a ticket or the project has stays listed, saying why.
    expect(engineOptions('codex', 'p1')).toEqual([
      { value: 'claude', label: 'Claude Code' },
      { value: 'codex', label: 'Harness nu (coupé pour ce projet)' },
    ]);
    expect(harnessHeld('p1', 'codex')).toBe(true);
    expect(harnessAsks('p1', 'codex')).toBe(false);
  });

  it('offer a private project’s harness not turned on yet, behind the question', () => {
    app.harnessScope = { p2: { off: [], asks: ['claude'] } };
    expect(offered('p2').map((e) => e.id)).toEqual(['claude', 'codex']);
    expect(harnessAsks('p2', 'claude')).toBe(true);
    expect(harnessHeld('p2', 'claude')).toBe(true);
    expect(harnessHeld('p1', 'claude')).toBe(false);
  });
});

describe('the first agent of a private project on a harness (ESC-404)', () => {
  it('asks once: « Annuler » makes nothing', async () => {
    app.harnessScope = { p2: { off: [], asks: ['claude'] } };
    const b = backend();
    const made = app.newAgent('p2', 'claude');
    await waitFor(() => expect(app.modal?.kind).toBe('harnessConsent'));
    const modal = app.modal as Extract<typeof app.modal, { kind: 'harnessConsent' }>;
    render(HarnessConsentModal, { ...modal });
    expect(await screen.findByText('Activer Claude Code pour ce projet privé ?')).toBeInTheDocument();
    // What it will get, and its permissions as the install window says them.
    expect(screen.getByText(/Ce que l’agent écrit et lit dans ce projet/)).toBeInTheDocument();
    expect(await screen.findByText('claude -p')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    await made;
    expect(app.modal).toBeNull();
    expect(b.called('harness_enable_in')).toHaveLength(0);
    expect(b.called('create_agent')).toHaveLength(0);
  });

  it('« Activer » turns its plugin on for the project, then the agent is made', async () => {
    app.harnessScope = { p2: { off: [], asks: ['claude'] } };
    const b = backend();
    const made = app.newAgent('p2', 'claude');
    await waitFor(() => expect(app.modal?.kind).toBe('harnessConsent'));
    const modal = app.modal as Extract<typeof app.modal, { kind: 'harnessConsent' }>;
    render(HarnessConsentModal, { ...modal });
    await userEvent.click(await screen.findByRole('button', { name: 'Activer' }));
    await made;
    expect(b.called('harness_enable_in').map((c) => c.args)).toEqual([{ project: 'p2', engine: 'claude' }]);
    expect(b.called('create_agent')).toHaveLength(1);
    expect(app.agents.new).toBeDefined();
  });

  it('asks nothing in a project where the harness is on', async () => {
    const b = backend();
    await app.newAgent('p1', 'claude');
    expect(app.modal).toBeNull();
    expect(b.called('create_agent')).toHaveLength(1);
  });
});

describe('the cards of a harness off for its project (ESC-404)', () => {
  it('say so on the conversation of its agent, with « Activer »', async () => {
    const a = agent({ id: 'held', projectId: 'p2', status: 'idle', engine: 'claude' });
    resetApp({ projects: [OPEN, VAULT], agents: [a] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.harnessScope = { p2: { off: [], asks: ['claude'] } };
    backend();
    render(Conversation, { agent: a, project: VAULT });
    const card = await screen.findByTestId('harness-held');
    expect(card).toHaveTextContent('Claude Code n’est pas activé pour ce projet privé');
    await userEvent.click(screen.getByRole('button', { name: 'Activer' }));
    expect(app.modal).toMatchObject({ kind: 'harnessConsent', projectId: 'p2', engine: 'claude' });
    cleanup();
    // On again: no card.
    app.harnessScope = {};
    app.modal = null;
    render(Conversation, { agent: a, project: VAULT });
    expect(screen.queryByTestId('harness-held')).toBeNull();
  });

  it('give a blocked ticket « Activer », which takes it up once agreed', async () => {
    app.harnessScope = { p2: { off: [], asks: ['claude'] } };
    const b = backend({ ticket_resume: () => null });
    const t = ticket({ id: 't9', projectId: 'p2', column: 'doing', agentId: null, blocked: 'Claude Code n’est pas activé…' });
    render(TicketCard, { ticket: t, project: VAULT, queueIndex: 0, busyCount: 0, quota: null, onedit: () => {} });
    expect(screen.queryByRole('button', { name: 'Reprendre' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Activer' }));
    const modal = app.modal as Extract<typeof app.modal, { kind: 'harnessConsent' }>;
    expect(modal).toMatchObject({ kind: 'harnessConsent', projectId: 'p2', engine: 'claude' });
    cleanup();
    render(HarnessConsentModal, { ...modal });
    await userEvent.click(await screen.findByRole('button', { name: 'Activer' }));
    await waitFor(() => expect(b.called('ticket_resume').map((c) => c.args)).toEqual([{ id: 't9' }]));
    expect(b.called('harness_enable_in')).toHaveLength(1);
  });
});
