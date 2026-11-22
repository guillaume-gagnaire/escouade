import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { LaunchAsk } from '../../lib/types';
import { agent, CLAUDE_ENGINE, fakeBackend, project, resetApp, usableEngines } from '../../test/ipc';
import Conversation from '../Conversation.svelte';
import HarnessLaunchModal from './HarnessLaunchModal.svelte';

// The agreement to a harness a plugin of a third party describes (audit SEC-01, ESC-589): its first launch says its whole
// command (the program found, its arguments, its variables), « Autoriser et lancer » keeps the line read by its
// fingerprint, « Refuser » keeps nothing; its agent's conversation says it waits, or that « Refuser » holds it.

const OUTIL = 'plugin:dev.example.outil:outil';
const P1 = project({ id: 'p1', name: 'demo-api' });
const LINE: LaunchAsk = {
  engine: OUTIL,
  harness: 'Outil',
  plugin: 'dev.example.outil',
  pluginName: 'Outil pour X',
  program: 'C:\\Users\\moi\\AppData\\Roaming\\npm\\npx.cmd',
  args: ['un-paquet', '--mode', 'acp'],
  resumeArgs: ['--resume', '{session}'],
  versionArgs: ['--version'],
  env: [{ name: 'OUTIL_COULEUR', value: 'bleu clair' }],
  removeEnv: ['OUTIL_TOKEN'],
  escouadeEnv: ['OUTIL_HOME'],
  fingerprint: 'f'.repeat(64),
  held: 'asks',
};

function backend(line: LaunchAsk = LINE) {
  return fakeBackend({
    harness_launch_ask: () => line,
    harness_agree: () => null,
    harness_launches: () => ({}),
    get_conversation: () => [],
  });
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [P1] });
  app.engines = [CLAUDE_ENGINE, { ...CLAUDE_ENGINE, id: OUTIL, name: 'Outil' }];
  usableEngines('claude', OUTIL);
});

afterEach(() => cleanup());

describe('the window of the agreement to a third party’s harness (SEC-01)', () => {
  it('says its whole command: the program found, its arguments, its variables', async () => {
    backend();
    render(HarnessLaunchModal, { engine: OUTIL, agentId: 'a1' });
    expect(await screen.findByText('Lancer Outil ?')).toBeInTheDocument();
    expect(screen.getByText(/vient du plugin tiers Outil pour X \(dev\.example\.outil\)/)).toBeInTheDocument();
    expect(screen.getByTestId('launch-line')).toHaveTextContent('C:\\Users\\moi\\AppData\\Roaming\\npm\\npx.cmd un-paquet --mode acp');
    expect(screen.getByTestId('launch-program')).toHaveTextContent('C:\\Users\\moi\\AppData\\Roaming\\npm\\npx.cmd');
    expect(screen.getByText('--resume {session}')).toBeInTheDocument();
    expect(screen.getByText('--version')).toBeInTheDocument();
    expect(screen.getByTestId('launch-env')).toHaveTextContent('OUTIL_COULEUR="bleu clair"');
    expect(screen.getByText('OUTIL_TOKEN')).toBeInTheDocument();
    expect(screen.getByText('OUTIL_HOME')).toBeInTheDocument();
    expect(screen.getByText(/une mise à jour du plugin qui la change te la redemande/)).toBeInTheDocument();
  });

  it('« Autoriser et lancer » agrees to the line read, by its fingerprint', async () => {
    const b = backend();
    app.modal = { kind: 'harnessLaunch', engine: OUTIL, agentId: 'a1' };
    render(HarnessLaunchModal, { engine: OUTIL, agentId: 'a1' });
    await userEvent.click(await screen.findByRole('button', { name: 'Autoriser et lancer' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(b.called('harness_agree').map((c) => c.args)).toEqual([
      { engine: OUTIL, agent: 'a1', fingerprint: LINE.fingerprint, allow: true },
    ]);
    expect(b.called('harness_launches')).toHaveLength(1);
  });

  it('« Refuser » keeps nothing and says so to the core', async () => {
    const b = backend();
    app.modal = { kind: 'harnessLaunch', engine: OUTIL, agentId: 'a1' };
    render(HarnessLaunchModal, { engine: OUTIL, agentId: 'a1' });
    await userEvent.click(await screen.findByRole('button', { name: 'Refuser' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(b.called('harness_agree').map((c) => c.args)).toEqual([
      { engine: OUTIL, agent: 'a1', fingerprint: LINE.fingerprint, allow: false },
    ]);
  });

  it('with « Refuser » set, says no harness of a third party launches, and offers nothing to agree to', async () => {
    backend({ ...LINE, held: 'refused' });
    render(HarnessLaunchModal, { engine: OUTIL, agentId: 'a1' });
    expect(await screen.findByText(/aucun harness d’un plugin tiers ne se lance/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Autoriser et lancer' })).toBeNull();
    expect(screen.getAllByRole('button', { name: 'Fermer' }).length).toBeGreaterThan(0);
  });
});

describe('an agent of a third party’s harness (SEC-01)', () => {
  it('says it waits for the agreement, and « Voir la commande… » opens the window', async () => {
    backend();
    const a = agent({ id: 'a1', projectId: 'p1', engine: OUTIL });
    resetApp({ projects: [P1], agents: [a] });
    app.harnessLaunches = { [OUTIL]: 'asks' };
    render(Conversation, { agent: a, project: P1 });
    const banner = await screen.findByTestId('harness-launch-held');
    expect(banner).toHaveTextContent(/vient d’un plugin tiers : il attend ton accord pour lancer sa commande/);
    await userEvent.click(screen.getByRole('button', { name: 'Voir la commande…' }));
    expect(app.modal).toEqual({ kind: 'harnessLaunch', engine: OUTIL, agentId: 'a1' });
  });

  it('says « Refuser » holds it', async () => {
    backend();
    const a = agent({ id: 'a1', projectId: 'p1', engine: OUTIL });
    resetApp({ projects: [P1], agents: [a] });
    app.harnessLaunches = { [OUTIL]: 'refused' };
    render(Conversation, { agent: a, project: P1 });
    expect(await screen.findByTestId('harness-launch-held')).toHaveTextContent('« Harness des plugins tiers » est sur « Refuser »');
  });

  it('a new agent of it asks at once', async () => {
    fakeBackend({
      create_agent: () => agent({ id: 'new', projectId: 'p1', engine: OUTIL }),
      get_conversation: () => [],
    });
    app.harnessLaunches = { [OUTIL]: 'asks' };
    await app.newAgent('p1', OUTIL);
    expect(app.modal).toEqual({ kind: 'harnessLaunch', engine: OUTIL, agentId: 'new' });
  });

  it('an agent of the first hand says nothing', async () => {
    backend();
    const a = agent({ id: 'a2', projectId: 'p1', engine: 'claude' });
    resetApp({ projects: [P1], agents: [a] });
    app.harnessLaunches = {};
    render(Conversation, { agent: a, project: P1 });
    await waitFor(() => expect(screen.queryByTestId('harness-launch-held')).toBeNull());
  });
});
