import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { checkDuration, checkPhase, ciSummary, deployState, sortChecks } from '../lib/prview';
import { app } from '../lib/state.svelte';
import type { PrCheck, PrDetail } from '../lib/types';
import { agent, fakeBackend, project, pullRequest as pr, resetApp } from '../test/ipc';
import PrPanel from './PrPanel.svelte';
import SidePanel from './SidePanel.svelte';

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock('../lib/opener', () => ({ openUrl }));

const WT = { path: 'C:\\wt\\login', branch: 'escouade/login', baseBranch: 'main', existing: false };
const NOW = 1_793_260_800;

const check = (over: Partial<PrCheck>): PrCheck => ({
  name: 'test',
  state: 'success',
  url: null,
  job: null,
  started: null,
  finished: null,
  cancelled: false,
  ...over,
});

const DETAIL: PrDetail = {
  checks: [
    check({ name: 'lint', started: NOW - 100, finished: NOW - 90 }),
    check({
      name: 'test',
      state: 'failure',
      job: '77',
      url: 'https://github.com/acme/demo/actions/runs/7/job/77',
      started: NOW - 300,
      finished: NOW - 150,
    }),
    check({ name: 'e2e', state: 'failure', cancelled: true, started: NOW - 300, finished: NOW - 290 }),
    check({ name: 'build', state: 'pending' }),
  ],
  reviews: [
    { author: 'bob', state: 'changesRequested', body: 'Ajoute un test.' },
    { author: 'eve', state: 'approved', body: '' },
    { author: 'zed', state: 'commented', body: '' },
  ],
  comments: [
    { id: '1', author: 'bob', body: 'Ce nom est peu clair.', path: 'src/app.ts', line: 12, resolved: false },
    { id: '2', author: 'eve', body: 'Réglé.', path: null, line: null, resolved: true },
  ],
  deployments: [{ environment: 'Preview', state: 'success', url: 'https://demo-git-login.vercel.app' }],
  can: { checks: true, jobLogs: true, rerun: true, reviews: true, deployments: true },
};

const open = (over = {}) =>
  pr({
    number: 7,
    title: 'Connexion par jeton',
    head: 'escouade/login',
    base: 'main',
    checks: 'failure',
    mergeable: false,
    author: 'ada',
    created: NOW - 7200,
    ...over,
  });

describe('the checks of a pull request', () => {
  it('say their phase, their order and how long they ran', () => {
    expect(checkPhase(check({ state: 'pending' }))).toBe('queued');
    expect(checkPhase(check({ state: 'pending', started: 1 }))).toBe('running');
    expect(checkPhase(check({ state: 'failure', cancelled: true }))).toBe('cancelled');
    expect(sortChecks(DETAIL.checks).map((c) => c.name)).toEqual(['test', 'build', 'e2e', 'lint']);
    expect(checkDuration(DETAIL.checks[1], NOW * 1000)).toBe('2m 30s');
    expect(checkDuration(check({ state: 'pending', started: NOW - 65 }), NOW * 1000)).toBe('1m 05s');
    expect(checkDuration(check({}), NOW * 1000)).toBeNull();
    expect(ciSummary(DETAIL.checks)).toBe('1 échec sur 4');
    expect(ciSummary([check({}), check({ state: 'pending', started: 1 })])).toBe('1 en cours sur 2');
    expect(ciSummary([check({}), check({})])).toBe('2 vérifications réussies');
    expect(ciSummary([check({}), check({ state: 'failure', cancelled: true })])).toBe('1 vérification réussie · 1 annulée');
  });
});

it('says a deployment’s state in words, another one as the host says it', () => {
  expect([deployState('success'), deployState('in_progress'), deployState('inactive'), deployState('waiting')]).toEqual([
    'déployé',
    'en cours',
    'remplacé',
    'waiting',
  ]);
});

