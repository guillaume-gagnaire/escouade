import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// "▶ Tester" starts test launches, which have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import type { Report } from '../../lib/escouade';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import CriteriaReport from './CriteriaReport.svelte';

const full = (): Report => ({
  criteria: [
    { n: 1, ok: true, note: '' },
    { n: 2, ok: false, note: 'pas encore' },
    { n: 3, ok: false, note: '' },
  ],
  progress: ['Le fichier est créé'],
  recipe: {
    prepare: [{ command: 'npm ci', dir: '' }],
    processes: [
      { name: 'web', command: 'npm run dev', dir: '', env: {}, url: 'http://localhost:3000' },
      { name: '', command: 'npm run api', dir: '', env: {}, url: '' },
    ],
    open: 'http://localhost:3000',
  },
});

const criteria = [{ text: 'Le fichier existe', ok: false, note: '' }];

// The agent of a worktree that can be tested.
const tester = () => agent({ worktree: { path: 'C:/wt', branch: 'ccm/a1', baseBranch: 'main' }, recipe: full().recipe });

describe('CriteriaReport', () => {
  beforeEach(() => {
    resetApp({ agents: [tester()] });
    fakeBackend();
  });

  it('lists the criteria met, then what was done and the test launch', () => {
    render(CriteriaReport, { report: full(), criteria, agent: app.agents.a1 });
    expect(screen.getByText('Bilan des critères')).toBeInTheDocument();
    expect(screen.getByText('1/3')).toBeInTheDocument();
    const list = within(screen.getByRole('list', { name: 'Bilan des critères' }));
    expect(list.getByText('Le fichier existe')).toBeInTheDocument();
    // A criterion the ticket does not have is numbered.
    expect(list.getByText('Critère 2')).toBeInTheDocument();
    expect(list.getByText('pas encore')).toBeInTheDocument();
    expect(screen.getByText('Ce qui a été fait')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Avancement' })).toHaveTextContent('Le fichier est créé');
    const launch = within(screen.getByRole('list', { name: 'Lancement de test' }));
    expect(launch.getByText('Préparation')).toBeInTheDocument();
    expect(launch.getByText('web')).toBeInTheDocument();
    expect(launch.getByText('processus 2')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '▶ Tester' })).toBeInTheDocument();
  });

  it('is written in English', () => {
    setLang('en');
    render(CriteriaReport, { report: full(), criteria, agent: app.agents.a1 });
    expect(screen.getByText('Criteria report')).toBeInTheDocument();
    const list = within(screen.getByRole('list', { name: 'Criteria report' }));
    expect(list.getByText('Criterion 2')).toBeInTheDocument();
    expect(screen.getByText('What was done')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Progress' })).toBeInTheDocument();
    const launch = within(screen.getByRole('list', { name: 'Test launch' }));
    expect(launch.getByText('Setup')).toBeInTheDocument();
    expect(launch.getByText('process 2')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '▶ Test' })).toBeInTheDocument();
  });
});
