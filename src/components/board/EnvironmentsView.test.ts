import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
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
import type { CodeHost, Environment, ProjectEnvironments, Ticket } from '../../lib/types';
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

const GITHUB_HOST: CodeHost = { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true };

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
    app.codeHosts.p1 = GITHUB_HOST;
    localStorage.clear();
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

describe('The environments shown, the header of the view (ESC-390)', () => {
  /** github-pages, never built; npm, the publication of a package, where every merged ticket lands. */
  const PAGES: Environment = {
    name: 'github-pages',
    production: false,
    failed: false,
    running: null,
    latest: { environment: 'github-pages', state: 'in_progress', url: null, sha: 'c80b3cf9999' },
  };
  const NPM: Environment = {
    name: 'npm',
    production: false,
    failed: false,
    running: { environment: 'npm', state: 'success', url: null, sha: 'df890f9aaaa', created: NOW / 1000 - 3 * 86400, author: 'ada' },
  };
  const WITH = (...more: Environment[]): ProjectEnvironments => ({ ...ENVS, environments: [...more, ...ENVS.environments] });
  const shownLanes = () => [...document.querySelectorAll('[data-env-lane]')].map((l) => l.getAttribute('data-env-lane'));
  const menuButton = () => screen.getByRole('button', { name: /environnements? sur/ });

  beforeEach(() => {
    resetApp({
      tickets: [
        done({ id: 'n', key: 'DEM-50', title: 'SDK', environments: [{ name: 'npm', deployed: true }] }),
        done({
          id: 'p',
          key: 'DEM-43',
          title: 'Recherche',
          environments: [
            { name: 'staging', deployed: true },
            { name: 'production', deployed: true },
            { name: 'npm', deployed: true },
          ],
        }),
      ],
    });
    app.enginesFound = ['claude'];
    app.now = NOW;
    app.codeHosts.p1 = GITHUB_HOST;
    localStorage.clear();
  });

  it('has no « Environnements » without a code host that reads deployments, and shows the tickets then', () => {
    fakeBackend({ project_environments: () => ENVS });
    app.envView.p1 = true;
    app.codeHosts.p1 = null;
    const { unmount } = render(Board, { project: app.projects[0] });
    expect(screen.queryByRole('button', { name: 'Environnements' })).toBeNull();
    expect(screen.getByRole('heading', { name: 'Kanban' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Terminé' })).toBeInTheDocument();
    unmount();
    // Gitea: a code host, no deployments to read.
    app.codeHosts.p1 = { ...GITHUB_HOST, kind: 'gitea', name: 'Gitea', deployments: false };
    render(Board, { project: app.projects[0] });
    expect(screen.queryByRole('button', { name: 'Environnements' })).toBeNull();
  });

  it('leaves out an environment where nothing ever ran, says so, and says a deployment under way on its lane', async () => {
    fakeBackend({ project_environments: () => WITH(PAGES, NPM) });
    app.envView.p1 = true;
    render(Board, { project: app.projects[0] });
    await waitFor(() => expect(lane('npm')).toBeTruthy());
    expect(shownLanes()).toEqual(['waiting', 'npm', 'staging', 'production']);
    expect(document.querySelector('[data-never-ran]')).toHaveTextContent('github-pages n’est pas montré : aucun déploiement réussi.');
    expect(menuButton()).toHaveTextContent('3 environnements sur 4');
    // The tab counts the environments shown; the header says the host and when it was read.
    expect(screen.getByRole('button', { name: 'Environnements' })).toHaveTextContent('3');
    expect(screen.getByText('demo-api · GitHub · lu il y a 1 min')).toBeInTheDocument();
    // An empty lane says it in its place.
    expect(lane('waiting')).toHaveTextContent('Rien n’attend un déploiement.');
    expect(lane('staging')).toHaveTextContent('Aucun ticket n’est arrivé ici.');
  });

  it('hides an environment from its menu: its tickets wait again, the choice kept for the project', async () => {
    fakeBackend({ project_environments: () => WITH(PAGES, NPM) });
    app.envView.p1 = true;
    const { unmount } = render(Board, { project: app.projects[0] });
    await waitFor(() => expect(lane('npm')).toBeTruthy());
    expect(within(lane('npm')).getByText('DEM-50')).toBeInTheDocument();
    await userEvent.click(menuButton());
    const dialog = screen.getByRole('dialog', { name: 'Environnements montrés' });
    const rows = () => [...dialog.querySelectorAll('[data-env-row]')].map((r) => r.textContent?.replace(/\s+/g, ' ').trim());
    expect(rows()).toEqual(['github-pages aucun déploiement réussi', 'npm 1 ticket', 'staging 0 ticket', 'production production']);
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'npm' }));
    expect(lane('npm')).toBeNull();
    expect(within(lane('waiting')).getByText('DEM-50')).toBeInTheDocument();
    expect(rows()[1]).toBe('npm masqué par toi');
    expect(menuButton()).toHaveTextContent('2 environnements sur 4');
    // Shown again: github-pages, though nothing ran there, with its deployment under way.
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'github-pages' }));
    expect(lane('github-pages')).toHaveTextContent('Aucun déploiement réussi');
    expect(lane('github-pages').querySelector('[data-env-underway]')).toHaveTextContent('Déploiement en cours · c80b3cf');
    // Escape closes it; the choice stays for the project.
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    unmount();
    render(Board, { project: app.projects[0] });
    await waitFor(() => expect(lane('github-pages')).toBeTruthy());
    expect(lane('npm')).toBeNull();
  });

  it('orders the environments with Alt+↑ and Alt+↓, or by dragging; a ticket goes to the furthest lane it reached', async () => {
    fakeBackend({ project_environments: () => WITH(NPM) });
    app.envView.p1 = true;
    render(Board, { project: app.projects[0] });
    await waitFor(() => expect(lane('npm')).toBeTruthy());
    expect(within(lane('production')).getByText('DEM-43')).toBeInTheDocument();
    await userEvent.click(menuButton());
    const dialog = screen.getByRole('dialog', { name: 'Environnements montrés' });
    // npm after production: the ticket in both is placed in npm, the furthest now.
    within(dialog).getByRole('checkbox', { name: 'npm' }).focus();
    await userEvent.keyboard('{Alt>}{ArrowDown}{ArrowDown}{/Alt}');
    expect(shownLanes()).toEqual(['waiting', 'staging', 'production', 'npm']);
    expect(within(lane('npm')).getByText('DEM-43')).toBeInTheDocument();
    await waitFor(() => expect(within(dialog).getByRole('checkbox', { name: 'npm' })).toHaveFocus());
    // Dragged back before staging.
    const row = (n: string) => dialog.querySelector(`[data-env-row="${n}"]`)!;
    await fireEvent.dragStart(row('npm'));
    await fireEvent.dragOver(row('staging'));
    await fireEvent.drop(row('staging'));
    expect(shownLanes()).toEqual(['waiting', 'npm', 'staging', 'production']);
  });
});
