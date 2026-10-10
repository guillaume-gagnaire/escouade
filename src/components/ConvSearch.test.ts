import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { conversationOf } from '../lib/conversations.svelte';
import { app } from '../lib/state.svelte';
import type { ConvHit, ConvSearchResult } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import ConvSearch from './ConvSearch.svelte';

const NOW = Date.now();

function hit(over: Partial<ConvHit> = {}): ConvHit {
  return {
    agentId: 'a1',
    projectId: 'p1',
    agentName: 'refacto-auth',
    archived: false,
    eventIndex: 0,
    itemId: 'u1',
    snippet: 'Ajoute la pagination à l’API',
    mark: [10, 20],
    at: NOW - 2 * 3600 * 1000,
    ...over,
  };
}

const found = (hits: ConvHit[], over: Partial<ConvSearchResult> = {}): ConvSearchResult => ({
  hits,
  capped: false,
  timedOut: false,
  ...over,
});

const HITS = [
  hit(),
  hit({ itemId: 'm1:0', eventIndex: 3, snippet: 'J’ai ajouté la pagination', mark: [15, 25], at: NOW - 30_000 }),
  hit({
    agentId: 'a2',
    projectId: 'p2',
    agentName: 'vieux-chantier',
    archived: true,
    itemId: 'u9',
    snippet: 'pagination cassée',
    mark: [0, 10],
  }),
];

function setup(answer: (args: any) => unknown = () => found(HITS)) {
  resetApp({
    projects: [project(), project({ id: 'p2', name: 'studio' })],
    agents: [agent(), agent({ id: 'a2', projectId: 'p2', name: 'vieux-chantier', archived: true })],
  });
  app.now = NOW;
  app.modal = { kind: 'convSearch' };
  const backend = fakeBackend({ search_conversations: answer, get_conversation: () => [] });
  render(ConvSearch);
  return { backend, field: screen.getByRole('combobox', { name: 'Rechercher dans les conversations' }) };
}

const searches = (backend: ReturnType<typeof fakeBackend>) => backend.called('search_conversations').map((c) => c.args);

