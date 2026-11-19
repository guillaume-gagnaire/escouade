import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { PendingChange, ReviewAsk } from '../../lib/myreview';
import type { PrWriting } from '../../lib/prthreads';
import { app } from '../../lib/state.svelte';
import type { PendingNote, PendingReview, PrContents, PrContentsFile, PrFailure, TicketPr } from '../../lib/types';
import { agent, fakeBackend, project, pullRequest, resetApp, ticket } from '../../test/ipc';
import PrView from './PrView.svelte';

// « Ta relecture » in the view of a pull request (ESC-494, PR-7): a remark added under its line from « + », kept waiting
// (« en attente · pas publiée »), edited and taken out; the header's button with its count; its panel with the text, the
// remarks (a click goes to the line, a stale one said), the verdicts the host gives (a comment alone on the user's own
// pull request), the case for the ticket's agent, « Publier » the only gesture that sends, a refusal that keeps it all, and
// « Abandonner ma relecture » after a confirmation.

const GITHUB: PrWriting = {
  comment: true,
  lineComment: true,
  reply: true,
  resolve: true,
  reopen: true,
  review: true,
  approve: true,
  requestChanges: true,
};

const LINK: PrContentsFile = {
  path: 'src/auth/magic-link.ts',
  status: 'modified',
  additions: 2,
  deletions: 1,
  lines: [
    { kind: '@', text: '@@ -1,3 +1,4 @@' },
    { kind: ' ', old: 1, new: 1, text: 'export const ttl = 10;' },
    { kind: '-', old: 2, text: 'return mailer.send(email);' },
    { kind: '+', new: 2, text: 'await mailer.send(email, { locale });' },
    { kind: '+', new: 3, text: 'return { sent: true };' },
    { kind: ' ', old: 3, new: 4, text: '}' },
  ],
  binary: false,
  generated: false,
  cut: false,
};

const PR = (author = 'bob') =>
  pullRequest({ number: 214, title: 'Connexion par lien magique', head: 'ticket/atl-42', base: 'main', author });

const contents = (author = 'bob', writing: PrWriting = GITHUB): PrContents => ({
  thirdParty: true,
  word: 'PR',
  pr: PR(author),
  commits: [],
  files: [LINK],
  threads: [],
  reading: { description: true, commits: true, files: true, threads: true },
  writing,
  diff: true,
  tip: 'a1b2c3d',
  readAt: 0,
  checkedAt: 0,
});

/** A backend that keeps the review waiting as the core does, and publishes it as `submit` says. */
function backend(o: { own?: boolean; submit?: (ask: ReviewAsk) => unknown; start?: PendingNote[]; writing?: PrWriting } = {}) {
  let review: PendingReview = { projectId: 'p1', number: 214, body: '', notes: o.start ?? [], updatedAt: 0 };
  const read = () => ({ review: structuredClone(review), own: !!o.own });
  const b = fakeBackend({
    pr_contents: () => ({ data: contents(o.own ? 'ada' : 'bob', o.writing ?? GITHUB) }),
    pr_detail: () => ({ data: null }),
    pr_control: () => ({ failure: { kind: 'other', message: 'non' } }),
    pr_pending: () => read(),
    pr_pending_change: ({ change }: { change: PendingChange }) => {
      const next = review.notes.reduce((m, n) => Math.max(m, n.id), 0) + 1;
      if (change.kind === 'add')
        review.notes.push({ id: next, path: change.path, line: change.line, side: change.side, body: change.body, code: [], created: 0 });
      if (change.kind === 'edit') review.notes = review.notes.map((n) => (n.id === change.id ? { ...n, body: change.body } : n));
      if (change.kind === 'remove') review.notes = review.notes.filter((n) => n.id !== change.id);
      if (change.kind === 'body') review.body = change.body;
      if (change.kind === 'discard') review = { ...review, body: '', notes: [] };
      return read();
    },
    pr_review_submit: ({ ask }: { ask: ReviewAsk }) => {
      const out = o.submit ? o.submit(ask) : { data: { lines: review.notes.length, stale: 0 } };
      if ((out as { data?: unknown }).data) review = { ...review, body: '', notes: [] };
      return out;
    },
  });
  return b;
}

const sent = (b: ReturnType<typeof fakeBackend>) => b.calls.filter((c) => ['pr_write', 'pr_review_submit', 'pr_merge'].includes(c.cmd));

async function view() {
  const main = await screen.findByRole('main', { name: 'La PR #214' });
  return main;
}

