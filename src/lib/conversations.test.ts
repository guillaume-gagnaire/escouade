import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend } from '../test/ipc';
import { Conversation, conversationOf, releaseIdle } from './conversations.svelte';
import type { ConvItem } from './types';

const text = (id: string, t: string, streaming = true): ConvItem => ({ kind: 'text', id, text: t, streaming });

describe('Conversation', () => {
  it('applies append, delta and patch ops', async () => {
    fakeBackend({ get_conversation: () => [] });
    const c = new Conversation('a1');
    await c.load();
    c.apply({ op: 'append', item: text('m:0', '') });
    c.apply({ op: 'delta', id: 'm:0', text: 'Bon' });
    c.apply({ op: 'delta', id: 'm:0', text: 'jour' });
    expect(c.items[0]).toMatchObject({ text: 'Bonjour', streaming: true });
    c.apply({ op: 'patch', id: 'm:0', patch: { text: 'Bonjour !', streaming: false } });
    expect(c.items).toEqual([text('m:0', 'Bonjour !', false)]);
  });

  it('replaces an item appended twice instead of duplicating it', async () => {
    fakeBackend({ get_conversation: () => [] });
    const c = new Conversation('a1');
    await c.load();
    c.apply({ op: 'append', item: text('x', 'v1') });
    c.apply({ op: 'append', item: text('x', 'v2') });
    expect(c.items).toHaveLength(1);
    expect(c.items[0]).toMatchObject({ text: 'v2' });
  });

  it('ignores ops for unknown items', async () => {
    fakeBackend({ get_conversation: () => [] });
    const c = new Conversation('a1');
    await c.load();
    c.apply({ op: 'delta', id: 'nope', text: 'x' });
    c.apply({ op: 'patch', id: 'nope', patch: { text: 'y' } });
    expect(c.items).toEqual([]);
  });

  it('does not duplicate text streamed while the snapshot was loading', async () => {
    let release!: (v: ConvItem[]) => void;
    fakeBackend({ get_conversation: () => new Promise<ConvItem[]>((r) => (release = r)) });
    const c = new Conversation('a1');
    const loading = c.load();
    // Ops emitted while the backend builds the snapshot: already contained in it.
    c.apply({ op: 'delta', id: 'm:0', text: 'jour' });
    c.apply({ op: 'append', item: { kind: 'notice', id: 'n1', ts: 1, level: 'info', text: 'Contexte compacté' } });
    release([text('m:0', 'Bonjour')]);
    await loading;
    expect(c.loaded).toBe(true);
    expect(c.items.map((i) => i.id)).toEqual(['m:0', 'n1']);
    expect(c.items[0]).toMatchObject({ text: 'Bonjour' });
    // Later deltas apply normally.
    c.apply({ op: 'delta', id: 'm:0', text: ' !' });
    expect(c.items[0]).toMatchObject({ text: 'Bonjour !' });
  });

  it('records a load failure', async () => {
    fakeBackend({
      get_conversation: () => {
        throw new Error('agent introuvable');
      },
    });
    const c = new Conversation('a1');
    await c.load();
    expect(c.error).toMatch(/agent introuvable/);
    expect(c.loaded).toBe(true);
  });

  it('keeps the message asked for until the view shows it', () => {
    fakeBackend({ get_conversation: () => [] });
    const c = new Conversation('a1');
    expect(c.jump).toBeNull();
    c.reveal('m:0');
    expect(c.jump).toBe('m:0');
  });

  it('tells which item shows a message: itself, or the call of the subagent that wrote it', async () => {
    fakeBackend({
      get_conversation: () => [
        text('m:0', 'Je délègue', false),
        { kind: 'tool', id: 't1', name: 'Agent', input: {}, status: 'ok', ts: 1 },
        { kind: 'tool', id: 't2', name: 'Agent', input: {}, status: 'ok', ts: 1, parent: 't1' },
        { ...text('s:0', 'Trouvé', false), parent: 't2' },
        { ...text('lost:0', 'Orphelin', false), parent: 'gone' },
      ],
    });
    const c = new Conversation('a1');
    await c.load();
    expect(c.shownAs('m:0')).toBe('m:0');
    expect(c.shownAs('s:0')).toBe('t1');
    expect(c.shownAs('lost:0')).toBeNull();
    expect(c.shownAs('nope')).toBeNull();
  });
});

