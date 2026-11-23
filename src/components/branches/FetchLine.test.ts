import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { project, resetApp } from '../../test/ipc';
import type { FetchFailure } from '../../lib/types';
import FetchLine from './FetchLine.svelte';

// The Git panel's line under « Fetch » (ESC-561): when it was fetched, why the background fetch failed, quietly.
describe('FetchLine', () => {
  const now = Date.UTC(2026, 10, 20, 14, 0, 0);
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.now = now;
  });
  afterEach(() => {
    cleanup();
    setLang('fr');
  });

  const failure = (kind: FetchFailure['kind'], message = 'git said'): FetchFailure => ({
    kind,
    message,
    at: now - 60_000,
    timeoutSecs: 300,
  });

  it('says when it was last fetched, the time on hover', () => {
    render(FetchLine, { projectId: 'p1', lastFetch: now - 3 * 60_000 });
    const line = screen.getByText('Récupéré il y a 3 min');
    expect(line).toHaveAccessibleDescription(/^Dernier fetch : /);
    expect(screen.queryByText(/Fetch automatique/)).toBeNull();
  });

  it('says never fetched', () => {
    render(FetchLine, { projectId: 'p1', lastFetch: null });
    expect(screen.getByText('Jamais récupéré')).toBeInTheDocument();
  });

  it('tells each failure by its reason in plain words, git’s on hover, never as an alert', () => {
    const { unmount } = render(FetchLine, {
      projectId: 'p1',
      lastFetch: null,
      failure: failure('credentials', 'git@github.com: Permission denied (publickey).'),
    });
    const said = screen.getByText(/il demande des identifiants\. Fais « Fetch » une fois : il reprendra ensuite\./);
    expect(said).toHaveAccessibleDescription('git@github.com: Permission denied (publickey).');
    expect(screen.queryByRole('alert')).toBeNull();
    unmount();
    render(FetchLine, { projectId: 'p1', lastFetch: null, failure: failure('network') });
    expect(screen.getByText(/le réseau ou le dépôt distant est injoignable\./)).toBeInTheDocument();
  });

  it('a time limit passed says it and opens where it is set: the project’s own, else the general one', async () => {
    let closed = 0;
    render(FetchLine, { projectId: 'p1', lastFetch: now - 3600_000, failure: failure('timeout'), onsettings: () => closed++ });
    expect(screen.getByText(/pas fini en 5 min\. Le délai d’attente se règle dans les réglages\./)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Régler' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'allProjects', projectId: 'p1' });
    expect(closed).toBe(1);
    cleanup();
    app.projects[0].gitFetch = { mode: 'default', everyMin: 0, timeoutSecs: 900 };
    render(FetchLine, { projectId: 'p1', lastFetch: null, failure: failure('timeout') });
    await userEvent.click(screen.getByRole('button', { name: 'Régler' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'git', projectId: 'p1' });
  });

  it('says to update a Git for Windows whose ssh stalls', () => {
    render(FetchLine, { projectId: 'p1', lastFetch: now, sshNote: true });
    expect(screen.getByText('Récupéré à l’instant')).toBeInTheDocument();
    expect(screen.getByText(/Mets Git à jour \(2\.47\.0\(2\) ou plus\)\./)).toBeInTheDocument();
  });

  it('is in English too', () => {
    setLang('en');
    render(FetchLine, { projectId: 'p1', lastFetch: now - 3 * 60_000, failure: failure('network') });
    expect(screen.getByText(/^Fetched /)).toBeInTheDocument();
    expect(screen.getByText(/the network or the remote cannot be reached\./)).toBeInTheDocument();
  });
});
