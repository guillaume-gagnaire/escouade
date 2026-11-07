import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { publishBlock, reviewMark, tally } from '../../lib/prreview';
import { app } from '../../lib/state.svelte';
import type { AgentReview, ReviewNote, UiEvent } from '../../lib/types';
import { fakeBackend, project, pullRequest as pr, resetApp } from '../../test/ipc';
import PrReviewViewHost from './PrReviewView.host.test.svelte';
import PrsView from './PrsView.svelte';

function note(id: number, over: Partial<ReviewNote> = {}): ReviewNote {
  return {
    id,
    path: 'src/auth.ts',
    line: 41 + id,
    body: `Commentaire ${id}`,
    proposed: `Commentaire ${id}`,
    choice: 'open',
    excerpt: [
      { kind: ' ', line: 40 + id, text: 'const a = 1;' },
      { kind: '+', line: 41 + id, text: 'store.set(email, token);' },
    ],
    outside: false,
    ...over,
  };
}

function review(over: Partial<AgentReview> = {}): AgentReview {
  return {
    projectId: 'p1',
    number: 12,
    word: 'PR',
    host: 'GitHub',
    author: 'bob',
    title: 'Connexion par lien',
    head: 'feat/lien',
    base: 'main',
    url: 'https://github.com/acme/demo/pull/12',
    headSha: 'abc',
    phase: 'ready',
    summary: 'Le jeton n’expire pas.',
    notes: [note(1), note(2, { path: 'tests/a.ts', outside: true })],
    leftOut: [],
    by: 'Claude Sonnet',
    at: 1,
    ...over,
  };
}

/** The backend of a review: each call changes it as the core does, and is noted. */
function backend(start: AgentReview) {
  let r = start;
  const calls: string[] = [];
  const set = (next: AgentReview) => {
    r = next;
    app.prReviews.p1 = { 12: r };
    return r;
  };
  app.prReviews.p1 = { 12: r };
  const publish = vi.fn((a: { verdict: string; acceptMoved: boolean }) => {
    calls.push(`publish:${a.verdict}:${a.acceptMoved}`);
    return { review: set({ ...r, phase: 'published', publishedAt: 1 }), moved: false };
  });
  fakeBackend({
    pr_review_note: (a: { id: number; choice: ReviewNote['choice']; body?: string }) => {
      calls.push(`note:${a.id}:${a.choice}`);
      return set({
        ...r,
        notes: r.notes.map((n) => (n.id === a.id ? { ...n, choice: a.choice, body: a.body ?? n.body } : n)),
      });
    },
    pr_review_summary: (a: { summary: string }) => {
      calls.push('summary');
      return set({ ...r, summary: a.summary });
    },
    pr_review_publish: publish,
    pr_review_discard: () => {
      calls.push('discard');
    },
    pr_review_start: () => {
      calls.push('start');
      return r;
    },
  });
  return { calls, publish, current: () => r };
}

/** The view, following the review as the backend changes it. */
function show(me = 'ada') {
  return render(PrReviewViewHost, { me });
}

const card = (name: RegExp) => screen.getByRole('article', { name });

beforeEach(() => {
  setLang('fr');
  localStorage.clear();
  resetApp({ projects: [project()] });
});

