import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { docsStore } from '../../lib/docs.svelte';
import { handleShortcut } from '../../lib/shortcuts';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import type { Discussion, DocsTree } from '../../lib/types';
import DocsSidebar from './DocsSidebar.svelte';
import DocsView from './DocsView.svelte';
import NewDiscussionModal from './NewDiscussionModal.svelte';

// The discussions of the Docs tab (ESC-261): « Nouvelle discussion », their cards, a discussion open with « Ce qui ira
// dans la spec » and « Verser dans la spec », and the gestures of the spec that open one.

const EMPTY: DocsTree = { branch: '', pages: 0, subjects: [], existingSpec: [] };

const disc = (o: Partial<Discussion> = {}): Discussion => ({
  kind: 'brainstorm',
  subject: 'Relance des adhésions',
  pages: [],
  decisions: [],
  ...o,
});

const PROPOSED = disc({
  pages: [
    {
      path: 'espace-adherent/relances',
      before: null,
      page: {
        path: 'espace-adherent/relances',
        title: 'Relances',
        subject: 'Espace adhérent',
        body: 'Trois relances par e-mail.',
      } as never,
      added: 1,
      removed: 0,
    },
    {
      path: 'notifications/general',
      before: 'Aucune notification n’est planifiée.',
      page: {
        path: 'notifications/general',
        title: 'Notifications',
        subject: 'notifications',
        body: 'Les relances sont planifiées chaque nuit.',
      } as never,
      added: 1,
      removed: 1,
    },
  ],
  decisions: [{ title: 'Relances par e-mail seulement', decision: 'Pas de SMS.', consequences: '', supersedes: [] } as never],
});

function backend(over: Record<string, (a: any) => unknown> = {}) {
  return fakeBackend({
    docs_spec_tree: () => EMPTY,
    docs_decisions: () => [],
    docs_drive: () => ({ folders: [], files: [] }),
    docs_recent: () => [],
    docs_pending: () => ({ files: [], count: 0, codeCommits: 0, conflict: null }),
    get_conversation: () => [],
    ...over,
  });
}

