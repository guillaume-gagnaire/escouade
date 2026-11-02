import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { ProjectEnvironments, Ticket } from '../../lib/types';
import { fakeBackend, resetApp, ticket } from '../../test/ipc';
import Board from './Board.svelte';

const NOW = 1_800_000_000_000;

const done = (over: Partial<Ticket>) =>
  ticket({ column: 'done', outcome: '⤵ Mergé dans main', outcomeCommit: 'abc1234def', doneAt: NOW - 3600_000, ...over });

const ENVS: ProjectEnvironments = {
  projectId: 'p1',
  host: 'GitHub',
  unsupported: false,
  readAt: NOW - 60_000,
  environments: [
    {
      name: 'staging',
      url: 'https://staging.example.com',
      production: false,
      failed: false,
      running: { environment: 'staging', state: 'success', url: null, sha: 'c71e5d2aaaa', created: NOW / 1000 - 3600, author: 'ada' },
    },
    {
      name: 'production',
      url: 'https://example.com',
      production: true,
      failed: true,
      running: { environment: 'production', state: 'success', url: null, sha: '9a40b17bbbb', author: 'grace' },
      latest: {
        environment: 'production',
        state: 'failure',
        url: null,
        sha: 'c71e5d2aaaa',
        log: 'https://github.com/acme/demo/actions/runs/1/job/2',
      },
    },
  ],
};

const lane = (name: string) => document.querySelector(`[data-env-lane="${name}"]`) as HTMLElement;

describe('The view « Environnements » (ESC-67)', () => {
  beforeEach(() => {
    resetApp({
      tickets: [
        done({ id: 'w', key: 'DEM-48', title: 'Code promo', environments: [] }),
        done({
          id: 's',
          key: 'DEM-46',
          title: 'Paiement 3DS',
          environments: [
            { name: 'staging', deployed: true },
            { name: 'production', deployed: false },
          ],
        }),
        done({
          id: 'p',
          key: 'DEM-43',
          title: 'Recherche',
          environments: [
            { name: 'staging', deployed: true },
            { name: 'production', deployed: true },
          ],
        }),
      ],
    });
    app.enginesFound = ['claude'];
    app.now = NOW;
  });

  it('shows what runs where, the tickets of each environment, those waiting for production and a failure', async () => {
    const backend = fakeBackend({ project_environments: () => null, project_environments_read: () => ENVS });
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: 'Environnements' }));
    await waitFor(() => expect(lane('production')).toBeTruthy());
    expect(backend.called('project_environments_read')).toEqual([{ cmd: 'project_environments_read', args: { projectId: 'p1' } }]);
    // Merged, deployed nowhere yet; then each ticket in the furthest environment it reached.
    expect(within(lane('waiting')).getByText('DEM-48')).toBeInTheDocument();
    expect(within(lane('staging')).getByText('DEM-46')).toBeInTheDocument();
    expect(within(lane('production')).getByText('DEM-43')).toBeInTheDocument();
    expect(within(lane('production')).queryByText('DEM-46')).toBeNull();
    // What runs there: its commit, when, who, its address.
    expect(lane('staging')).toHaveTextContent('c71e5d2 · il y a 1 h · par ada');
    expect(document.querySelector('[data-not-in-prod]')).toHaveTextContent('2 tickets pas encore en production');
    expect(within(lane('production')).getByRole('status')).toHaveTextContent('Dernier déploiement en échec · c71e5d2');
    await userEvent.click(within(lane('production')).getByRole('button', { name: /Voir le job/ }));
    expect(backend.called('open_url').at(-1)!.args.url).toBe('https://github.com/acme/demo/actions/runs/1/job/2');
    await userEvent.click(within(lane('staging')).getByRole('button', { name: /staging\.example\.com/ }));
    expect(backend.called('open_url').at(-1)!.args.url).toBe('https://staging.example.com');
    // « Rafraîchir » reads again; « Tickets » brings the columns back.
    await userEvent.click(screen.getByRole('button', { name: 'Rafraîchir' }));
    expect(backend.called('project_environments_read')).toHaveLength(2);
    await userEvent.click(screen.getByRole('button', { name: 'Tickets' }));
    expect(screen.getByRole('region', { name: 'Terminé' })).toBeInTheDocument();
  });

  it('opens on the environments the backend holds, without reading again when read lately', async () => {
    const backend = fakeBackend({ project_environments: () => ENVS });
    app.envView.p1 = true;
    render(Board, { project: app.projects[0] });
    await waitFor(() => expect(lane('staging')).toBeTruthy());
    expect(backend.called('project_environments_read')).toEqual([]);
  });

  it('says when the host has no deployments, and in English', async () => {
    fakeBackend({
      project_environments: () => null,
      project_environments_read: () => ({ ...ENVS, host: 'Gitea', unsupported: true, environments: [] }),
    });
    app.envView.p1 = true;
    render(Board, { project: app.projects[0] });
    expect(await screen.findByText(/Gitea n’a pas d’API de déploiements/)).toBeInTheDocument();
    setLang('en');
    expect(await screen.findByText(/Gitea has no deployments API/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Environments' })).toBeInTheDocument();
    setLang('fr');
  });
});
