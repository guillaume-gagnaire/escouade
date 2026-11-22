// The phone's spaces (MO-1): a connection of its own to each, through the app's transport, what each says read and
// kept through a cut, the foreground alone, and what the screens read of it.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { FakeSocket, glance, tick } from '../test/fake-socket';
import {
  ALL,
  allAgents,
  boxAsks,
  callIn,
  pauseSpaces,
  phone,
  resumeSpaces,
  shownSpaces,
  startSpaces,
  stateOf,
  stopSpaces,
  stoppedAgents,
  waitingIn,
} from './phone-spaces.svelte';
import type { PairedSpace } from './paired';

const PAIRED: PairedSpace[] = [
  { id: 'here', name: 'Ce poste', here: true, url: 'ws://127.0.0.1:7001/a/remote', token: 'tok-poste' },
  { id: 'travail', name: 'Travail', here: false, url: 'ws://127.0.0.1:7002/b/remote', token: 'tok-travail' },
];

describe('the phone’s spaces', () => {
  let sockets: FakeSocket[];
  const of = (url: string) => sockets.filter((s) => s.url === url).at(-1)!;

  beforeEach(() => {
    sockets = [];
    phone.spaces = [];
    phone.filter = ALL;
  });
  afterEach(() => stopSpaces());

  const start = () =>
    startSpaces(PAIRED, {
      open: (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      delays: [10_000],
      every: 60_000,
      version: '2.3.0',
    });

  it('opens one connection per space, each with its own address and token, and reads what waits there', async () => {
    start();
    expect(sockets.map((s) => s.url)).toEqual(PAIRED.map((p) => p.url));
    expect(phone.spaces.map((s) => stateOf(s))).toEqual(['connecting', 'connecting']);
    for (const [i, p] of PAIRED.entries()) {
      sockets[i].welcome();
      expect(sockets[i].texts()[0]).toMatchObject({ t: 'hello', version: '2.3.0', token: p.token });
    }
    await tick();
    // Asked before the handshake, the glance waited for it.
    expect(sockets.every((s) => s.calls().some((c) => c.cmd === 'space_glance' && c.args.counts === false))).toBe(true);
    of(PAIRED[1].url).answerGlances(glance({ waiting: 3 }));
    of(PAIRED[0].url).answerGlances(glance({ waiting: 1 }));
    await tick();
    expect(phone.spaces.map(waitingIn)).toEqual([1, 3]);
    expect(phone.spaces.map((s) => stateOf(s))).toEqual(['open', 'open']);
  });

  it('a cut of one space leaves the other open, and keeps what the cut one said last', async () => {
    start();
    sockets.forEach((s) => s.welcome());
    await tick();
    sockets.forEach((s) => s.answerGlances(glance({ waiting: 2 })));
    await tick();
    of(PAIRED[1].url).close(1006, 'gone');
    expect(stateOf(phone.spaces[1])).toBe('cut');
    expect(stateOf(phone.spaces[0])).toBe('open');
    expect(phone.spaces[1].conn.retryAt).toBeGreaterThan(Date.now());
    expect(waitingIn(phone.spaces[1])).toBe(2);
  });

  it('a space that refuses the phone (a revoked token) stops its tries and says why', async () => {
    start();
    const s = of(PAIRED[1].url);
    s.onopen?.();
    s.json({ t: 'refused', code: 'token', error: 'Jeton révoqué' });
    s.close(1008, 'refused');
    expect(stateOf(phone.spaces[1])).toBe('refused');
    expect(phone.spaces[1].conn.reason).not.toBe('');
  });

  it('refuses a command the core keeps for its own window: the phone has none to run it', async () => {
    start();
    sockets.forEach((s) => s.welcome());
    await expect(callIn('travail', 'open_url', { url: 'https://example.com' })).rejects.toBe('open_url se fait au poste.');
    expect(
      of(PAIRED[1].url)
        .calls()
        .some((c) => c.cmd === 'open_url'),
    ).toBe(false);
  });

  it('closes every connection in the background and opens them again when the app comes back', async () => {
    start();
    sockets.forEach((s) => s.welcome());
    await tick();
    sockets.forEach((s) => s.answerGlances(glance({ waiting: 1 })));
    await tick();
    pauseSpaces();
    expect(sockets.every((s) => s.closed)).toBe(true);
    // What each said is still shown meanwhile.
    expect(phone.spaces.map(waitingIn)).toEqual([1, 1]);
    resumeSpaces();
    expect(sockets).toHaveLength(4);
    expect(sockets.slice(2).map((s) => s.url)).toEqual(PAIRED.map((p) => p.url));
    expect(phone.spaces.map(waitingIn)).toEqual([1, 1]);
  });
});

describe('what the screens read of the spaces', () => {
  const project = { id: 'p1', name: 'facturation', color: 'oklch(0.72 0.12 250)' };
  const agent = (id: string, status: string, lastActivity: number) => ({
    id,
    projectId: 'p1',
    name: id,
    status,
    activity: status === 'running' ? 'Écrit les tests' : null,
    lastActivity,
    request: null,
  });
  const space = (id: string, g: ReturnType<typeof glance> | null) => ({
    id,
    name: id,
    here: false,
    conn: { state: 'open' } as never,
    glance: g as never,
    at: 1,
  });

  it('puts the agents that wait first, the longest waiting first, then the latest at work', () => {
    const spaces = [
      space('a', glance({ projects: [project], agents: [agent('w2', 'waiting', 20), agent('r1', 'running', 5)] })),
      space('b', glance({ projects: [project], agents: [agent('w1', 'waiting', 10), agent('r2', 'running', 50)] })),
    ];
    expect(stoppedAgents(spaces).map((x) => x.item.id)).toEqual(['w1', 'w2']);
    expect(allAgents(spaces).map((x) => x.item.id)).toEqual(['w1', 'w2', 'r2', 'r1']);
    expect(stoppedAgents(spaces)[0]).toMatchObject({ space: { id: 'b', name: 'b' }, project });
  });

  it('shows the requests of the box, the newest first, and nothing of a space that gives its numbers alone', () => {
    const spaces = [
      space(
        'a',
        glance({
          asks: [
            { id: 'k1', projectId: 'p1', title: 'Old', createdAt: 1 },
            { id: 'k2', projectId: 'p1', title: 'New', createdAt: 9 },
          ],
        }),
      ),
      space('b', glance({ countsOnly: true, waiting: 4, asks: [{ id: 'k3', projectId: 'p1', title: 'Hidden', createdAt: 5 }] })),
    ];
    expect(boxAsks(spaces).map((x) => x.item.title)).toEqual(['New', 'Old']);
  });

  it('shows every space for « Partout », one for its chip', () => {
    const spaces = [space('a', null), space('b', null)];
    expect(shownSpaces(spaces, ALL).map((s) => s.id)).toEqual(['a', 'b']);
    expect(shownSpaces(spaces, 'b').map((s) => s.id)).toEqual(['b']);
  });
});