describe('the discussions of the Docs tab', () => {
  beforeEach(() => {
    docsStore.tree = {};
  });

  it('starts a brainstorm on a subject with a file joined, filed first, then opens it', async () => {
    resetApp();
    app.openDocs('p1');
    const made = agent({ id: 'd1', discussion: disc() });
    const b = backend({
      docs_document_check: (a) => a.files.map((f: any) => ({ ...f, lfs: false, large: false })),
      attach_open: () => 'h1',
      attach_finish: () => ({ name: 'cr.pdf', path: 'C:/data/drafts/x/cr.pdf', claudePath: '', size: 2, origin: false }),
      docs_document_add: () => ({
        file: { path: 'Comptes rendus/cr.pdf', name: 'cr.pdf', folder: 'Comptes rendus' },
        placement: { folder: 'Comptes rendus', new: false, by: 'sort' },
        lfs: false,
      }),
      discussion_start: () => made,
    });
    render(NewDiscussionModal, { projectId: 'p1' });
    const dialog = screen.getByRole('dialog', { name: 'Nouvelle discussion' });
    // Two agents to choose from; the spec writer by default.
    expect(within(dialog).getByRole('radio', { name: /Discussion/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(within(dialog).getByRole('radio', { name: /Brainstorm/ }));
    expect(within(dialog).getByRole('radio', { name: /Brainstorm/ })).toHaveTextContent('RGPD');
    const start = within(dialog).getByRole('button', { name: 'Commencer' });
    expect(start).toBeDisabled();
    await userEvent.type(within(dialog).getByRole('textbox'), 'Relance des adhésions');
    await userEvent.upload(screen.getByTestId('discussion-files'), new File(['%PDF'], 'cr.pdf'));
    expect(within(dialog).getByText('cr.pdf')).toBeInTheDocument();
    // The footer: the project's harness, account and model.
    expect(within(dialog).getByRole('button', { name: /^Harness : Claude Code/ })).toBeInTheDocument();
    expect(within(dialog).getByText('branche courante')).toBeInTheDocument();
    await userEvent.click(start);
    await waitFor(() => expect(b.called('discussion_start')).toHaveLength(1));
    // No folder chosen: Escouade files it; the agent is told where.
    expect(b.called('docs_document_add')[0].args).toMatchObject({ id: 'p1', folder: null });
    expect(b.called('discussion_start')[0].args).toEqual({
      id: 'p1',
      start: {
        kind: 'brainstorm',
        subject: 'Relance des adhésions',
        documents: ['Comptes rendus/cr.pdf'],
        message: null,
        gap: null,
        // Named, so that it never starts on a harness a discussion does not run on.
        engine: 'claude',
        account: null,
        model: null,
      },
    });
    expect(app.modal).toBeNull();
    expect(app.docsDiscussion.p1).toBe('d1');
    expect(app.agents.d1.discussion?.kind).toBe('brainstorm');
  });

  it('lists the discussions in cards, each marked with its kind and its state, and opens one', async () => {
    resetApp({
      agents: [
        agent({ id: 'd1', createdAt: 3, status: 'waiting', discussion: disc() }),
        agent({ id: 'd2', createdAt: 2, status: 'running', discussion: disc({ kind: 'discussion', subject: 'Paiement en 4 fois ?' }) }),
        agent({
          id: 'd3',
          createdAt: 1,
          status: 'done',
          discussion: disc({ subject: 'Tarif famille', poured: { at: 1, pages: ['a/b', 'a/c'], decisions: [14] } }),
        }),
        agent({ id: 'a1', name: 'un-agent' }),
      ],
    });
    app.openDocs('p1');
    backend();
    render(DocsSidebar, { project: app.projects[0] });
    const list = screen.getByRole('region', { name: 'Discussions' });
    const cards = within(list).getAllByRole('button');
    expect(cards.map((c) => c.getAttribute('data-discussion'))).toEqual(['d1', 'd2', 'd3']);
    expect(cards[0]).toHaveTextContent('Brainstorm');
    expect(cards[0]).toHaveTextContent('Question');
    expect(cards[1]).toHaveTextContent('Discussion');
    expect(cards[1]).toHaveTextContent('Répond');
    expect(cards[2]).toHaveTextContent('Versé');
    expect(cards[2]).toHaveTextContent('2 pages · n° 014');
    await userEvent.click(cards[1]);
    expect(app.docsDiscussion.p1).toBe('d2');
    // A space chosen: the discussion leaves the main area.
    await userEvent.click(screen.getByRole('button', { name: /Décisions/ }));
    expect(app.docsDiscussion.p1).toBeNull();
    // « Nouvelle discussion », and Ctrl+B in the tab.
    await userEvent.click(screen.getByRole('button', { name: /Nouvelle discussion/ }));
    expect(app.modal).toEqual({ kind: 'newDiscussion', projectId: 'p1', preset: undefined });
    app.modal = null;
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'b', ctrlKey: true }))).toBe(true);
    expect(app.modal).toMatchObject({ kind: 'newDiscussion', projectId: 'p1' });
  });

  it('shows what will go in the spec page by page with the decisions, pours it, and takes a proposal out', async () => {
    resetApp({ agents: [agent({ id: 'd1', status: 'done', discussion: PROPOSED })] });
    app.openDiscussion('p1', 'd1');
    const b = backend({
      discussion_pour: () => ({ at: 1, pages: ['espace-adherent/relances', 'notifications/general'], decisions: [15] }),
    });
    render(DocsView, { project: app.projects[0] });
    const panel = await screen.findByRole('complementary', { name: 'Ce qui ira dans la spec' });
    const added = within(panel).getByText('+ Trois relances par e-mail.');
    expect(added).toHaveClass('add');
    expect(panel.querySelector('[data-page="espace-adherent/relances"]')).toHaveTextContent('nouvelle section');
    const changed = panel.querySelector('[data-page="notifications/general"]') as HTMLElement;
    expect(within(changed).getByText('Aucune notification n’est planifiée.')).toHaveClass('del');
    expect(within(changed).getByText('Les relances sont planifiées chaque nuit.')).toHaveClass('add');
    expect(within(panel).getByText('Relances par e-mail seulement')).toBeInTheDocument();
    expect(screen.getByText('Avec l’agent de brainstorm')).toBeInTheDocument();
    // Taken out of what goes in the spec.
    await userEvent.click(within(panel).getByRole('button', { name: 'Retirer la page Notifications de ce qui ira dans la spec' }));
    expect(b.called('discussion_drop')[0].args).toEqual({ agent: 'd1', page: 'notifications/general', decision: null });
    await userEvent.click(within(panel).getByRole('button', { name: /Retirer la décision Relances/ }));
    expect(b.called('discussion_drop')[1].args).toEqual({ agent: 'd1', page: null, decision: 1 });
    await userEvent.click(screen.getByRole('button', { name: 'Verser dans la spec' }));
    await waitFor(() => expect(b.called('discussion_pour')).toHaveLength(1));
    expect(b.called('discussion_pour')[0].args).toEqual({ agent: 'd1' });
    await waitFor(() => expect(app.toasts.map((x) => x.text)).toContain('Versé dans la spec : 2 pages. Pas encore partagé.'));
  });

  it('has nothing to pour while the agent proposes nothing', async () => {
    resetApp({ agents: [agent({ id: 'd1', discussion: disc({ kind: 'discussion' }) })] });
    app.openDiscussion('p1', 'd1');
    backend();
    render(DocsView, { project: app.projects[0] });
    expect(await screen.findByText(/Rien pour l’instant/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Verser dans la spec' })).toBeDisabled();
    expect(screen.getByText('Avec l’agent rédacteur')).toBeInTheDocument();
  });

  it('offers a project without a spec to have its first one written from the code', async () => {
    resetApp();
    app.openDocs('p1');
    backend();
    render(DocsView, { project: app.projects[0] });
    await userEvent.click(await screen.findByRole('button', { name: 'Écrire une première spec depuis le code' }));
    expect(app.modal).toMatchObject({
      kind: 'newDiscussion',
      projectId: 'p1',
      preset: { kind: 'discussion', subject: 'Première spec depuis le code' },
    });
    expect((app.modal as { preset: { message: string } }).preset.message).toContain('rangée par sujet');
  });

  it('never starts on a harness that writes without Escouade, and says so', async () => {
    resetApp();
    app.projects[0].engine = 'cursor';
    const b = backend({ discussion_start: () => agent({ id: 'd8', discussion: disc({ kind: 'discussion' }) }) });
    render(NewDiscussionModal, { projectId: app.projects[0].id });
    const dialog = screen.getByRole('dialog', { name: 'Nouvelle discussion' });
    expect(within(dialog).getByText(/Cursor CLI modifie les fichiers de son dossier sans passer par Escouade/)).toHaveTextContent(
      'une discussion part sur Claude Code',
    );
    expect(within(dialog).getByRole('button', { name: /^Harness : Claude Code/ })).toBeInTheDocument();
    await userEvent.type(within(dialog).getByRole('textbox'), 'Échéances');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Commencer' }));
    await waitFor(() => expect(b.called('discussion_start')).toHaveLength(1));
    expect(b.called('discussion_start')[0].args.start).toMatchObject({ engine: 'claude' });
  });

  it('starts the preset of a gesture with its own first message', async () => {
    resetApp();
    const b = backend({ discussion_start: () => agent({ id: 'd9', discussion: disc({ kind: 'discussion' }) }) });
    render(NewDiscussionModal, {
      projectId: 'p1',
      preset: { kind: 'discussion', subject: 'Mettre à jour « Échéances »', message: 'Lis le code.' },
    });
    expect(screen.getByRole('textbox')).toHaveValue('Mettre à jour « Échéances »');
    await userEvent.click(screen.getByRole('button', { name: 'Commencer' }));
    await waitFor(() => expect(b.called('discussion_start')).toHaveLength(1));
    expect(b.called('discussion_start')[0].args.start).toMatchObject({ kind: 'discussion', message: 'Lis le code.', documents: [] });
  });

  // ESC-289: refused after its agent was made, the backend removes it; the window says why and stays, with what was chosen.
  it('says a refusal of « Commencer » in the window, which stays open to start again', async () => {
    resetApp();
    app.modal = { kind: 'newDiscussion', projectId: 'p1' };
    app.docsDiscussion.p1 = null;
    let refuse = true;
    const b = backend({
      discussion_start: () => {
        if (refuse) throw 'Claude Code ne démarre pas';
        return agent({ id: 'd7', discussion: disc({ kind: 'discussion' }) });
      },
    });
    render(NewDiscussionModal, {
      projectId: 'p1',
      preset: { kind: 'discussion', subject: 'Première spec depuis le code', message: 'Lis le code.' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Commencer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Claude Code ne démarre pas');
    expect(app.modal).not.toBeNull();
    expect(app.docsDiscussion.p1).toBeNull();
    expect(screen.getByRole('textbox')).toHaveValue('Première spec depuis le code');
    // Started again: the refusal goes, the discussion opens.
    refuse = false;
    await userEvent.click(screen.getByRole('button', { name: 'Commencer' }));
    await waitFor(() => expect(app.modal).toBeNull());
    expect(b.called('discussion_start')).toHaveLength(2);
    expect(app.docsDiscussion.p1).toBe('d7');
  });
});
