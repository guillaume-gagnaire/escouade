import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, pullRequest as pr, resetApp } from '../../test/ipc';
import PrsView from './PrsView.svelte';

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock('../../lib/opener', () => ({ openUrl }));

const PRS = [
  pr({ number: 12, title: 'Pagination par curseur', head: 'feat/cursor', author: 'ada', checks: 'pending' }),
  pr({ number: 11, title: 'Rotation des jetons', head: 'feat/tokens', author: 'bob', checks: 'success', approvals: 1, reviewers: ['ada'] }),
  pr({
    number: 10,
    title: 'Traductions espagnoles',
    url: 'https://github.com/acme/demo/pull/10',
    head: 'i18n-es',
    author: 'marta',
    fork: true,
    headRepo: 'marta/demo',
    mergeable: false,
  }),
];

beforeEach(() => {
  setLang('fr');
  localStorage.clear();
  openUrl.mockClear();
  resetApp({ projects: [project()] });
});

describe('the « Pull requests » view (ESC-65)', () => {
  it('lists the open pull requests with their CI, reviews, conflict and fork, and filters them', async () => {
    fakeBackend({ project_prs: () => ({ data: { word: 'PR', prs: PRS, me: 'ada' } }) });
    render(PrsView, { project: project() });
    expect(screen.getByRole('heading', { name: 'Pull requests' })).toBeInTheDocument();
    const rows = await screen.findAllByRole('listitem');
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent('#12');
    expect(rows[0]).toHaveTextContent('CI en cours');
    expect(rows[1]).toHaveTextContent('1 approbation');
    expect(rows[1]).toHaveTextContent('Ta relecture');
    expect(rows[2]).toHaveTextContent('fork marta');
    expect(rows[2]).toHaveTextContent('Conflit');
    expect(screen.getByText('3 ouvertes')).toBeInTheDocument();
    const tabs = screen.getByRole('tablist', { name: 'Quelles PR' });
    expect(within(tabs).getByRole('tab', { name: 'Toutes 3' })).toHaveAttribute('aria-selected', 'true');

    await userEvent.click(within(tabs).getByRole('tab', { name: 'Les miennes 1' }));
    expect(screen.getAllByRole('listitem').map((r) => r.textContent)).toEqual([expect.stringContaining('#12')]);
    await userEvent.click(within(tabs).getByRole('tab', { name: 'Relecture demandée 1' }));
    expect(screen.getAllByRole('listitem').map((r) => r.textContent)).toEqual([expect.stringContaining('#11')]);
    expect(localStorage.getItem('escouade.prsFilter')).toBe('review');
  });

  it('starts an agent on the pull request chosen, by its button or by Entrée, and opens it in the browser', async () => {
    const started = agent({ id: 'a9', name: 'pr-10' });
    const backend = fakeBackend({
      project_prs: () => ({ data: { word: 'PR', prs: PRS, me: 'ada' } }),
      create_agent_on_pr: () => started,
    });
    render(PrsView, { project: project() });
    const rows = await screen.findAllByRole('listitem');
    await userEvent.click(within(rows[2]).getByRole('button', { name: 'Ouvrir dans le navigateur' }));
    expect(openUrl).toHaveBeenCalledWith('https://github.com/acme/demo/pull/10');

    await userEvent.click(within(rows[2]).getByRole('button', { name: 'Lancer un agent' }));
    await waitFor(() => expect(backend.called('create_agent_on_pr').map((c) => c.args)).toEqual([{ projectId: 'p1', number: 10 }]));
    expect(app.agents.a9).toBeDefined();
    expect(app.ui.selectedAgent.p1).toBe('a9');

    // The keyboard: the first row chosen, ↓ to the next, Entrée starts the agent.
    app.agents = {};
    const main = within(rows[0]).getByRole('button', { name: 'PR #12 · Pagination par curseur' });
    main.focus();
    await userEvent.keyboard('{ArrowDown}{Enter}');
    await waitFor(() => expect(backend.called('create_agent_on_pr').at(-1)?.args).toEqual({ projectId: 'p1', number: 11 }));
  });

  it('opens the agent already on a pull request instead of starting another', async () => {
    const backend = fakeBackend({ project_prs: () => ({ data: { word: 'PR', prs: PRS, me: 'ada' } }) });
    resetApp({
      projects: [project()],
      agents: [
        agent({ id: 'a1', name: 'curseur', worktree: { path: 'C:\\wt', branch: 'feat/cursor', baseBranch: 'main', existing: true } }),
      ],
    });
    render(PrsView, { project: project() });
    const rows = await screen.findAllByRole('listitem');
    expect(rows[0]).toHaveTextContent('curseur');
    await userEvent.click(within(rows[0]).getByRole('button', { name: 'Ouvrir l’agent' }));
    expect(app.ui.selectedAgent.p1).toBe('a1');
    expect(backend.called('create_agent_on_pr')).toEqual([]);
  });

  it('says MR on GitLab, and why it is empty with a way back to all', async () => {
    fakeBackend({ project_prs: () => ({ data: { word: 'MR', prs: PRS, me: 'zoe' } }) });
    localStorage.setItem('escouade.prsFilter', 'mine');
    render(PrsView, { project: project() });
    expect(await screen.findByRole('heading', { name: 'Merge requests' })).toBeInTheDocument();
    expect(screen.getByText('Aucune MR ouverte par toi.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Toutes les MR' }));
    expect(screen.getAllByRole('listitem')).toHaveLength(3);
    expect(app.prWords.p1).toBe('MR');
  });

  it('says why it could not read them, with « Réessayer », and keeps what it showed on a later failure', async () => {
    let fail = true;
    fakeBackend({
      project_prs: () =>
        fail ? { failure: { kind: 'unreachable', message: '502 Bad Gateway' } } : { data: { word: 'PR', prs: PRS, me: 'ada' } },
    });
    render(PrsView, { project: project() });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Hébergeur injoignable.');
    expect(alert).toHaveTextContent('502 Bad Gateway');
    fail = false;
    await userEvent.click(within(alert).getByRole('button', { name: 'Réessayer' }));
    expect(await screen.findAllByRole('listitem')).toHaveLength(3);
    fail = true;
    await userEvent.click(screen.getByRole('button', { name: 'Rafraîchir' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Hébergeur injoignable.');
    expect(screen.getAllByRole('listitem')).toHaveLength(3);
  });

  it('leads a refused token to the page « Services connectés » of the settings', async () => {
    fakeBackend({ project_prs: () => ({ failure: { kind: 'refused', message: '401 Bad credentials' } }) });
    render(PrsView, { project: project() });
    const alert = await screen.findByRole('alert');
    await userEvent.click(within(alert).getByRole('button', { name: 'Réglages › Services connectés' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'services' });
  });

  it('speaks English', async () => {
    setLang('en');
    fakeBackend({ project_prs: () => ({ data: { word: 'PR', prs: PRS, me: 'ada' } }) });
    render(PrsView, { project: project() });
    const rows = await screen.findAllByRole('listitem');
    expect(within(rows[0]).getByRole('button', { name: 'Start an agent' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Review requested 1' })).toBeInTheDocument();
    expect(rows[1]).toHaveTextContent('Your review');
  });
});