describe('the review of a pull request by an agent (ESC-66)', () => {
  it('shows its comments file by file with their lines, and holds the publication until each is decided', async () => {
    const user = userEvent.setup();
    const b = backend(review());
    show();
    expect(screen.getByRole('heading', { name: /Connexion par lien/ })).toBeInTheDocument();
    expect(screen.getByText('Relu par Claude Sonnet')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'src/auth.ts' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'tests/a.ts' })).toBeInTheDocument();
    expect(screen.getAllByText('store.set(email, token);')).toHaveLength(2);
    expect(screen.getByText(/Hors du diff/)).toBeInTheDocument();
    expect(screen.getByText('2 à trancher')).toBeInTheDocument();

    const publish = screen.getByRole('button', { name: 'Publier sur GitHub' });
    expect(publish).toBeDisabled();
    await user.click(publish);
    expect(b.publish).not.toHaveBeenCalled();

    await user.click(within(card(/src\/auth\.ts/)).getByRole('button', { name: 'Garder' }));
    await user.click(within(card(/tests\/a\.ts/)).getByRole('button', { name: 'Écarter' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Publier sur GitHub' })).toBeEnabled());
    expect(screen.getByText(/1 gardé · 1 écarté/)).toBeInTheDocument();
    expect(b.publish).not.toHaveBeenCalled();

    await user.click(screen.getByRole('button', { name: 'Publier sur GitHub' }));
    await waitFor(() => expect(b.publish).toHaveBeenCalledOnce());
    expect(b.calls).toEqual(['note:1:kept', 'note:2:dropped', 'publish:commented:false']);
    expect(await screen.findByText(/Publiée sur GitHub/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Publier sur GitHub' })).not.toBeInTheDocument();
  });

  it('edits a comment, brings the agent’s text back, and takes back a comment dropped', async () => {
    const user = userEvent.setup();
    const b = backend(review({ notes: [note(1)] }));
    show();
    await user.click(screen.getByRole('button', { name: 'Corriger' }));
    const box = screen.getByRole('textbox', { name: 'Corriger le commentaire' });
    await user.clear(box);
    expect(screen.getByRole('button', { name: 'Garder ce texte' })).toBeDisabled();
    await user.type(box, 'Mon texte');
    await user.click(screen.getByRole('button', { name: 'Revenir au texte de l’agent' }));
    expect(box).toHaveValue('Commentaire 1');
    await user.clear(box);
    await user.type(box, 'Mon texte');
    await user.click(screen.getByRole('button', { name: 'Garder ce texte' }));
    expect(await screen.findByText('Mon texte')).toBeInTheDocument();
    expect(screen.getByText('Corrigé et gardé')).toBeInTheDocument();
    expect(b.current().notes[0].body).toBe('Mon texte');

    await user.click(screen.getByRole('button', { name: 'Écarter' }));
    await user.click(await screen.findByRole('button', { name: 'Reprendre' }));
    await waitFor(() => expect(b.current().notes[0].choice).toBe('open'));
  });

  it('starts on « Commenter », never on what the review could suggest, and the text saved before publishing', async () => {
    const user = userEvent.setup();
    const b = backend(review({ notes: [] }));
    show();
    const group = screen.getByRole('radiogroup', { name: 'Ce que dit la relecture' });
    expect(within(group).getByRole('radio', { name: 'Commenter' })).toBeChecked();
    await user.click(within(group).getByRole('radio', { name: 'Approuver' }));
    const text = screen.getByRole('textbox', { name: 'Texte de la relecture' });
    await user.clear(text);
    await user.type(text, 'Rien à redire.');
    await user.click(screen.getByRole('button', { name: 'Publier sur GitHub' }));
    await waitFor(() => expect(b.publish).toHaveBeenCalledOnce());
    expect(b.calls).toEqual(['summary', 'publish:approved:false']);
  });

  it('offers only « Commenter » on one’s own pull request', () => {
    backend(review({ author: 'ADA' }));
    show('ada');
    expect(screen.getByRole('radio', { name: 'Approuver' })).toBeDisabled();
    expect(screen.getByRole('radio', { name: 'Demander des changements' })).toBeDisabled();
    expect(screen.getByRole('radio', { name: 'Commenter' })).toBeEnabled();
  });

  it('says a push came after the reading before anything leaves', async () => {
    const user = userEvent.setup();
    const r = review({ notes: [] });
    const b = backend(r);
    b.publish.mockImplementationOnce(() => ({ review: r, moved: true }));
    show();
    await user.click(screen.getByRole('button', { name: 'Publier sur GitHub' }));
    await waitFor(() => expect(app.modal?.kind).toBe('confirm'));
    const m = app.modal as { kind: 'confirm'; title: string; onConfirm: (o: boolean) => void };
    expect(m.title).toBe('La PR a bougé depuis la relecture');
    await m.onConfirm(false);
    expect(b.publish).toHaveBeenCalledTimes(2);
    expect(b.publish.mock.calls[1][0].acceptMoved).toBe(true);
  });

  it('says what it is doing: reading, failed, a publication refused, files left out', async () => {
    backend(review({ phase: 'reading', notes: [], summary: '' }));
    const { unmount } = show();
    expect(screen.getByRole('status')).toHaveTextContent('L’agent relit la PR #12…');
    expect(screen.queryByRole('button', { name: /Publier/ })).not.toBeInTheDocument();
    unmount();

    backend(review({ phase: 'failed', error: 'pas de réponse', notes: [] }));
    const second = show();
    expect(screen.getByRole('alert')).toHaveTextContent('pas de réponse');
    second.unmount();

    backend(review({ error: 'Publication refusée', leftOut: ['a.lock', 'b.lock'] }));
    show();
    expect(screen.getByRole('alert')).toHaveTextContent('Publication refusée');
    expect(screen.getByText('2 fichiers n’ont pas été lus : trop longs ou générés.')).toBeInTheDocument();
  });

  it('asks before a new reading replaces what was decided', async () => {
    const user = userEvent.setup();
    const b = backend(review({ notes: [note(1, { choice: 'kept' })] }));
    show();
    await user.click(screen.getByRole('button', { name: 'Relire à nouveau' }));
    expect(b.calls).toEqual([]);
    expect(app.modal?.kind).toBe('confirm');
  });

  it('is asked from the list, and the row says where it stands', async () => {
    const user = userEvent.setup();
    let started = 0;
    fakeBackend({
      project_prs: () => ({
        data: { word: 'PR', prs: [pr({ number: 12, title: 'Lien' }), pr({ number: 13, title: 'Autre' })], me: 'ada' },
      }),
      pr_reviews: () => [review({ number: 13 })],
      pr_review_start: () => {
        started += 1;
        app.prReviews.p1 = { ...app.prReviews.p1, 12: review({ phase: 'reading', notes: [] }) };
        return review({ phase: 'reading', notes: [] });
      },
    });
    render(PrsView, { project: project() });
    const rows = await screen.findAllByRole('listitem');
    await waitFor(() => expect(rows[1]).toHaveTextContent('Relecture : 2 à trancher'));
    await user.click(within(rows[0]).getByRole('button', { name: 'Faire relire' }));
    await waitFor(() => expect(started).toBe(1));
    expect(await screen.findByRole('status')).toHaveTextContent('L’agent relit la PR #12…');
  });

  it('sees the review of an agent end out of sight, with a way to it', () => {
    const r = review({ phase: 'reading' });
    (app as unknown as { onEvent: (e: UiEvent) => void }).onEvent({ type: 'prReview', review: r });
    (app as unknown as { onEvent: (e: UiEvent) => void }).onEvent({ type: 'prReview', review: { ...r, phase: 'ready' } });
    expect(app.toasts.at(-1)?.text).toBe('Relecture de la PR #12 prête.');
    app.toasts.at(-1)?.action?.onClick();
    expect(app.prReviewOpen.p1).toBe(12);
    (app as unknown as { onEvent: (e: UiEvent) => void }).onEvent({ type: 'prReviewGone', projectId: 'p1', number: 12 });
    expect(app.prReviews.p1[12]).toBeUndefined();
  });
});

describe('what the window says of a review', () => {
  it('counts, blocks and marks as the backend refuses', () => {
    const r = review({ notes: [note(1, { choice: 'kept' }), note(2), note(3, { choice: 'dropped' })] });
    expect(tally(r.notes)).toEqual({ open: 1, kept: 1, dropped: 1 });
    expect(publishBlock(r, 'commented', 'x')).toBe('Encore 1 commentaire à garder ou à écarter.');
    expect(reviewMark(r).text).toBe('Relecture : 1 à trancher');
    const done = review({ notes: [note(1, { choice: 'dropped' })] });
    expect(publishBlock(done, 'approved', '')).toBeNull();
    expect(publishBlock(done, 'commented', '  ')).toBe('Rien à publier.');
    expect(publishBlock(done, 'changesRequested', '')).toBe('Écris le texte de la relecture pour demander des changements.');
    expect(reviewMark(done)).toEqual({ text: 'Relecture à publier', tone: 'ask' });
  });
});