describe('the « PR et CI » tab', () => {
  beforeEach(() => {
    resetApp({
      projects: [project()],
      agents: [agent({ worktree: WT, prWord: 'PR', prs: [open(), pr({ number: 3, state: 'merged', title: 'Ancienne' })] })],
    });
    app.now = NOW * 1000;
  });
  afterEach(() => {
    setLang('fr');
    vi.useRealTimers();
  });

  it('has no waiting text any more, and a dot when the CI of the agent’s pull request failed', async () => {
    fakeBackend({ git_files: () => [], agent_pr_detail: () => ({ data: DETAIL }) });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    const tab = screen.getByRole('tab', { name: /^PR et CI/ });
    expect(tab.textContent?.trim()).toBe('PR et CI');
    expect(within(tab).getByRole('img', { name: 'La CI de la PR #7 a échoué' })).toBeInTheDocument();
    await userEvent.click(tab);
    expect(await screen.findByText('Connexion par jeton')).toBeInTheDocument();
    expect(screen.queryByText(/dans la 2\.1/)).toBeNull();
  });

  it('shows the agent’s pull request: state, branch → target, conflict, age and author, then its CI, reviews and deployments', async () => {
    const backend = fakeBackend({ agent_pr_detail: () => ({ data: DETAIL }) });
    render(PrPanel, { project: project(), agent: app.agents.a1 });
    const head = screen.getByRole('region', { name: 'PR #7 · Connexion par jeton' });
    expect(head).toHaveTextContent('ouverte');
    expect(head).toHaveTextContent('escouade/login → main');
    expect(head).toHaveTextContent('Conflit avec main');
    expect(head).toHaveTextContent('créée il y a 2 h par ada');
    expect(within(head).getByRole('button', { name: 'L’agent corrige' })).toBeInTheDocument();
    expect(backend.called('agent_pr_detail').map((c) => c.args)).toEqual([{ id: 'a1', number: 7 }]);

    const ci = await screen.findByRole('region', { name: 'CI' });
    expect(ci).toHaveTextContent('1 échec sur 4');
    const rows = within(ci).getAllByRole('listitem');
    expect(rows.map((r) => r.getAttribute('data-check'))).toEqual(['test', 'build', 'e2e', 'lint']);
    expect(rows[0]).toHaveTextContent('échouée');
    expect(rows[0]).toHaveTextContent('2m 30s');
    expect(rows[1]).toHaveTextContent('en attente');
    expect(rows[2]).toHaveTextContent('annulée');
    expect(rows[3]).toHaveTextContent('réussie');

    const reviews = screen.getByRole('region', { name: 'Revues et commentaires' });
    expect(reviews).toHaveTextContent('bob demande des changements');
    expect(reviews).toHaveTextContent('eve a approuvé');
    expect(reviews).not.toHaveTextContent('zed');
    expect(reviews).toHaveTextContent('src/app.ts:12');
    expect(reviews).toHaveTextContent('En lecture : Escouade ne publie rien en ton nom.');
    // Nothing to answer with: no field, no reply.
    expect(within(reviews).queryByRole('textbox')).toBeNull();
    expect(within(reviews).queryByRole('button')).toBeNull();

    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir https://demo-git-login.vercel.app' }));
    expect(openUrl).toHaveBeenCalledWith('https://demo-git-login.vercel.app');
    // Its other pull requests under it.
    expect(screen.getByRole('region', { name: 'Ses autres PR' })).toHaveTextContent('#3');
    // Nothing was written: no correction, no re-run.
    expect(backend.called('agent_pr_fix')).toHaveLength(0);
    expect(backend.called('agent_pr_rerun')).toHaveLength(0);
  });

  it('opens a failed check on the end of its log, and re-runs the failures on a click only', async () => {
    const backend = fakeBackend({
      agent_pr_detail: () => ({ data: DETAIL }),
      agent_pr_log: () => ({ data: '…\nnpm test\nFAIL src/app.test.ts' }),
      agent_pr_rerun: () => 1,
    });
    render(PrPanel, { project: project(), agent: app.agents.a1 });
    const name = await screen.findByRole('button', { name: 'test' });
    expect(name).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(name);
    const log = await screen.findByRole('listitem', { name: 'Journal de test' });
    await waitFor(() => expect(log).toHaveTextContent('FAIL src/app.test.ts'));
    expect(backend.called('agent_pr_log').map((c) => c.args)).toEqual([{ id: 'a1', number: 7, job: '77' }]);
    expect(log.querySelector('pre')).not.toBeNull();
    // A cancelled one has no log to open.
    expect(screen.queryByRole('button', { name: 'e2e' })).toBeNull();

    expect(backend.called('agent_pr_rerun')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Relancer les échecs' }));
    await waitFor(() => expect(backend.called('agent_pr_rerun').map((c) => c.args)).toEqual([{ id: 'a1', number: 7 }]));
    await waitFor(() => expect(app.toasts.map((x) => x.text)).toContain('1 exécution relancée.'));
  });

  it('reads the checks again every 20 s while one runs, and no more once none does', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let running = true;
    const backend = fakeBackend({
      agent_pr_detail: () => ({
        data: { ...DETAIL, checks: [check({ name: 'build', state: running ? 'pending' : 'success', started: NOW })] },
      }),
    });
    render(PrPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByRole('region', { name: 'CI' });
    expect(backend.called('agent_pr_detail')).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(19_000);
    expect(backend.called('agent_pr_detail')).toHaveLength(1);
    running = false;
    await vi.advanceTimersByTimeAsync(1_500);
    expect(backend.called('agent_pr_detail')).toHaveLength(2);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(backend.called('agent_pr_detail')).toHaveLength(2);
  });

  it('draws the host’s errors: a refused token with the way to the settings, the limit with its end', async () => {
    let failure: object = { kind: 'refused', message: 'GitHub : jeton refusé (401)' };
    fakeBackend({ agent_pr_detail: () => ({ failure }) });
    const { unmount } = render(PrPanel, { project: project(), agent: app.agents.a1 });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Jeton refusé par l’hébergeur : reconnecte son compte.');
    await userEvent.click(within(alert).getByRole('button', { name: 'Réglages › Intégrations' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'integrations' });
    unmount();
    failure = { kind: 'rateLimit', message: 'GitHub : limite atteinte (403)', reset: NOW + 3600 };
    render(PrPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByRole('alert')).toHaveTextContent(/Limite d’API de l’hébergeur atteinte, jusqu’à \d\d:\d\d\./);
    expect(screen.getByRole('button', { name: 'Réessayer' })).toBeInTheDocument();
  });

  it('offers « Ouvrir une PR… » without one, or says why not', async () => {
    app.agents.a1.prs = [];
    const backend = fakeBackend({ agent_no_pr: () => ({ reason: 'Aucun compte connecté pour github.com', settings: true }) });
    const { unmount } = render(PrPanel, { project: project(), agent: app.agents.a1 });
    expect(screen.getByText('Pas de PR sur cette branche')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir une PR…' }));
    expect(app.modal).toEqual({ kind: 'pullRequest', agentId: 'a1' });
    expect(backend.called('agent_no_pr')).toHaveLength(0);
    unmount();
    app.agents.a1.prWord = null;
    render(PrPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('Aucun compte connecté pour github.com')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Réglages › Intégrations' })).toBeInTheDocument();
  });

  it('lists the project’s open pull requests without an agent, an agent’s opening it', async () => {
    resetApp({ projects: [project()], agents: [agent({ worktree: WT, prWord: 'PR' })] });
    const backend = fakeBackend({
      project_prs: () => ({
        data: {
          word: 'PR',
          prs: [
            open({ number: 12, title: 'Mode sombre', head: 'escouade/login', checks: 'success', mergeable: true }),
            open({
              number: 13,
              url: 'https://github.com/acme/demo/pull/13',
              title: 'Lien du footer',
              head: 'bob/footer',
              checks: 'pending',
            }),
          ],
        },
      }),
    });
    render(PrPanel, { project: project(), agent: null });
    const list = await screen.findByRole('region', { name: 'PR ouvertes du projet' });
    expect(await within(list).findAllByRole('listitem')).toHaveLength(2);
    expect(list).toHaveTextContent('CI verte');
    expect(list).toHaveTextContent('CI en cours');
    await userEvent.click(within(list).getByRole('button', { name: 'Ouvrir l’agent refacto-auth' }));
    expect(app.ui.selectedAgent.p1).toBe('a1');
    await userEvent.click(within(list).getByRole('button', { name: 'Ouvrir la PR #13 dans le navigateur (ouverte)' }));
    expect(openUrl).toHaveBeenCalledWith('https://github.com/acme/demo/pull/13');
    expect(backend.called('project_prs').map((c) => c.args)).toEqual([{ projectId: 'p1' }]);
  });

  it('says why the project has none to list', async () => {
    fakeBackend({
      project_prs: () => ({ failure: { kind: 'unavailable', message: 'Ce projet n’a pas de dépôt distant chez un hébergeur de code.' } }),
    });
    render(PrPanel, { project: project(), agent: null });
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Ce projet n’a pas de dépôt distant');
    expect(within(alert).queryByRole('button')).toBeNull();
  });

  it('speaks English', async () => {
    setLang('en');
    fakeBackend({ agent_pr_detail: () => ({ data: DETAIL }) });
    render(PrPanel, { project: project(), agent: app.agents.a1 });
    expect(screen.getByText('Conflicts with main')).toBeInTheDocument();
    expect(await screen.findByText('1 failure out of 4')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Re-run failed jobs' })).toBeInTheDocument();
    expect(screen.getByText('Read only: Escouade publishes nothing in your name.')).toBeInTheDocument();
  });
});
