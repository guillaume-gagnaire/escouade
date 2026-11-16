import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { conversationCount, missingParts, pageFailure, reviewers, threadPlace, timeline } from '../../lib/prpage';
import { app } from '../../lib/state.svelte';
import type { PrContents, PrDetail, UiEvent } from '../../lib/types';
import { agent, fakeBackend, project, pullRequest as pr, resetApp, ticket, ticketPr } from '../../test/ipc';
import PrView from './PrView.svelte';
import PrsView from './PrsView.svelte';

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock('../../lib/opener', () => ({ openUrl }));

const T0 = 1_793_260_800;

const PR = pr({
  number: 214,
  title: 'Connexion par lien magique',
  url: 'https://github.com/acme/demo/pull/214',
  head: 'ticket/atl-42',
  base: 'main',
  author: 'bob',
  created: T0,
  mergeable: true,
  reviewers: ['marta'],
});

const contents = (over: Partial<PrContents> = {}): PrContents => ({
  thirdParty: true,
  word: 'PR',
  pr: PR,
  description: {
    body: 'Connexion sans mot de passe.\n\n![traceur](https://evil.example/pixel.png) [lien](https://evil.example)',
    labels: ['auth', 'sécurité'],
  },
  commits: [
    { sha: 'a1b2c3d4e5', message: 'feat: lien magique\n\nLe lien vaut 10 minutes.', author: 'bob', created: T0 + 60 },
    { sha: '9e8f7a6b5c', message: 'test: lien expiré', author: 'bob', created: T0 + 120 },
    { sha: '3f2a91c000', message: 'fix(auth): durée du lien', author: 'ada', created: T0 + 900 },
  ],
  files: [],
  threads: [
    {
      id: 'PRRT_1',
      path: 'src/auth/magic-link.ts',
      line: 23,
      side: 'right',
      state: 'open',
      comments: [
        { id: '1', author: 'marta', body: 'Compare en temps constant.', created: T0 + 300 },
        { id: '2', author: 'bob', body: 'Corrigé.', created: T0 + 600 },
      ],
    },
    {
      id: 'PRRT_2',
      path: 'src/auth/session.ts',
      line: 10,
      side: 'right',
      state: 'resolved',
      comments: [{ id: '3', author: 'marta', body: 'Le cookie sans sameSite.', created: T0 + 400 }],
    },
    { id: '', side: 'right', state: 'open', comments: [{ id: '9', author: 'marta', body: 'Merci !', created: T0 + 1000 }] },
  ],
  reading: { description: true, commits: true, files: true, threads: true },
  diff: false,
  readAt: T0 * 1000,
  checkedAt: T0 * 1000,
  ...over,
});

const DETAIL: PrDetail = {
  checks: [
    { name: 'lint', state: 'success', url: null, job: null, started: null, finished: null, cancelled: false },
    { name: 'e2e', state: 'failure', url: null, job: '77', started: null, finished: null, cancelled: false },
  ],
  reviews: [{ author: 'marta', state: 'changesRequested', body: 'Deux points de sécurité.' }],
  comments: [],
  deployments: [],
  can: { checks: true, jobLogs: true, rerun: true, reviews: true, deployments: false },
};

const emit = (e: UiEvent) => (app as unknown as { onEvent: (e: UiEvent) => void }).onEvent(e);

beforeEach(() => {
  setLang('fr');
  openUrl.mockClear();
  resetApp({ projects: [project()] });
  app.now = (T0 + 3600) * 1000;
});

describe('what the view says (lib/prpage)', () => {
  it('orders the timeline by time, the commits of an author together, the reviews after', () => {
    const items = timeline(contents(), DETAIL);
    expect(items.map((x) => x.kind)).toEqual(['commits', 'thread', 'thread', 'commits', 'comment', 'review']);
    expect(items[0].kind === 'commits' && items[0].commits.length).toBe(2);
    expect(conversationCount(contents())).toBe(4);
  });

  it('says a reviewer’s last word, those asked, a thread’s place and what the host does not give', () => {
    expect(
      reviewers(PR, {
        ...DETAIL,
        reviews: [
          { author: 'eve', state: 'approved', body: '' },
          { author: 'eve', state: 'commented', body: 'Et ici ?' },
        ],
      }),
    ).toEqual([
      { name: 'eve', state: 'approved' },
      { name: 'marta', state: 'requested' },
    ]);
    expect(threadPlace({ id: 'x', path: 'a.ts', line: 12, startLine: 10, side: 'right', state: 'open', comments: [] })).toBe('a.ts:10-12');
    expect(missingParts(contents({ reading: { description: false, commits: true, files: false, threads: false } }))).toEqual([
      'la description',
      'les fils',
    ]);
    expect(pageFailure({ kind: 'other', message: 'Not Found' }, 'PR', 9)).toBe('PR #9 introuvable chez l’hébergeur.');
    expect(pageFailure({ kind: 'unreachable', message: 'x' }, 'PR', 9, false)).toBe('Hors ligne : la PR se relira au retour du réseau.');
    expect(pageFailure({ kind: 'unreachable', message: 'x' }, 'PR', 9)).toBe('Hébergeur injoignable.');
  });
});

