import { beforeEach, describe, expect, it } from 'vitest';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { askFile, windowAsk } from '../../test/inbox';
import { app, asksUser } from '../state.svelte';
import {
  actionable,
  fLater,
  groups,
  inbox,
  inboxCount,
  isLight,
  itemKey,
  laterTimes,
  readFile,
  RECENT_MAX,
  startOption,
  stoppedAgents,
  stoppedWhat,
} from './inbox.svelte';

const HOUR = 3600_000;
const DAY = 24 * HOUR;

describe('the queue of the box', () => {
  const now = new Date(2026, 10, 17, 15, 0, 0).getTime();

  it('puts the stopped agents first (the oldest first), then today, earlier, later and recently answered', () => {
    const asks = [
      windowAsk({ id: 'a-1', createdAt: now - 3 * DAY }),
      windowAsk({ id: 'a-2', createdAt: now - HOUR }),
      windowAsk({ id: 'a-3', createdAt: now - 4 * DAY, urgency: 'today' }),
      windowAsk({ id: 'a-4', createdAt: now - 5 * DAY, woke: true }),
      windowAsk({ id: 'a-5', state: 'snoozed', snoozedUntil: now + DAY }),
      windowAsk({ id: 'a-6', state: 'read', answer: { at: now - HOUR } as never }),
      windowAsk({ id: 'a-7', state: 'withdrawn', closedAt: now - 8 * DAY }),
      windowAsk({ id: 'a-8', createdAt: now - 2 * DAY, tickets: ['t1'] }),
    ];
    const stopped = [
      agent({ id: 'x', status: 'waiting', lastActivity: now - 100 }),
      agent({ id: 'y', status: 'waiting', lastActivity: now - 900 }),
    ];
    const g = groups(asks, stopped, now);
    expect(g.stopped.map(itemKey)).toEqual(['agent:y', 'agent:x']);
    // « Aujourd'hui »: urgent today first, back from « Plus tard », then the oldest first.
    expect(g.today.map(itemKey)).toEqual(['a-3', 'a-4', 'a-8', 'a-2']);
    expect(g.earlier.map(itemKey)).toEqual(['a-1']);
    expect(g.later.map((a) => a.id)).toEqual(['a-5']);
    // Closed more than 7 days ago: out of « Répondu récemment ».
    expect(g.recent.map((a) => a.id)).toEqual(['a-6']);
    expect(actionable(g).map(itemKey)).toEqual(['agent:y', 'agent:x', 'a-3', 'a-4', 'a-8', 'a-2', 'a-1']);
  });

  it('keeps 50 answers at most in « Répondu récemment », the latest first', () => {
    const asks = Array.from({ length: 60 }, (_, i) =>
      windowAsk({ id: `a-${i}`, state: 'answered', answer: { at: now - i * 1000 } as never }),
    );
    const g = groups(asks, [], now);
    expect(g.recent).toHaveLength(RECENT_MAX);
    expect(g.recent[0].id).toBe('a-0');
  });
});

describe('the count of the box', () => {
  beforeEach(() => resetApp());

  it('counts the requests open and the agents stopped of « en attente », never a request put off', () => {
    resetApp({
      agents: [
        agent({ id: 'w1', status: 'waiting' }),
        agent({ id: 'w2', status: 'running', deleteAsk: { by: 'w1', at: 1, worktree: null } as never }),
        agent({ id: 'w3', status: 'waiting', archived: true }),
        agent({ id: 'r1', status: 'running' }),
      ],
    });
    // The very agents the status bar counts as « en attente » (`asksUser`, the archived ones aside).
    const waiting = Object.values(app.agents)
      .filter((x) => !x.archived && asksUser(x))
      .map((x) => x.id);
    expect(stoppedAgents(app.agents).map((x) => x.id)).toEqual(waiting);
    expect(waiting).toEqual(['w1', 'w2']);
    const c = inboxCount(
      [windowAsk({ id: 'a-1' }), windowAsk({ id: 'a-2', state: 'snoozed' }), windowAsk({ id: 'a-3', state: 'answered' })],
      stoppedAgents(app.agents),
    );
    expect(c).toEqual({ asks: 1, stopped: 2, total: 3 });
  });

  it('says what a stopped agent waits for', () => {
    expect(stoppedWhat(agent({ status: 'waiting', requests: [{ kind: 'question', tool: 'AskUserQuestion' } as never] }))).toBe('question');
    expect(stoppedWhat(agent({ status: 'waiting', requests: [{ kind: 'permission', tool: 'ExitPlanMode' } as never] }))).toBe('plan');
    expect(stoppedWhat(agent({ status: 'waiting', requests: [{ kind: 'permission', tool: 'Bash' } as never] }))).toBe('permission');
    expect(stoppedWhat(agent({ status: 'running', deleteAsk: { by: 'x', at: 1, worktree: null } as never }))).toBe('delete');
  });
});