describe('ConvSearch', () => {
  beforeEach(() => resetApp());

  it('opens on its field, on this project, archived agents included', () => {
    const { field } = setup();
    expect(screen.getByRole('dialog', { name: 'Rechercher dans les conversations' })).toBeInTheDocument();
    expect(field).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Ce projet' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Tous les projets' })).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByRole('checkbox', { name: 'Agents archivés' })).toBeChecked();
  });

  it('searches this project’s conversations as you type, then every project’s, or without the archived agents', async () => {
    const { backend, field } = setup();
    await userEvent.type(field, 'pagination');
    await waitFor(() => expect(searches(backend).at(-1)).toEqual({ query: 'pagination', projectId: 'p1', archived: true }));
    // Typing does not ask at each key.
    expect(searches(backend)).toHaveLength(1);
    await userEvent.click(screen.getByRole('button', { name: 'Tous les projets' }));
    await waitFor(() => expect(searches(backend).at(-1)).toEqual({ query: 'pagination', projectId: null, archived: true }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Agents archivés' }));
    await waitFor(() => expect(searches(backend).at(-1)).toEqual({ query: 'pagination', projectId: null, archived: false }));
  });

  it('waits for two characters', async () => {
    const { backend, field } = setup();
    await userEvent.type(field, 'p');
    await new Promise((r) => setTimeout(r, 300));
    expect(searches(backend)).toHaveLength(0);
    expect(screen.getByText(/Cherche dans les messages/)).toBeInTheDocument();
  });

  it('groups the results by agent, with its project and « archivé », the match highlighted and its date', async () => {
    const { field } = setup();
    await userEvent.type(field, 'pagination');
    const groups = await screen.findAllByRole('group', { name: /^(refacto-auth|vieux-chantier)/ });
    expect(groups).toEqual([
      screen.getByRole('group', { name: 'refacto-auth, demo-api' }),
      screen.getByRole('group', { name: 'vieux-chantier, studio, archivé' }),
    ]);
    const [first, second] = groups;
    expect(first).toHaveTextContent('refacto-auth');
    expect(first).toHaveTextContent('demo-api');
    expect(first).not.toHaveTextContent('archivé');
    expect(second).toHaveTextContent('studio');
    expect(second).toHaveTextContent('archivé');
    const options = within(first).getAllByRole('option');
    expect(options).toHaveLength(2);
    expect(options[0].querySelector('mark')).toHaveTextContent(/^pagination$/);
    expect(options[0]).toHaveTextContent('Ajoute la pagination à l’API');
    expect(options[0]).toHaveTextContent('il y a 2 h');
    expect(options[1]).toHaveTextContent('à l’instant');
    expect(screen.getByText('3 résultats')).toBeInTheDocument();
  });

  it('moves through the results with ↑ ↓ and opens the chosen one with Enter', async () => {
    const { field } = setup();
    await userEvent.type(field, 'pagination');
    const options = await screen.findAllByRole('option');
    const selected = () => options.findIndex((o) => o.getAttribute('aria-selected') === 'true');
    expect(selected()).toBe(0);
    expect(field).toHaveAttribute('aria-activedescendant', options[0].id);
    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(selected()).toBe(2);
    expect(field).toHaveAttribute('aria-activedescendant', options[2].id);
    await userEvent.keyboard('{ArrowDown}');
    expect(selected()).toBe(0);
    await userEvent.keyboard('{ArrowUp}{ArrowUp}');
    expect(selected()).toBe(1);
    await userEvent.keyboard('{Enter}');
    expect(app.modal).toBeNull();
    expect(app.agent?.id).toBe('a1');
    // Its conversation is asked to show the message, by its id.
    expect(conversationOf('a1').jump).toBe('m1:0');
  });

  it('opens an archived agent, to read, in its project', async () => {
    const { field } = setup();
    await userEvent.type(field, 'pagination');
    await userEvent.click(await screen.findByRole('option', { name: /pagination cassée/ }));
    expect(app.modal).toBeNull();
    expect(app.project?.id).toBe('p2');
    expect(app.agent?.id).toBe('a2');
    expect(app.showArchived).toBe(true);
    expect(conversationOf('a2').jump).toBe('u9');
  });

  it('shows the conversation rather than the editor', async () => {
    const { field } = setup();
    await app.openEditor({ projectId: 'p1', source: 'project' });
    app.modal = { kind: 'convSearch' };
    await userEvent.type(field, 'pagination');
    await screen.findAllByRole('option');
    await userEvent.keyboard('{Enter}');
    expect(app.editorOn).toBe(false);
  });

  it('closes with Escape', async () => {
    setup();
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
  });

  it('says when nothing matches, and when the results stop short', async () => {
    let answer = found([]);
    const { field } = setup(() => answer);
    await userEvent.type(field, 'zzz');
    expect(await screen.findByText('Aucun message ne correspond.')).toBeInTheDocument();
    answer = found(
      Array.from({ length: 200 }, (_, i) => hit({ itemId: `u${i}` })),
      { capped: true },
    );
    await userEvent.type(field, 'z');
    expect(await screen.findByText('Les 200 premiers résultats : précise ta recherche.')).toBeInTheDocument();
    answer = found(HITS, { timedOut: true });
    await userEvent.type(field, 'z');
    expect(await screen.findByText('3 résultats, recherche arrêtée au bout de 5 s : précise ta recherche.')).toBeInTheDocument();
  });

  it('only shows the answer to the last search', async () => {
    const answers: ((r: ConvSearchResult) => void)[] = [];
    const { field } = setup(() => new Promise<ConvSearchResult>((r) => answers.push(r)));
    await userEvent.type(field, 'pagi');
    await waitFor(() => expect(answers).toHaveLength(1));
    await userEvent.type(field, 'nation');
    await waitFor(() => expect(answers).toHaveLength(2));
    answers[1](found([hit({ snippet: 'la dernière', mark: [3, 11] })]));
    expect(await screen.findByText('dernière')).toBeInTheDocument();
    answers[0](found(HITS));
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.getAllByRole('option')).toHaveLength(1);
  });
});
