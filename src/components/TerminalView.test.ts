import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { TermInfo } from '../lib/types';
import { fakeBackend, project, resetApp } from '../test/ipc';
import TerminalView from './TerminalView.svelte';

const mounted = vi.hoisted(() => [] as (string | null)[]);
const opened = vi.hoisted(() => [] as string[]);
vi.mock('../lib/terminals', () => ({
  getXTerm: () => undefined,
  mountTerminal: (key: string, el: HTMLElement | null) => mounted.push(el ? key : null),
  disposeTerminal() {},
  openTerminal: async (projectId: string, shell: string, name: string) => {
    opened.push(`${projectId}:${shell}:${name}`);
    return { id: 'term-2', projectId, name, shell };
  },
}));

const TERM: TermInfo = { id: 'term-1', projectId: 'p1', name: 'pwsh-1', shell: 'pwsh' };
const P = project();

describe('TerminalView', () => {
  beforeEach(() => {
    resetApp({ projects: [P] });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    app.terminals = [TERM];
    mounted.length = 0;
    opened.length = 0;
  });

  it('shows the terminal, its shell and the folder of its project', () => {
    fakeBackend();
    render(TerminalView, { term: TERM, project: P });
    expect(screen.getByText('pwsh-1')).toBeInTheDocument();
    expect(screen.getByText('PowerShell 7')).toBeInTheDocument();
    expect(mounted).toEqual(['term-1']);
    expect(screen.queryByText(/Processus terminé/)).not.toBeInTheDocument();
  });

  it('says the process ended, with its code, and starts the shell again', async () => {
    fakeBackend();
    app.exitedTerms['term-1'] = 2;
    render(TerminalView, { term: TERM, project: P });
    expect(screen.getByText(/Processus terminé \(code 2\)\./)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Relancer' }));
    expect(opened).toEqual(['p1:pwsh:pwsh-1']);
    expect(app.terminals.map((t) => t.id)).toEqual(['term-2']);
  });

  it('says it without a code when the shell gave none, and closes the terminal', async () => {
    fakeBackend();
    app.exitedTerms['term-1'] = null;
    render(TerminalView, { term: TERM, project: P });
    expect(screen.getByText(/Processus terminé\./)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le terminal' }));
    expect(app.terminals).toEqual([]);
  });
});

describe('TerminalView in English', () => {
  beforeEach(() => {
    resetApp({ projects: [P] });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    app.terminals = [TERM];
    setLang('en');
  });

  it('writes the buttons and the end of the process in English', () => {
    fakeBackend();
    app.exitedTerms['term-1'] = 2;
    render(TerminalView, { term: TERM, project: P });
    expect(screen.getByText(/Process exited \(code 2\)\./)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Restart' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close terminal' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Clear' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '⌕' })).toHaveAttribute('title', 'Search (Ctrl+Shift+F)');
  });

  it('says it without a code when the shell gave none', () => {
    fakeBackend();
    app.exitedTerms['term-1'] = null;
    render(TerminalView, { term: TERM, project: P });
    expect(screen.getByText(/Process exited\./)).toBeInTheDocument();
  });
});
