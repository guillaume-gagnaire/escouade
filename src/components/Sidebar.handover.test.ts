import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { Account, Agent, EngineDecl } from '../lib/types';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, project, resetApp } from '../test/ipc';
import Sidebar from './Sidebar.svelte';

// « Continuer sur un autre harness » and « Revenir sur <harness> » in the agent's menu (ESC-88).

vi.mock('../lib/terminals', async (original) => ({
  ...(await original<typeof import('../lib/terminals')>()),
  openTerminal: vi.fn(),
}));

const CODEX: EngineDecl = { ...BARE_ENGINE, id: 'codex', name: 'Codex', speaker: 'Codex' };
const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const CODEX_ACCOUNT: Account = { id: 'principal-codex', name: 'Principal', configDir: '', claudePath: '', active: true, engine: 'codex' };

const NOW = new Date(2026, 9, 10, 14, 0).getTime();

function withTwoHarnesses(a: Agent, accounts: Account[] = [PRINCIPAL, CODEX_ACCOUNT]) {
  resetApp({ projects: [project()], agents: [a] });
  app.now = NOW;
  app.engines = [CLAUDE_ENGINE, CODEX];
  app.enginesFound = ['claude', 'codex'];
  app.settings.accounts = accounts;
  app.settings.engineOrder = ['claude', 'codex'];
}

const items = () => menu.open?.items ?? [];
const item = (label: string) => items().find((i) => i.label === label);
const settle = () => new Promise((r) => setTimeout(r));

describe('Sidebar, the harness of an agent', () => {
  it('lets the agent go on on another harness at any time, from its menu', async () => {
    withTwoHarnesses(agent({ status: 'done' }));
    const backend = fakeBackend({ continue_on_engine: () => null });
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    const harnesses = item('Continuer sur un autre harness')!.submenu!;
    expect(harnesses.map((i) => i.label)).toEqual(['Continuer sur Codex']);
    harnesses[0].onClick!();
    await settle();
    expect(backend.called('continue_on_engine')[0].args).toEqual({ agentId: 'a1', engine: 'codex' });
  });

  it('waits for the end of a running turn, and offers nothing without another harness serving the project', async () => {
    withTwoHarnesses(agent({ status: 'running' }));
    fakeBackend();
    const { unmount } = render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(item('Continuer sur un autre harness')).toMatchObject({ disabled: true, title: 'Attends la fin du tour' });
    unmount();
    withTwoHarnesses(agent({ status: 'done' }), [PRINCIPAL, { ...CODEX_ACCOUNT, projects: ['p2'] }]);
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(item('Continuer sur un autre harness')).toBeUndefined();
  });

  it('offers the way back once the quota of the harness left came back', async () => {
    const left = { engine: 'claude' as const, account: 'principal', until: NOW - 1 };
    withTwoHarnesses(agent({ status: 'done', engine: 'codex', account: 'principal-codex', handedFrom: left }));
    const backend = fakeBackend({ continue_on_engine: () => null });
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    // The only other harness is the way back: no « Continuer sur… ».
    expect(item('Continuer sur un autre harness')).toBeUndefined();
    item('Revenir sur Claude Code')!.onClick!();
    await settle();
    expect(backend.called('continue_on_engine')[0].args).toEqual({ agentId: 'a1', engine: 'claude' });
  });

  it('does not offer the harness left before its quota resets', async () => {
    const left = { engine: 'claude' as const, account: 'principal', until: NOW + 60_000 };
    withTwoHarnesses(agent({ status: 'done', engine: 'codex', account: 'principal-codex', handedFrom: left }));
    fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(item('Revenir sur Claude Code')).toBeUndefined();
    expect(item('Continuer sur un autre harness')).toBeUndefined();
  });
});