describe('Conversations out of sight', () => {
  const MINUTE = 60_000;
  const idle = () => false;
  let reads = 0;
  beforeEach(() => {
    vi.useFakeTimers();
    reads = 0;
    fakeBackend({
      get_conversation: () => {
        reads++;
        return [text('m:0', 'Bonjour', false)];
      },
    });
  });
  afterEach(() => vi.useRealTimers());

  /** A conversation loaded for a view that showed it, then went: the clock starts then. */
  async function shownThenLeft() {
    const c = conversationOf(`gone-${Math.random()}`);
    const hide = c.show();
    await vi.waitFor(() => expect(c.loaded).toBe(true));
    hide();
    return c;
  }

  it('are let go of after 10 minutes, then read again for the next view', async () => {
    const c = await shownThenLeft();
    vi.advanceTimersByTime(10 * MINUTE - 1);
    releaseIdle(Date.now(), idle);
    expect(c.items).toHaveLength(1);
    vi.advanceTimersByTime(1);
    releaseIdle(Date.now(), idle);
    expect(c.items).toEqual([]);
    expect(c.loaded).toBe(false);
    expect(reads).toBe(1);
    c.show();
    await vi.waitFor(() => expect(c.loaded).toBe(true));
    expect(reads).toBe(2);
    expect(c.items.map((i) => i.id)).toEqual(['m:0']);
  });

  it('are kept while a view shows them, however long', async () => {
    const c = conversationOf(`shown-${Math.random()}`);
    c.show();
    await vi.waitFor(() => expect(c.loaded).toBe(true));
    vi.advanceTimersByTime(60 * MINUTE);
    releaseIdle(Date.now(), idle);
    expect(c.loaded).toBe(true);
    expect(c.items).toHaveLength(1);
  });

  it('are kept for an agent at work or waiting for an answer', async () => {
    const c = await shownThenLeft();
    vi.advanceTimersByTime(60 * MINUTE);
    releaseIdle(Date.now(), (id) => id === c.agentId);
    expect(c.loaded).toBe(true);
    expect(c.items).toHaveLength(1);
  });

  it('are kept while a message waits for the preparation of the worktree', async () => {
    const c = await shownThenLeft();
    const unhold = c.holdMessage({ text: 'Ajoute des tests', images: 0 });
    vi.advanceTimersByTime(60 * MINUTE);
    releaseIdle(Date.now(), idle);
    expect(c.items).toHaveLength(1);
    unhold();
    releaseIdle(Date.now(), idle);
    expect(c.items).toEqual([]);
  });

  it('count their 10 minutes from when the last view went', async () => {
    const c = await shownThenLeft();
    vi.advanceTimersByTime(9 * MINUTE);
    c.show()();
    vi.advanceTimersByTime(9 * MINUTE);
    releaseIdle(Date.now(), idle);
    expect(c.items).toHaveLength(1);
  });

  it('keep where the reader was and the message asked for, and leave out what comes meanwhile', async () => {
    const c = await shownThenLeft();
    const place = { stick: false, top: 300, anchor: { item: 'm:0', offset: -20 }, from: 'm:0' };
    c.place = place;
    vi.advanceTimersByTime(10 * MINUTE);
    releaseIdle(Date.now(), idle);
    c.reveal('m:0');
    // The next read has them: the backend records what comes.
    c.apply({ op: 'append', item: text('m:1', 'Suite', false) });
    expect(c.items).toEqual([]);
    expect(c.place).toEqual(place);
    expect(c.jump).toBe('m:0');
  });
});