async function files(main: HTMLElement) {
  await userEvent.click(await within(main).findByRole('tab', { name: /^Fichiers/ }));
  return within(main).getByRole('region', { name: LINK.path });
}

async function panel(main: HTMLElement) {
  await userEvent.click(within(main).getByRole('button', { name: /^Ta relecture/ }));
  return within(main).getByRole('dialog', { name: 'Ta relecture de la PR #214' });
}

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project()] });
  app.codeHosts = { p1: { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true } };
});

describe('« Ta relecture » (ESC-494)', () => {
  it('keeps a remark added under its line waiting, edited and taken out, nothing sent', async () => {
    const b = backend();
    render(PrView, { project: project(), number: 214 });
    const main = await view();
    const link = await files(main);
    await userEvent.click(within(link).getByRole('button', { name: 'Commenter la ligne 3' }));
    // Both ways, the review's first once begun.
    expect(within(link).getByRole('button', { name: 'Commenter maintenant' })).toBeInTheDocument();
    await userEvent.type(within(link).getByRole('textbox', { name: /ligne 3/ }), 'Un **test** ici ?');
    await userEvent.click(within(link).getByRole('button', { name: 'Ajouter à ma relecture' }));
    await waitFor(() =>
      expect(b.called('pr_pending_change').map((c) => c.args.change)).toEqual([
        { kind: 'add', path: LINK.path, line: 3, side: 'right', body: 'Un **test** ici ?' },
      ]),
    );
    const mine = await within(link).findByRole('article', { name: `Fil sur ${LINK.path}:3` });
    expect(mine).toHaveTextContent('Toi');
    expect(mine).toHaveTextContent('en attente · pas publiée');
    expect(within(mine).getByText('test').tagName).toBe('STRONG');
    // Under its line.
    const row = link.querySelector('.row[data-new="3"]')!;
    expect(row.nextElementSibling?.contains(mine)).toBe(true);
    expect(within(main).getByRole('button', { name: 'Ta relecture, 1 remarque en attente' })).toHaveTextContent('1');

    await userEvent.click(within(mine).getByRole('button', { name: 'Modifier' }));
    const box = within(mine).getByRole('textbox', { name: 'Modifier' });
    await userEvent.clear(box);
    await userEvent.type(box, 'Un test ici.');
    await userEvent.click(within(mine).getByRole('button', { name: 'Enregistrer' }));
    await waitFor(() => expect(mine).toHaveTextContent('Un test ici.'));

    await userEvent.click(within(mine).getByRole('button', { name: 'Retirer' }));
    await waitFor(() => expect(within(link).queryByRole('article', { name: `Fil sur ${LINK.path}:3` })).toBeNull());
    expect(sent(b)).toEqual([]);
  });

  it('publishes it all in one review on its click, with the verdict, the text and the case for the ticket’s agent', async () => {
    const tp = { number: 214, namespace: 'acme', name: 'demo' } as TicketPr;
    resetApp({
      projects: [project()],
      tickets: [ticket({ id: 't9', key: 'ATL-42', column: 'pr', agentId: 'a1', pr: tp })],
      agents: [agent({ id: 'a1', name: 'Lien magique' })],
    });
    app.codeHosts = { p1: { kind: 'github', name: 'GitHub', word: 'PR', pullRequests: true, deployments: true } };
    const b = backend({
      start: [{ id: 1, path: LINK.path, line: 3, side: 'right', body: 'Un test ici ?', code: [], created: 0 }],
      submit: () => ({ data: { lines: 1, stale: 0, agentTicket: 'ATL-42' } }),
    });
    render(PrView, { project: project(), number: 214 });
    const main = await view();
    const p = await panel(main);
    expect(p).toHaveTextContent('1 remarque en attente');
    expect(p).toHaveTextContent(`${LINK.path}:3`);
    const radios = within(p).getAllByRole('radio');
    expect(radios.map((r) => (r as HTMLInputElement).disabled)).toEqual([false, false, false]);

    // Changes asked: a text is needed, the case for the agent checked.
    await userEvent.click(within(p).getByRole('radio', { name: /Demander des changements/ }));
    expect(within(p).getByRole('button', { name: 'Publier sur GitHub' })).toBeDisabled();
    expect(p).toHaveTextContent('Écris le texte de ta relecture pour demander des changements.');
    expect(within(p).getByRole('checkbox', { name: 'Renvoyer aussi à l’agent du ticket' })).toBeChecked();
    await userEvent.type(within(p).getByRole('textbox', { name: 'Texte de ta relecture' }), 'Deux points.');
    expect(sent(b)).toEqual([]);

    await userEvent.click(within(p).getByRole('button', { name: 'Publier sur GitHub' }));
    await waitFor(() =>
      expect(b.called('pr_review_submit').map((c) => c.args)).toEqual([
        { projectId: 'p1', number: 214, ask: { verdict: 'changesRequested', body: 'Deux points.', toAgent: true } },
      ]),
    );
    await waitFor(() => expect(within(main).queryByRole('dialog')).toBeNull());
    expect(app.toasts.at(-1)?.text).toBe('Ta relecture est publiée sur GitHub, et l’agent de ATL-42 la reçoit.');
    expect(within(main).queryByRole('button', { name: /remarque en attente/ })).toBeNull();
  });

  it('offers only a comment on the user’s own pull request, and keeps it all when the host refuses', async () => {
    const fail: PrFailure = { kind: 'forbidden', message: 'Resource not accessible' };
    const b = backend({
      own: true,
      start: [{ id: 1, path: LINK.path, line: 9, side: 'right', body: 'Ce bloc ?', code: [], created: 0, stale: true }],
      submit: () => ({ failure: fail }),
    });
    render(PrView, { project: project(), number: 214 });
    const main = await view();
    const p = await panel(main);
    expect(within(p).getByRole('radio', { name: /Approuver/ })).toBeDisabled();
    expect(within(p).getByRole('radio', { name: /Demander des changements/ })).toBeDisabled();
    expect(p).toHaveTextContent('GitHub ne permet que de commenter ta propre PR : ni approuver, ni demander des changements.');
    // A remark whose line is gone: said, it goes in the text.
    expect(p).toHaveTextContent('dépassée');
    expect(p).toHaveTextContent('elle partira dans le texte de ta relecture');
    await userEvent.click(within(p).getByRole('button', { name: 'Publier sur GitHub' }));
    expect(await within(p).findByRole('alert')).toHaveTextContent(
      'Ton jeton GitHub n’a pas le droit d’écrire sur ce dépôt. Rien n’est parti, ta relecture reste en attente.',
    );
    expect(b.called('pr_review_submit')[0].args.ask.verdict).toBe('commented');
    expect(within(main).getByRole('button', { name: 'Ta relecture, 1 remarque en attente' })).toBeInTheDocument();
  });

  it('gives it up after a confirmation, and says a host that publishes no review', async () => {
    const b = backend({ start: [{ id: 1, path: LINK.path, line: 3, side: 'right', body: 'x', code: [], created: 0 }] });
    render(PrView, { project: project(), number: 214 });
    const main = await view();
    let p = await panel(main);
    await userEvent.click(within(p).getByRole('button', { name: 'Abandonner ma relecture' }));
    const confirm = within(p).getByRole('alertdialog');
    expect(confirm).toHaveTextContent('Abandonner ta relecture ? Son texte et sa remarque sont perdus.');
    await userEvent.click(within(confirm).getByRole('button', { name: 'Garder' }));
    expect(b.called('pr_pending_change')).toEqual([]);
    await userEvent.click(within(p).getByRole('button', { name: 'Abandonner ma relecture' }));
    await userEvent.click(within(p).getByRole('button', { name: 'Abandonner' }));
    await waitFor(() => expect(b.called('pr_pending_change').map((c) => c.args.change)).toEqual([{ kind: 'discard' }]));
    await waitFor(() => expect(within(main).queryByRole('dialog')).toBeNull());
    p = await panel(main);
    expect(p).toHaveTextContent('Aucune remarque en attente');
    expect(sent(b)).toEqual([]);
  });

  it('shows no « Ta relecture » nor « Ajouter à ma relecture » for a host that publishes no review', async () => {
    backend({ writing: { ...GITHUB, review: false } });
    render(PrView, { project: project(), number: 214 });
    const main = await view();
    const link = await files(main);
    expect(within(main).queryByRole('button', { name: /^Ta relecture/ })).toBeNull();
    await userEvent.click(within(link).getByRole('button', { name: 'Commenter la ligne 3' }));
    expect(within(link).queryByRole('button', { name: 'Ajouter à ma relecture' })).toBeNull();
    expect(within(link).getByRole('button', { name: 'Commenter' })).toBeInTheDocument();
  });
});
