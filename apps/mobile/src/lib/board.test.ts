// The phone's Kanban (MO-7): what its segments, lines and gestures read of a board, the project chosen and kept, and
// each gesture through its space's connection, the lock asked first for « Valider », « Lancer » and « L'agent résout ».
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '$shared/lib/i18n';
import { FakeSocket, tick } from '../test/fake-socket';
import {
  approve,
  chooseFrom,
  firstColumn,
  gesturesOf,
  kanban,
  readProjects,
  reject,
  resolve,
  segmentsOf,
  shownColumn,
  start,
  type BoardGlance,
  type BoardTicket,
} from './board.svelte';
import { useLock } from './lock';
import { ALL, phone, startSpaces, stopSpaces } from './phone-spaces.svelte';

const counts = (over: Partial<BoardGlance['counts']> = {}) => ({ todo: 0, doing: 0, review: 0, pr: 0, done: 0, ...over });

const ticket = (over: Partial<BoardTicket> = {}): BoardTicket => ({
  id: 't1',
  key: 'FAC-38',
  title: 'Avoirs partiels',
  column: 'review',
  agent: 'tests-e2e',
  iteration: 3,
  maxLoops: 5,
  criteria: [true, true, false],
  partial: false,
  blocked: null,
  conflict: false,
  step: null,
  ...over,
});

const facturation = { id: 'p1', name: 'facturation', color: 'oklch(0.72 0.12 250)' };
const relances = { id: 'p2', name: 'relances', color: 'oklch(0.72 0.12 48)' };

describe('what the screen reads', () => {
  it('shows « En revue » only when the project holds one', () => {
    expect(segmentsOf(counts())).toEqual(['todo', 'doing', 'review', 'done']);
    expect(segmentsOf(counts({ pr: 1 }))).toEqual(['todo', 'doing', 'review', 'pr', 'done']);
  });

  it('opens on what waits for the user, else what is under way, else what is to do', () => {
    expect(firstColumn(counts({ review: 2, doing: 1 }))).toBe('review');
    expect(firstColumn(counts({ doing: 1, todo: 4 }))).toBe('doing');
    expect(firstColumn(counts({ done: 3 }))).toBe('todo');
    // A column chosen stays while it is a segment.
    expect(shownColumn(counts({ review: 2 }), 'done')).toBe('done');
    expect(shownColumn(counts({ review: 2 }), 'pr')).toBe('review');
  });

  it('offers validate and send back to a ticket to test, start to one to do, the agent to a conflict, nothing during its validation', () => {
    expect(gesturesOf(ticket())).toEqual(['reject', 'approve']);
    expect(gesturesOf(ticket({ column: 'todo' }))).toEqual(['start']);
    expect(gesturesOf(ticket({ conflict: true, blocked: 'Conflit' }))).toEqual(['resolve']);
    expect(gesturesOf(ticket({ step: 'Merge…' }))).toEqual([]);
    expect(gesturesOf(ticket({ column: 'doing' }))).toEqual([]);
    expect(gesturesOf(ticket({ column: 'done' }))).toEqual([]);
  });

  it('keeps the project chosen while its space has it, the first project of the first space else', () => {
    const lists = { here: [], travail: [facturation, relances] };
    expect(chooseFrom(lists, ['here', 'travail'], null)).toEqual({ space: 'travail', project: 'p1' });
    expect(chooseFrom(lists, ['here', 'travail'], { space: 'travail', project: 'p2' })).toEqual({ space: 'travail', project: 'p2' });
    expect(chooseFrom(lists, ['here', 'travail'], { space: 'travail', project: 'gone' })).toEqual({ space: 'travail', project: 'p1' });
    expect(chooseFrom({ here: [] }, ['here'], null)).toBeNull();
  });
});

