// The thread of « À toi » (MO-5): the desktop's groups and order over every space, the box's views and requests, the
// glance alone when the box is not read, a line that keeps its place and its dot, the next line after one settled.

import { describe, expect, it } from 'vitest';
import type { InboxItem } from '$shared/lib/inbox/views';
import type { PendingRequest } from '$shared/lib/types';
import { windowAsk } from '$shared/test/inbox';
import { glance } from '../test/fake-socket';
import { waitingIn, type PhoneSpace, type SpaceBox } from './phone-spaces.svelte';
import { inOrder, nextAfter, steady, stoppedTitle, stopOf, threadOf, unseen, waitingLines, type Line } from './thread';

const NOW = new Date(2026, 10, 27, 15, 0).getTime();
const MIN = 60_000;
const project = { id: 'p1', name: 'facturation', color: 'oklch(0.72 0.12 250)' };

const request = (over: Partial<PendingRequest> = {}): PendingRequest => ({
  id: 'req_perm',
  kind: 'permission',
  tool: 'Bash',
  arg: 'rm -rf build',
  description: 'Supprime le dossier build',
  reason: null,
  questions: [],
  cut: false,
  defaultNo: false,
  ...over,
});

const agent = (id: string, lastActivity: number, r: PendingRequest | null = request(), status = 'waiting') => ({
  id,
  projectId: 'p1',
  name: id,
  status,
  activity: null,
  lastActivity,
  request: r,
});

function view(over: Partial<InboxItem> = {}): InboxItem {
  return {
    key: 'k',
    source: 'ticketReview',
    group: 'today',
    projectId: 'p1',
    projectName: 'facturation',
    private: false,
    title: 'FAC-38 · Avoirs partiels',
    since: NOW - 2 * 60 * MIN,
    agentId: null,
    agentName: null,
    ticketId: 't1',
    ticketKey: 'FAC-38',
    snoozedUntil: null,
    request: null,
    column: 'review',
    blocked: null,
    conflict: false,
    partial: false,
    decision: null,
    files: 0,
    number: null,
    url: null,
    word: null,
    mark: null,
    ...over,
  };
}

function space(id: string, g: ReturnType<typeof glance>, box: Partial<SpaceBox> | null = null): PhoneSpace {
  return {
    id,
    name: id === 'here' ? 'Ce poste' : 'Travail',
    here: id === 'here',
    conn: { state: 'open' } as never,
    glance: g as never,
    at: NOW,
    box: box ? { enabled: true, asks: [], views: [], at: NOW, ...box } : null,
  };
}

const keys = (ls: Line[]) => ls.map((l) => l.key);

describe('the thread of « À toi »', () => {
  it('puts the stopped agents first, the longest waiting first, over every space', () => {
    const th = threadOf(
      [
        space('here', glance({ projects: [project], agents: [agent('a2', NOW - 4 * MIN), agent('r', NOW, null, 'running')] })),
        space(
          'travail',
          glance({
            projects: [project],
            agents: [
              agent(
                'a1',
                NOW - 12 * MIN,
                request({ kind: 'question', questions: [{ question: 'Garder les colonnes ?', options: ['Oui', 'Non'] }] }),
              ),
            ],
          }),
        ),
      ],
      NOW,
      'Approuver le plan',
    );
    expect(keys(th.stopped)).toEqual(['travail:agent:a1', 'here:agent:a2']);
    expect(th.stopped[0]).toMatchObject({ title: 'Garder les colonnes ?', stop: 'question', space: { name: 'Travail' }, project });
    expect(th.stopped[1]).toMatchObject({ title: 'Supprime le dossier build', stop: 'permission' });
  });

  it('answers a stopped agent through its view of the box when there is one, and takes its request whole', () => {
    const whole = request({ arg: 'x'.repeat(1500) });
    const v = view({
      key: 'perm:a1',
      source: 'permission',
      group: 'stopped',
      agentId: 'a1',
      agentName: 'tests-e2e',
      request: whole,
      since: NOW - 5 * MIN,
    });
    const th = threadOf(
      [
        space('here', glance({ projects: [project], agents: [agent('a1', NOW - 5 * MIN, request({ arg: 'x'.repeat(400), cut: true }))] }), {
          views: [v],
        }),
      ],
      NOW,
      'Approuver le plan',
    );
    expect(th.stopped).toHaveLength(1);
    expect(th.stopped[0]).toMatchObject({ key: 'here:agent:a1', agentName: 'tests-e2e', view: v, request: whole });
  });

  it('ranges the requests and the views as the desktop does: today, earlier, put off, the squad lead’s, answered', () => {
    const today = windowAsk({ id: 'k1', createdAt: NOW - 40 * MIN, title: 'Choisis la direction' });
    const urgent = windowAsk({ id: 'k2', createdAt: NOW - 20 * MIN, urgency: 'today', title: 'Publier ?' });
    const old = windowAsk({ id: 'k3', createdAt: NOW - 3 * 86_400_000, title: 'Ancienne' });
    const later = windowAsk({ id: 'k4', state: 'snoozed', snoozedUntil: NOW + 60 * MIN });
    const answered = windowAsk({ id: 'k5', state: 'answered', answer: { at: NOW - MIN } as never });
    const ticket = view({ key: 'rev:t1' });
    const claimed = view({
      key: 'rev:t2',
      mark: { kind: 'claimed', note: 'vérification en cours', by: null, byName: 'chef', at: NOW, sign: '' },
    });
    const settled = view({ key: 'rev:t3', mark: { kind: 'settled', note: 'fusionné', by: null, byName: 'chef', at: NOW, sign: '' } });
    const th = threadOf(
      [
        space('travail', glance({ projects: [project] }), {
          asks: [today, urgent, old, later, answered],
          views: [ticket, claimed, settled],
        }),
      ],
      NOW,
      '',
    );
    // An urgent one first, then the oldest first, then the views of the group.
    expect(keys(th.today)).toEqual(['travail:ask:k2', 'travail:ask:k1', 'travail:view:rev:t1']);
    expect(keys(th.earlier)).toEqual(['travail:ask:k3']);
    expect(keys(th.later)).toEqual(['travail:ask:k4']);
    expect(keys(th.claimed)).toEqual(['travail:view:rev:t2']);
    // The latest first: settled now, answered a minute ago.
    expect(keys(th.recent)).toEqual(['travail:view:rev:t3', 'travail:ask:k5']);
    expect(keys(waitingLines(th))).toEqual(['travail:ask:k2', 'travail:ask:k1', 'travail:view:rev:t1', 'travail:ask:k3']);
  });

  it('shows the requests the glance names when the box is not read (cut, or not yet): by their day', () => {
    const th = threadOf(
      [
        space(
          'travail',
          glance({
            projects: [project],
            asks: [
              { id: 'k1', projectId: 'p1', title: 'Aujourd’hui', createdAt: NOW - MIN },
              { id: 'k2', projectId: 'p1', title: 'Avant', createdAt: NOW - 2 * 86_400_000 },
            ],
          }),
        ),
      ],
      NOW,
      '',
    );
    expect(th.today.map((l) => l.title)).toEqual(['Aujourd’hui']);
    expect(th.earlier.map((l) => l.title)).toEqual(['Avant']);
    expect(th.today[0]).toMatchObject({ kind: 'ask', ask: null });
  });

  it('shows nothing of a space that gives its numbers alone', () => {
    const th = threadOf([space('travail', glance({ countsOnly: true, waiting: 3, agents: [agent('a', 1)] }))], NOW, '');
    expect(waitingLines(th)).toEqual([]);
  });

  it('names what an agent stopped for', () => {
    expect(stopOf(request({ tool: 'ExitPlanMode' }))).toBe('plan');
    expect(stopOf(request({ kind: 'question' }))).toBe('question');
    expect(stoppedTitle(request({ tool: 'ExitPlanMode' }), 'Approuver le plan')).toBe('Approuver le plan');
    expect(stoppedTitle(request({ description: null, arg: 'npm test\nnpm run lint' }), '')).toBe('Bash : npm test');
  });
});