describe('« Plus tard »', () => {
  it('offers an hour, this evening before 17:00, tomorrow and Monday at 9:00', () => {
    // Tuesday 17 November 2026, 15:00.
    const tue = new Date(2026, 10, 17, 15, 0, 0).getTime();
    const times = laterTimes(tue);
    expect(times.map((x) => x.key)).toEqual(['hour', 'evening', 'tomorrow', 'monday']);
    expect(new Date(times[1].at).getHours()).toBe(18);
    expect(new Date(times[2].at).getDate()).toBe(18);
    expect(new Date(times[2].at).getHours()).toBe(9);
    expect(new Date(times[3].at).getDay()).toBe(1);
    expect(new Date(times[3].at).getDate()).toBe(23);
  });

  it('says a time near now: today, tomorrow, a day of the week, a date', () => {
    const tue = new Date(2026, 10, 17, 15, 0, 0).getTime();
    expect(fLater(new Date(2026, 10, 17, 18, 0).getTime(), tue)).toBe('18:00');
    expect(fLater(new Date(2026, 10, 18, 9, 0).getTime(), tue)).toBe('demain 09:00');
    expect(fLater(new Date(2026, 10, 23, 9, 0).getTime(), tue)).toBe('lundi 09:00');
    expect(fLater(new Date(2026, 11, 2, 9, 0).getTime(), tue)).toBe('2 décembre 09:00');
  });

  it('drops this evening after 17:00, and Monday when tomorrow is Monday', () => {
    const sun = new Date(2026, 10, 22, 19, 0, 0).getTime();
    expect(laterTimes(sun).map((x) => x.key)).toEqual(['hour', 'tomorrow']);
  });
});

describe('an answer starts', () => {
  it('on the recommendation, never chosen for the user, else on the first option', () => {
    expect(startOption(windowAsk())).toBe('b');
    expect(startOption(windowAsk({ recommend: null }))).toBe('a');
  });

  it('tells a light render from a dark one by its name', () => {
    expect(isLight(askFile({ name: 'a-clair.png' }))).toBe(true);
    expect(isLight(askFile({ name: 'b_light.webp' }))).toBe(true);
    expect(isLight(askFile({ name: 'a.png' }))).toBe(false);
    expect(isLight(askFile({ name: 'eclairage.png' }))).toBe(false);
  });
});

describe('the files of a request', () => {
  beforeEach(() => resetApp());

  it('are read from the core’s copy by parts, never from a path', async () => {
    const bytes = Uint8Array.from({ length: 10 }, (_, i) => i);
    const ipc = fakeBackend({
      inbox_file: (a: { offset: number }) => ({
        name: 'x.bin',
        kind: 'other',
        mediaType: 'application/octet-stream',
        size: 10,
        offset: a.offset,
        data: btoa(String.fromCharCode(...bytes.subarray(a.offset, a.offset + 4))),
      }),
    });
    const got = await readFile('a-1', { file: 'u1/x.bin', size: 10 });
    expect([...got]).toEqual([...bytes]);
    expect(ipc.called('inbox_file').map((c) => c.args.offset)).toEqual([0, 4, 8]);
    expect(ipc.called('inbox_file').every((c) => c.args.id === 'a-1' && c.args.file === 'u1/x.bin')).toBe(true);
  });

  it('reads the box again and holds nothing once the server is off', async () => {
    fakeBackend({ inbox_list: () => ({ enabled: true, asks: [windowAsk()], open: 1 }) });
    await inbox.load();
    expect(inbox.enabled).toBe(true);
    expect(inbox.asks).toHaveLength(1);
    fakeBackend({ inbox_list: () => ({ enabled: false, asks: [], open: 0 }) });
    await inbox.load();
    expect(inbox.asks).toEqual([]);
    inbox.select('a-1');
    inbox.clear();
    expect(inbox.selected).toBeNull();
  });
});
