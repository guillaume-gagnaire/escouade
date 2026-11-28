// The phone's gestures (MO-5): each to the core of its own space, through the box's view when there is one, under the
// system's lock when it makes an agent act (M3), never sent when the lock was cancelled.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { InboxItem } from '$shared/lib/inbox/views';
import { windowAsk } from '$shared/test/inbox';
import { FakeSocket, tick } from '../test/fake-socket';
import { answerAsk, answerQuestion, answerStop, actOnView, failText, snooze, viewActLocked } from './act';
import { useLock } from './lock';
import { ALL, phone, startSpaces, stopSpaces } from './phone-spaces.svelte';
import type { StoppedLine } from './thread';

const travail = { id: 'travail', name: 'Travail', here: false };

describe('the phone’s gestures', () => {
  let sockets: FakeSocket[];
  let lock: 'ok' | 'cancel';
  const reasons: string[] = [];
  const sock = () => sockets.find((s) => s.url.includes('7002'))!;
  const other = () => sockets.find((s) => s.url.includes('7001'))!;
  /** The commands the space was sent, but its glances and box readings. */
  const sent = (s = sock()) => s.calls().filter((c) => !['space_glance', 'inbox_list', 'inbox_views'].includes(c.cmd));

  beforeEach(async () => {
    localStorage.clear();
    sockets = [];
    lock = 'ok';
    reasons.length = 0;
    phone.spaces = [];
    phone.filter = ALL;
    useLock({
      unlock: async ({ reason }) => {
        reasons.push(reason);
        if (lock === 'cancel') throw Object.assign(new Error('cancelled'), { code: 'Cancelled' });
      },
      lockState: async () => ({ screenLock: true, biometry: 'face' }),
    });
    startSpaces(
      [
        { id: 'here', name: 'Ce poste', here: true, url: 'ws://127.0.0.1:7001/a/remote', token: 't1' },
        { id: 'travail', name: 'Travail', here: false, url: 'ws://127.0.0.1:7002/b/remote', token: 't2' },
      ],
      {
        open: (url) => {
          const s = new FakeSocket(url);
          sockets.push(s);
          return s;
        },
        delays: [10_000],
        every: 60_000,
        version: '2.3.0',
      },
    );
    sockets.forEach((s) => s.welcome());
    await tick();
  });
  afterEach(() => {
    stopSpaces();
    useLock(null);
  });

  /** Answers the last call `cmd` of the space with `ok`. */
  async function reply(cmd: string, ok: unknown = null) {
    await tick();
    const c = sock()
      .calls()
      .filter((x) => x.cmd === cmd)
      .at(-1)!;
    sock().json({ t: 'reply', id: c.id, ok });
  }

  const line = (over: Partial<StoppedLine> = {}): StoppedLine => ({
    kind: 'stopped',
    key: 'travail:agent:a1',
    space: travail,
    project: null,
    title: 'Lancer les tests',
    since: 1,
    mark: '',
    agentId: 'a1',
    agentName: 'tests-e2e',
    stop: 'permission',
    request: {
      id: 'req_perm',
      kind: 'permission',
      tool: 'Bash',
      arg: 'npm test',
      description: null,
      reason: null,
      questions: [],
      cut: false,
      defaultNo: false,
    },
    view: null,
    ...over,
  });

  it('« Autoriser »: the system’s lock first, saying what goes and where, then the answer to its space alone', async () => {
    const done = answerStop(line(), 'allow');
    await reply('answer_permission');
    expect(await done).toEqual({ ok: true });
    expect(reasons).toEqual(['Autoriser la commande de tests-e2e ? · Escouade · Travail']);
    expect(sent()).toMatchObject([
      { cmd: 'answer_permission', args: { id: 'a1', requestId: 'req_perm', decision: 'allow', message: null } },
    ]);
    expect(sent(other())).toEqual([]);
  });

  it('the lock cancelled: nothing goes', async () => {
    lock = 'cancel';
    expect(await answerStop(line(), 'allow')).toEqual({ ok: false, locked: true });
    await tick();
    expect(sent()).toEqual([]);
  });

  it('« Refuser » with a word needs no lock; through the box’s view when the line has one', async () => {
    const v = { key: 'perm:a1' } as InboxItem;
    const done = answerStop(line({ view: v }), 'deny', '  pas sur la prod ');
    await reply('inbox_view_act');
    expect(await done).toEqual({ ok: true });
    expect(reasons).toEqual([]);
    expect(sent()).toMatchObject([{ cmd: 'inbox_view_act', args: { key: 'perm:a1', act: { act: 'deny', message: 'pas sur la prod' } } }]);
  });

  it('a question answered on the spot, without the lock', async () => {
    const done = answerQuestion(line({ stop: 'question' }), { 'Quelle base ?': 'Postgres' });
    await reply('answer_question');
    expect(await done).toEqual({ ok: true });
    expect(reasons).toEqual([]);
    expect(sent()).toMatchObject([
      { cmd: 'answer_question', args: { id: 'a1', requestId: 'req_perm', answers: { 'Quelle base ?': 'Postgres' } } },
    ]);
  });

  it('a request of the box: said from the phone; a « Oui » to a go-ahead under the lock, a choice without', async () => {
    const choice = answerAsk(travail, windowAsk({ id: 'k1' }), { options: ['b'], note: 'la courbe' });
    await reply('inbox_answer', windowAsk());
    expect(await choice).toEqual({ ok: true });
    expect(reasons).toEqual([]);
    const gate = answerAsk(travail, windowAsk({ id: 'k2', kind: 'confirm', gate: true, yesLabel: 'Oui, publie' }), { yes: true });
    await reply('inbox_answer', windowAsk());
    expect(await gate).toEqual({ ok: true });
    expect(reasons).toEqual(['« Oui, publie » ? · Escouade · Travail']);
    expect(sent().map((c) => c.args)).toMatchObject([
      { id: 'k1', answer: { options: ['b'], note: 'la courbe', via: 'phone' }, files: [] },
      { id: 'k2', answer: { yes: true, via: 'phone' } },
    ]);
  });

  it('« Valider » a ticket under the lock; accepting a decision without', () => {
    const v = (source: InboxItem['source']) => ({ source }) as InboxItem;
    expect(viewActLocked(v('ticketReview'), 'approve')).toBe(true);
    expect(viewActLocked(v('scheduled'), 'accept')).toBe(true);
    expect(viewActLocked(v('decision'), 'accept')).toBe(false);
    expect(viewActLocked(v('ticketReview'), 'reject')).toBe(false);
  });

  it('a view refused because it was settled elsewhere says so', async () => {
    const done = actOnView(travail, { key: 'rev:t1', source: 'decision' } as InboxItem, { act: 'accept' }, 'Accepter');
    await tick();
    const c = sock()
      .calls()
      .filter((x) => x.cmd === 'inbox_view_act')
      .at(-1)!;
    sock().json({ t: 'reply', id: c.id, error: 'Réglé ailleurs : la vue n’est plus en attente.' });
    expect(await done).toEqual({ ok: false, error: 'Réglé ailleurs : la boîte s’est mise à jour.' });
  });

  it('« Plus tard » on a request or a view', async () => {
    const a = snooze(travail, { ask: windowAsk({ id: 'k1' }) }, 99);
    await reply('inbox_snooze');
    await a;
    const v = snooze(travail, { view: { key: 'rev:t1' } as InboxItem }, 98);
    await reply('inbox_view_snooze');
    await v;
    expect(sent().map((c) => [c.cmd, c.args])).toEqual([
      ['inbox_snooze', { id: 'k1', until: 99 }],
      ['inbox_view_snooze', { key: 'rev:t1', until: 98 }],
    ]);
  });

  it('a call cut before its answer says the answer may have gone', () => {
    expect(failText(new Error('connection lost'))).toContain('peut-être partie');
  });
});