describe('the view of a pull request (ESC-457)', () => {
  it('shows its header, its description as a third party’s, its timeline with its threads and their state', async () => {
    resetApp({
      projects: [project()],
      agents: [agent({ id: 'a1', name: 'Lien magique', prs: [PR] })],
      tickets: [ticket({ key: 'ATL-42', column: 'pr', pr: ticketPr({ number: 214, namespace: 'acme', name: 'demo' }) })],
    });
    app.codeHosts.p1 = { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: false };
    fakeBackend({ pr_contents: () => ({ data: contents() }), pr_detail: () => ({ data: DETAIL }) });
    render(PrView, { project: project(), number: 214 });

    const main = await screen.findByRole('main', { name: 'La PR #214' });
    expect(await within(main).findByRole('heading', { name: /#214 ?Connexion par lien magique/ })).toBeInTheDocument();
    expect(main).toHaveTextContent('Ouverte');
    expect(main).toHaveTextContent('ticket/atl-42→main');
    expect(within(main).getByRole('button', { name: 'ticket ATL-42' })).toBeInTheDocument();
    expect(main).toHaveTextContent('par l’agent Lien magique');
    expect(main).toHaveTextContent('3 commits');
    // The tabs and their counts; Fichiers open (ESC-467).
    const tabs = within(main).getByRole('tablist', { name: 'Onglets de la PR' });
    expect(within(tabs).getByRole('tab', { name: 'Conversation 4' })).toHaveAttribute('aria-selected', 'true');
    expect(within(tabs).getByRole('tab', { name: 'Commits 3' })).toBeInTheDocument();
    await waitFor(() => expect(within(tabs).getByRole('tab', { name: 'Contrôles 1/2' })).toBeInTheDocument());
    expect(within(tabs).getByRole('tab', { name: /^Fichiers/ })).not.toHaveAttribute('aria-disabled');

    // The description: marked as a third party's, its image not loaded and no link of its own.
    const desc = within(main).getByRole('region', { name: 'contenu d’un tiers' });
    expect(desc).toHaveTextContent('Connexion sans mot de passe.');
    expect(desc.querySelector('img')).toBeNull();
    expect(desc).toHaveTextContent('Afficher l’image');

    // The timeline: the pushes, the threads with their place and state (the resolved folded), the review.
    expect(main).toHaveTextContent('a poussé 2 commits');
    const open = within(main).getByRole('article', { name: 'src/auth/magic-link.ts:23' });
    expect(open).toHaveTextContent('ouvert');
    expect(open).toHaveTextContent('Compare en temps constant.');
    expect(open).toHaveTextContent('Corrigé.');
    const folded = within(main).getByRole('button', { name: /résolu src\/auth\/session\.ts:10/ });
    expect(folded).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(folded);
    expect(within(main).getByRole('article', { name: 'src/auth/session.ts:10' })).toHaveTextContent('Le cookie sans sameSite.');
    await waitFor(() => expect(main).toHaveTextContent('a demandé des changements'));
    expect(main).toHaveTextContent('Deux points de sécurité.');

    // The aside: reviewers, labels, where it stands.
    const aside = main.querySelector('aside')!;
    expect(aside).toHaveTextContent('MAmarta changements');
    expect(aside).toHaveTextContent('auth');
    expect(aside).toHaveTextContent('Sans conflit avec main');
    expect(aside).toHaveTextContent('ATL-42 · En revue');

    // « Ouvrir sur GitHub » stays a button: the page in the browser.
    await userEvent.click(within(main).getByRole('button', { name: 'Ouvrir sur GitHub' }));
    expect(openUrl).toHaveBeenCalledWith('https://github.com/acme/demo/pull/214');
  });

  it('lists the commits, a click shows the whole message; the checks, their log and their rerun', async () => {
    const backend = fakeBackend({
      pr_contents: () => ({ data: contents() }),
      pr_detail: () => ({ data: DETAIL }),
      pr_log: () => ({ data: 'FAIL login.expired' }),
      pr_rerun: () => 1,
    });
    render(PrView, { project: project(), number: 214 });
    const main = await screen.findByRole('main', { name: 'La PR #214' });
    await userEvent.click(await within(main).findByRole('tab', { name: 'Commits 3' }));
    const list = within(main).getByRole('list', { name: 'Commits de la PR' });
    expect(within(list).getAllByRole('listitem')).toHaveLength(3);
    expect(list).toHaveTextContent('feat: lien magique');
    expect(list).toHaveTextContent('a1b2c3d');
    expect(list).not.toHaveTextContent('Le lien vaut 10 minutes.');
    await userEvent.click(within(list).getByRole('button', { name: 'Afficher le message entier' }));
    expect(list).toHaveTextContent('Le lien vaut 10 minutes.');

    await userEvent.click(within(main).getByRole('tab', { name: 'Contrôles 1/2' }));
    await userEvent.click(await within(main).findByRole('button', { name: /e2e/ }));
    await waitFor(() => expect(main).toHaveTextContent('FAIL login.expired'));
    expect(backend.called('pr_log').map((c) => c.args)).toEqual([{ projectId: 'p1', number: 214, job: '77' }]);
    await userEvent.click(within(main).getByRole('button', { name: /Relancer/ }));
    await waitFor(() => expect(backend.called('pr_rerun').map((c) => c.args)).toEqual([{ projectId: 'p1', number: 214 }]));
    // Nothing else written: the view reads.
    expect(
      backend.calls.map((c) => c.cmd).filter((c) => !['pr_contents', 'pr_detail', 'pr_log', 'pr_rerun', 'pr_reviews'].includes(c)),
    ).toEqual([]);
  });

  it('reads again when the host says it changed, and says what the host does not give', async () => {
    let body = 'Avant.';
    const backend = fakeBackend({
      pr_contents: () => ({
        data: contents({
          description: { body },
          reading: { description: true, commits: false, files: false, threads: false },
          commits: [],
          threads: [],
        }),
      }),
      pr_detail: () => ({ data: { ...DETAIL, checks: [] } }),
    });
    render(PrView, { project: project(), number: 214 });
    const main = await screen.findByRole('main', { name: 'La PR #214' });
    await waitFor(() => expect(main).toHaveTextContent('Avant.'));
    expect(within(main).getByRole('note')).toHaveTextContent('l’hébergeur ne donne pas à Escouade les commits et les fils.');
    body = 'Après.';
    // Another pull request changed: not read again.
    emit({ type: 'prChanged', projectId: 'p1', number: 9 });
    emit({ type: 'prChanged', projectId: 'p1', number: 214 });
    await waitFor(() => expect(main).toHaveTextContent('Après.'));
    expect(backend.called('pr_contents')).toHaveLength(2);
  });

  it('says a pull request not found, a refused token and the host unreachable, with « Réessayer »', async () => {
    let failure = { kind: 'other', message: 'Not Found' };
    fakeBackend({ pr_contents: () => ({ failure }), pr_detail: () => ({ failure }) });
    render(PrView, { project: project(), number: 9 });
    expect(await screen.findByRole('alert')).toHaveTextContent('PR #9 introuvable chez l’hébergeur.');
    failure = { kind: 'refused', message: 'Bad credentials' };
    await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Jeton refusé par l’hébergeur'));
    expect(screen.getByRole('button', { name: 'Réglages › Services connectés' })).toBeInTheDocument();
    failure = { kind: 'unreachable', message: 'timeout' };
    await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Hébergeur injoignable.'));
  });
});

describe('opening the view', () => {
  it('a click on a pull request of the list opens its view, the back button the list again', async () => {
    fakeBackend({
      project_prs: () => ({ data: { word: 'PR', prs: [PR], me: 'ada' } }),
      pr_contents: () => ({ data: contents() }),
      pr_detail: () => ({ data: DETAIL }),
    });
    app.prs.p1 = true;
    render(PrsView, { project: project() });
    await userEvent.click(await screen.findByRole('button', { name: /PR #214/ }));
    expect(openUrl).not.toHaveBeenCalled();
    const main = await screen.findByRole('main', { name: 'La PR #214' });
    await userEvent.click(within(main).getByRole('button', { name: 'Retour aux PR' }));
    expect(await screen.findByRole('heading', { name: 'Pull requests' })).toBeInTheDocument();
  });

  it('openPrView shows it in the project’s « Pull requests », and a review opens it on its tab « Fichiers »', () => {
    app.openPrView('p1', { number: 214, hostRepo: 'acme/up' });
    expect(app.prOpen.p1).toEqual({ number: 214, hostRepo: 'acme/up' });
    expect(app.prsOn).toBe(true);
    app.showPrReview('p1', 214);
    expect(app.prOpen.p1).toEqual({ number: 214, tab: 'files' });
  });
});