describe('the gestures, through the space’s connection', () => {
  let sockets: FakeSocket[];
  let asked: string[];
  let answer: 'ok' | 'cancel';

  const board = (tickets: BoardTicket[]): BoardGlance => ({
    projects: [facturation, relances],
    projectId: 'p1',
    counts: counts({
      review: tickets.filter((x) => x.column === 'review').length,
      todo: tickets.filter((x) => x.column === 'todo').length,
    }),
    tickets,
  });

  beforeEach(async () => {
    setLang('fr');
    localStorage.clear();
    sockets = [];
    asked = [];
    answer = 'ok';
    phone.spaces = [];
    phone.filter = ALL;
    Object.assign(kanban, { choice: null, lists: {}, board: null, column: null, open: null, rejecting: null, busy: [], error: null });
    useLock({
      unlock: async ({ reason }: { reason: string }) => {
        asked.push(reason);
        if (answer === 'cancel') throw new Error('cancelled');
      },
      lockState: async () => ({ screenLock: true, biometry: 'face' }),
    } as never);
    startSpaces([{ id: 'travail', name: 'Travail', here: false, url: 'ws://127.0.0.1:2/remote', token: 'b' }], {
      open: (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      delays: [10_000],
      every: 60_000,
    });
    sockets[0].welcome();
    await tick();
  });
  afterEach(() => {
    stopSpaces();
    useLock(null);
  });

  /** The projects read, then the board of the first one. */
  async function open(tickets: BoardTicket[]) {
    const read = readProjects();
    await tick();
    sockets[0].answer('board_glance', board(tickets));
    await tick();
    sockets[0].answer('board_glance', board(tickets));
    await read;
  }

  it('reads the space’s projects, shows the first one and keeps the choice on the phone', async () => {
    await open([ticket()]);
    expect(kanban.choice).toEqual({ space: 'travail', project: 'p1' });
    expect(kanban.lists.travail).toEqual([facturation, relances]);
    expect(kanban.board?.tickets[0].key).toBe('FAC-38');
    const args = sockets[0].calls().filter((c) => c.cmd === 'board_glance');
    expect(args[0].args).toEqual({});
    expect(args[1].args).toEqual({ projectId: 'p1' });
  });

  it('validates under the lock: the reason names the ticket and the space, then ticket_approve', async () => {
    await open([ticket()]);
    const done = approve(ticket());
    await tick();
    expect(asked).toEqual(['Valider FAC-38 ? · Escouade · Travail']);
    expect(kanban.busy).toEqual(['t1']);
    sockets[0].answer('ticket_approve', null);
    await tick();
    sockets[0].answer('board_glance', board([]));
    expect(await done).toBe(true);
    expect(sockets[0].calls().find((c) => c.cmd === 'ticket_approve')?.args).toEqual({ id: 't1' });
    expect(kanban.busy).toEqual([]);
  });

  it('sends back another ticket while a validation still waits for its answer (its merge, its cleaning)', async () => {
    await open([ticket(), ticket({ id: 't2', key: 'FAC-40' })]);
    const approving = approve(ticket());
    await tick();
    const sent = reject(ticket({ id: 't2', key: 'FAC-40' }), 'Ajoute un test');
    await tick();
    expect(sockets[0].calls().find((c) => c.cmd === 'ticket_reject')?.args).toEqual({ id: 't2', comment: 'Ajoute un test' });
    expect(kanban.busy).toEqual(['t1', 't2']);
    // A second « Valider » of the same ticket meanwhile goes nowhere.
    expect(await approve(ticket())).toBe(false);
    sockets[0].answer('ticket_reject', null);
    sockets[0].answer('ticket_approve', null);
    await tick();
    sockets[0].answer('board_glance', board([]));
    expect(await sent).toBe(true);
    expect(await approving).toBe(true);
    expect(sockets[0].calls().filter((c) => c.cmd === 'ticket_approve')).toHaveLength(1);
    expect(kanban.busy).toEqual([]);
  });

  it('a lock cancelled sends nothing', async () => {
    await open([ticket()]);
    answer = 'cancel';
    expect(await approve(ticket())).toBe(false);
    expect(await start(ticket({ column: 'todo' }))).toBe(false);
    expect(sockets[0].calls().some((c) => c.cmd === 'ticket_approve' || c.cmd === 'ticket_start')).toBe(false);
  });

  it('sends back with the word said, without the lock; an empty word sends nothing', async () => {
    await open([ticket()]);
    expect(await reject(ticket(), '   ')).toBe(false);
    const done = reject(ticket(), '  Ajoute un test du cas sans TVA ');
    await tick();
    sockets[0].answer('ticket_reject', null);
    await tick();
    sockets[0].answer('board_glance', board([]));
    expect(await done).toBe(true);
    expect(asked).toEqual([]);
    expect(sockets[0].calls().find((c) => c.cmd === 'ticket_reject')?.args).toEqual({
      id: 't1',
      comment: 'Ajoute un test du cas sans TVA',
    });
  });

  it('starts a ticket to do and lets the agent resolve a conflict, each under the lock', async () => {
    await open([ticket({ column: 'todo' })]);
    const started = start(ticket({ id: 't2', key: 'FAC-41', column: 'todo' }));
    await tick();
    sockets[0].answer('ticket_start', null);
    await tick();
    sockets[0].answer('board_glance', board([]));
    expect(await started).toBe(true);
    const resolved = resolve(ticket({ id: 't3', key: 'FAC-40', conflict: true }));
    await tick();
    sockets[0].answer('ticket_resolve_conflict', null);
    await tick();
    sockets[0].answer('board_glance', board([]));
    expect(await resolved).toBe(true);
    expect(asked).toEqual(['Lancer FAC-41 ? · Escouade · Travail', 'Laisser l’agent résoudre le conflit de FAC-40 ? · Escouade · Travail']);
    expect(sockets[0].calls().find((c) => c.cmd === 'ticket_start')?.args).toEqual({ id: 't2' });
    expect(sockets[0].calls().find((c) => c.cmd === 'ticket_resolve_conflict')?.args).toEqual({ id: 't3' });
  });

  it('says what a gesture failed on', async () => {
    await open([ticket()]);
    const done = approve(ticket());
    await tick();
    sockets[0].answer('ticket_approve', null, 'Le merge a échoué');
    await tick();
    sockets[0].answer('board_glance', board([ticket()]));
    expect(await done).toBe(false);
    expect(kanban.error).toBe('FAC-38 : Le merge a échoué');
  });
});
