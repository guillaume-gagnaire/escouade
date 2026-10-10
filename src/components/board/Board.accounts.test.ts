import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The cards' test launches have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { Account, AutopilotPause } from '../../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import Board from './Board.svelte';

const PRINCIPAL: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };

const col = (name: string) => screen.getByRole('region', { name });
const at = (h: number, m = 0) => {
  const d = new Date();
  d.setHours(h, m, 0, 0);
  return d.getTime();
};
const pause = (over: Partial<AutopilotPause> = {}): AutopilotPause => ({ reason: 'fiveHour', pct: 97, until: at(14), ...over });

describe('the board with several accounts', () => {
  beforeEach(() => {
    resetApp({ tickets: [ticket(), ticket({ id: 't2', key: 'DEM-2', title: 'Deuxième', rank: 2 })] });
    app.claudeFound = true;
    app.settings.accounts = [PRINCIPAL, PRO];
    fakeBackend();
  });

  it('names the accounts past the threshold under the pause', () => {
    app.autopilotPause = pause({ accounts: ['principal', 'pro'] });
    render(Board, { project: app.projects[0] });
    const line = screen.getByRole('status');
    expect(line).toHaveTextContent('Pilote auto en pause : quota de 5 h à 97 % (reprise à 14:00)');
    expect(line).toHaveTextContent('Les comptes Principal et Pro ont passé le seuil.');
  });

  it('names no account after a usage limit, nor when the backend names none', () => {
    app.autopilotPause = pause({ reason: 'limit', pct: null, accounts: ['principal', 'pro'] });
    render(Board, { project: app.projects[0] });
    expect(screen.getByRole('status')).not.toHaveTextContent('seuil');
  });

  it('speaks English', () => {
    setLang('en');
    app.autopilotPause = pause({ accounts: ['principal'] });
    render(Board, { project: app.projects[0] });
    expect(screen.getByRole('status')).toHaveTextContent('The Main account is past the threshold.');
  });

  it('shows a project that prefers an account the pause of that account alone', async () => {
    app.projects[0].account = 'pro';
    // Every other account is past the threshold, not Pro: this project's tickets go on.
    app.autopilotPause = pause({ accounts: ['principal'] });
    render(Board, { project: app.projects[0] });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.getByText('2 places libres')).toBeInTheDocument();
    // Pro past it: the backend tells this project's pause.
    app.projectPauses = { p1: pause({ pct: 100, accounts: ['pro'] }) };
    expect(await screen.findByRole('status')).toHaveTextContent('Le compte Pro a passé le seuil.');
    expect(within(col('À faire')).getAllByText('En attente : pilote auto en pause')).toHaveLength(2);
  });

  it('keeps the places when an agent waits for the quota of an account while another could take a ticket', async () => {
    app.agents = { a1: agent({ id: 'a1', account: 'principal', resumeAt: at(15), status: 'idle' }) };
    render(Board, { project: app.projects[0] });
    expect(screen.queryByText(/Quota atteint/)).not.toBeInTheDocument();
    expect(screen.getByText('2 places libres')).toBeInTheDocument();
    // Both wait: nothing starts before the first resume.
    app.agents = { ...app.agents, a2: agent({ id: 'a2', account: 'pro', resumeAt: at(16), status: 'idle' }) };
    expect(await screen.findByText('Quota atteint — reprise à 15:00')).toBeInTheDocument();
  });
});