describe('a line that changes stays in its place', () => {
  it('keeps the order the lines had, and puts a new one where the reading puts it', () => {
    expect(steady([], ['a', 'b', 'c'])).toEqual(['a', 'b', 'c']);
    // « b » moved to the front in the reading: it stays where it was.
    expect(steady(['a', 'b', 'c'], ['b', 'a', 'c'])).toEqual(['a', 'b', 'c']);
    // A new one after the line it follows; one gone is gone.
    expect(steady(['a', 'b', 'c'], ['a', 'x', 'c'])).toEqual(['a', 'x', 'c']);
    expect(steady(['a', 'c'], ['n', 'a', 'c'])).toEqual(['n', 'a', 'c']);
    expect(inOrder([{ key: 'c' }, { key: 'a' }], ['a', 'c']).map((l) => l.key)).toEqual(['a', 'c']);
  });

  it('keeps its dot until it is seen in the state it is in; a request of the box says it itself', () => {
    const [l] = threadOf([space('here', glance({ projects: [project], agents: [agent('a1', NOW - MIN)] }))], NOW, '').stopped;
    expect(unseen(l, {})).toBe(true);
    expect(unseen(l, { [l.key]: l.mark })).toBe(false);
    // Another request of the same agent: a new mark, the dot again.
    const [l2] = threadOf(
      [space('here', glance({ projects: [project], agents: [agent('a1', NOW, request({ id: 'req_2' }))] }))],
      NOW,
      '',
    ).stopped;
    expect(unseen(l2, { [l.key]: l.mark })).toBe(true);
    const ask = (seen: boolean, woke = false) =>
      threadOf([space('t', glance(), { asks: [windowAsk({ seen, woke, createdAt: NOW })] })], NOW, '').today[0];
    expect(unseen(ask(false), {})).toBe(true);
    expect(unseen(ask(true), {})).toBe(false);
    expect(unseen(ask(true, true), {})).toBe(true);
  });

  it('opens the line after the one settled, else the one before', () => {
    expect(nextAfter(['a', 'b', 'c'], 'b', ['a', 'b', 'c'])).toBe('c');
    expect(nextAfter(['a', 'b', 'c'], 'c', ['a', 'b', 'c'])).toBe('b');
    expect(nextAfter(['a'], 'a', ['a'])).toBeNull();
  });
});

describe('what a space counts', () => {
  it('counts as the desktop’s box once the box is read: the stopped, the requests open, the views that wait', () => {
    const g = glance({ waiting: 2, projects: [project], agents: [agent('a1', 1), agent('a2', 2)] });
    expect(waitingIn(space('t', g))).toBe(2);
    const s = space('t', g, {
      asks: [windowAsk({ id: 'k1' }), windowAsk({ id: 'k2', state: 'snoozed' })],
      views: [
        view({ key: 'rev' }),
        view({ key: 'later', snoozedUntil: NOW + MIN }),
        // An agent whose request the squad lead took in hand no longer counts.
        view({
          key: 'perm',
          group: 'stopped',
          source: 'permission',
          agentId: 'a2',
          mark: { kind: 'claimed', note: '', by: null, byName: 'chef', at: 1, sign: '' },
        }),
      ],
    });
    expect(waitingIn(s)).toBe(3);
  });
});
