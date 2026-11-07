import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, CURSOR_ENGINE, fakeBackend, project, SETTINGS } from '../test/ipc';
import type { DraftAttachment } from './attachments';
import { forgetDraft, getDraft } from './drafts';
import { conversationOf } from './conversations.svelte';
import { cancelHeld, editHeld, heldOf, hold, holdModeOf, restoreHeld, sendNow, settled } from './outbox.svelte';
import { readPref } from './prefs';
import { app } from './state.svelte';
import type { Agent, InitialState, UiEvent } from './types';

const copied: DraftAttachment = {
  name: 'notes.log',
  mediaType: '',
  data: '',
  kind: 'file',
  size: 3,
  file: { name: 'notes.log', path: '/copies/notes.log', claudePath: '', size: 3, origin: false },
};
const original: DraftAttachment = {
  ...copied,
  name: 'plan.md',
  file: { name: 'plan.md', path: '/home/plan.md', claudePath: '', size: 3, origin: true },
};

describe('outbox', () => {
  let emit: (e: UiEvent) => void;
  let backend: ReturnType<typeof fakeBackend>;
  /** What `send_message` refuses, by text. */
  let refused: Set<string>;
  /** What `send_message` answers instead, when set. */
  let answer: Promise<void> | null;

  async function start(agents: Agent[], delay = 10) {
    let channel: { onmessage: (e: UiEvent) => void } | null = null;
    const initial: InitialState = {
      projects: [project()],
      agents,
      ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
      settings: { ...SETTINGS, midTurnDelaySecs: delay },
      usage: { todayCost: 0, accounts: [], current: { claude: 'principal' } },
      git: {},
      shells: [],
      terminals: [],
      tickets: [],
      claudeFound: true,
      version: '0.1.0',
      models: [],
      engines: [CLAUDE_ENGINE, CURSOR_ENGINE, { ...BARE_ENGINE, id: 'copilot', caps: { ...BARE_ENGINE.caps, midTurn: 'refused' } }],
    };
    backend = fakeBackend({
      subscribe: (args: any) => {
        channel = args.channel;
        return initial;
      },
      get_conversation: () => [],
      send_message: (args: any) => {
        if (answer) return answer;
        if (refused.has(args.text)) throw new Error('Claude Code s’est arrêté');
      },
    });
    await app.init();
    emit = (e) => channel!.onmessage(e);
  }

  /** The agent `id` as the backend says it is now. */
  const update = (id: string, over: Partial<Agent>) => emit({ type: 'agent', agent: { ...app.agents[id], ...over } });
  const sent = () => backend.called('send_message').map((c) => c.args.text);
  /** Lets the sends under way (each after the one before) reach the backend. */
  const settle = () => vi.advanceTimersByTimeAsync(0);
  const holdNow = (id: string, text: string, files: DraftAttachment[] = []) => hold(id, text, files, holdModeOf(app.agents[id])!);

  beforeEach(() => {
    localStorage.clear();
    refused = new Set();
    answer = null;
    for (const id of ['c1', 'k1']) forgetDraft(id);
    vi.useFakeTimers();
  });
  afterEach(() => {
    // Nothing held is left for the next test: given back, then forgotten.
    window.dispatchEvent(new Event('pagehide'));
    vi.useRealTimers();
  });

  const claude = () => agent({ id: 'c1', status: 'running' });
  const cursor = () => agent({ id: 'k1', engine: 'cursor', status: 'running' });

  it('holds a message sent to Claude during a turn and sends it ten seconds later', async () => {
    await start([claude()]);
    expect(holdModeOf(app.agents.c1)).toBe('delay');
    const held = holdNow('c1', 'x');
    expect(held.at).toBe(Date.now() + 10_000);
    expect(heldOf('c1')).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(9_999);
    expect(sent()).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(backend.called('send_message').map((c) => c.args)).toEqual([{ id: 'c1', text: 'x', attachments: [], files: [] }]);
    expect(heldOf('c1')).toEqual([]);
  });

  it('keeps counting down when the turn ends or is stopped', async () => {
    await start([claude()]);
    holdNow('c1', 'x');
    await vi.advanceTimersByTimeAsync(3_000);
    update('c1', { status: 'done' });
    await vi.advanceTimersByTimeAsync(6_999);
    expect(sent()).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(sent()).toEqual(['x']);
  });

  it('drops the messages held for an agent that is removed', async () => {
    await start([claude()]);
    holdNow('c1', 'x');
    emit({ type: 'agentRemoved', id: 'c1', projectId: 'p1' });
    expect(heldOf('c1')).toEqual([]);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(sent()).toEqual([]);
  });

  it('sends nothing held when the delay is off, nor to an agent at rest, nor for a harness that refuses', async () => {
    await start([claude(), agent({ id: 'c2' }), agent({ id: 'p1', engine: 'copilot', status: 'running' })], 0);
    expect(holdModeOf(app.agents.c1)).toBeNull();
    app.settings.midTurnDelaySecs = 10;
    expect(holdModeOf(app.agents.c1)).toBe('delay');
    expect(holdModeOf(app.agents.c2)).toBeNull();
    expect(holdModeOf(app.agents.p1)).toBeNull();
    // A worktree being prepared holds the message its own way.
    expect(holdModeOf({ ...app.agents.c1, setup: 'npm ci (1/2)' })).toBeNull();
  });

  it('keeps a message to Cursor until the end of the turn, one per end, whatever the delay', async () => {
    await start([cursor()], 0);
    expect(holdModeOf(app.agents.k1)).toBe('turnEnd');
    const first = holdNow('k1', 'un');
    holdNow('k1', 'deux');
    expect(first.at).toBeNull();
    await vi.advanceTimersByTimeAsync(60_000);
    expect(sent()).toEqual([]);
    update('k1', { status: 'done' });
    await settle();
    expect(sent()).toEqual(['un']);
    expect(heldOf('k1').map((h) => h.body)).toEqual(['deux']);
    update('k1', { status: 'running' });
    update('k1', { status: 'done' });
    await settle();
    expect(sent()).toEqual(['un', 'deux']);
  });

  it('still sends the next message to Cursor when the one sent at the end of the turn is refused', async () => {
    await start([cursor()]);
    refused.add('un');
    holdNow('k1', 'un');
    holdNow('k1', 'deux');
    update('k1', { status: 'done' });
    await settle();
    // Refused, it started no turn: the next goes now.
    expect(sent()).toEqual(['un', 'deux']);
    expect(getDraft('k1').text).toBe('un');
    expect(heldOf('k1')).toEqual([]);
    expect(holdModeOf(app.agents.k1)).toBeNull();
  });

  it('sends the older held messages first when one is sent now', async () => {
    await start([claude()]);
    holdNow('c1', 'A');
    const b = holdNow('c1', 'B');
    holdNow('c1', 'C');
    sendNow(b.id);
    await settle();
    expect(sent()).toEqual(['A', 'B']);
    expect(heldOf('c1').map((h) => h.body)).toEqual(['C']);
  });

  it('holds a message sent at rest behind one still held', async () => {
    await start([claude()]);
    holdNow('c1', 'A');
    update('c1', { status: 'done' });
    expect(holdModeOf(app.agents.c1)).toBe('delay');
  });

  it('gives an edited message back to the draft and sends nothing', async () => {
    await start([claude()]);
    const held = holdNow('c1', 'x', [original]);
    const focus = app.focusComposer;
    editHeld(held.id);
    expect(heldOf('c1')).toEqual([]);
    expect(getDraft('c1')).toEqual({ text: 'x', files: [original] });
    expect(app.focusComposer).toBe(focus + 1);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(sent()).toEqual([]);
  });

  it('drops a cancelled message and erases its copied files', async () => {
    await start([claude()]);
    const held = holdNow('c1', 'x', [copied, original]);
    cancelHeld(held.id);
    expect(heldOf('c1')).toEqual([]);
    expect(getDraft('c1').text).toBe('');
    expect(backend.called('attach_discard').map((c) => c.args)).toEqual([{ owner: 'c1', path: '/copies/notes.log' }]);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(sent()).toEqual([]);
  });

  it('never sends to an archived agent: its messages go back to its draft', async () => {
    await start([claude(), cursor()]);
    holdNow('k1', 'fin de tour');
    holdNow('c1', 'délai');
    update('k1', { status: 'idle', archived: true });
    expect(heldOf('k1')).toEqual([]);
    expect(getDraft('k1').text).toBe('fin de tour');
    update('c1', { archived: true });
    await vi.advanceTimersByTimeAsync(10_000);
    expect(sent()).toEqual([]);
    expect(getDraft('c1').text).toBe('délai');
  });

  it('keeps what it holds in the preferences, given back to the drafts at the next start', async () => {
    await start([claude(), cursor()]);
    const a = holdNow('c1', 'A');
    holdNow('c1', 'B');
    holdNow('k1', 'K');
    expect(JSON.parse(readPref('held')!)).toEqual({ c1: 'A\n\nB', k1: 'K' });
    cancelHeld(a.id);
    expect(JSON.parse(readPref('held')!)).toEqual({ c1: 'B', k1: 'K' });
    // The app stopped without the window being told: the next start finds them there.
    restoreHeld();
    expect(getDraft('c1').text).toBe('B');
    expect(getDraft('k1').text).toBe('K');
    expect(readPref('held')).toBeNull();
  });

  it('never sends a message before the one held before it, the delay shortened meanwhile', async () => {
    await start([claude()]);
    holdNow('c1', 'A');
    app.settings.midTurnDelaySecs = 0;
    holdNow('c1', 'B');
    await settle();
    expect(sent()).toEqual([]);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(sent()).toEqual(['A', 'B']);
  });

  it('writes the held messages to the draft when the window goes', async () => {
    await start([claude(), cursor()]);
    holdNow('c1', 'A');
    holdNow('c1', 'B');
    holdNow('k1', 'K');
    window.dispatchEvent(new Event('pagehide'));
    expect(readPref('draft.c1')).toBe('A\n\nB');
    expect(readPref('draft.k1')).toBe('K');
    expect(heldOf('c1')).toEqual([]);
  });

  it('tells the backend which agents it holds a message for', async () => {
    await start([claude(), cursor()]);
    const held = () => backend.called('set_held').map((c) => c.args.agents);
    const x = holdNow('c1', 'x');
    holdNow('c1', 'y');
    holdNow('k1', 'K');
    expect(held()).toEqual([['c1'], ['c1', 'k1']]);
    cancelHeld(x.id);
    expect(held()).toHaveLength(2);
    update('k1', { status: 'done' });
    await settle();
    expect(held().at(-1)).toEqual(['c1']);
  });

  it('keeps what it holds when the app warns of a restart: the backend waits for it', async () => {
    await start([claude()]);
    holdNow('c1', 'x');
    emit({ type: 'updateRestart', at: Date.now() + 30_000 });
    expect(heldOf('c1').map((h) => h.body)).toEqual(['x']);
  });

  it('still sends the next message when what tells a refusal throws', async () => {
    await start([claude()]);
    refused.add('A');
    vi.spyOn(app, 'toast').mockImplementationOnce(() => {
      throw new Error('toast');
    });
    holdNow('c1', 'A');
    const b = holdNow('c1', 'B');
    sendNow(b.id);
    await settle();
    expect(sent()).toEqual(['A', 'B']);
    await expect(settled('c1')).resolves.toBeUndefined();
  });

  it('takes down the missing account card once a held message goes', async () => {
    await start([claude()]);
    conversationOf('c1').noAccount = { engine: 'claude', text: 'Aucun compte' };
    holdNow('c1', 'x');
    await vi.advanceTimersByTimeAsync(10_000);
    expect(sent()).toEqual(['x']);
    expect(conversationOf('c1').noAccount).toBeNull();
  });

  it('says nothing of a refusal that comes back once its agent is deleted', async () => {
    await start([claude()]);
    let refuse = (_: Error) => {};
    answer = new Promise((_, reject) => (refuse = reject));
    const x = holdNow('c1', 'x');
    sendNow(x.id);
    await settle();
    emit({ type: 'agentRemoved', id: 'c1', projectId: 'p1' });
    const toasts = app.toasts.length;
    refuse(new Error('Agent introuvable'));
    await vi.advanceTimersByTimeAsync(1_000);
    expect(app.toasts).toHaveLength(toasts);
    expect(readPref('draft.c1')).toBeNull();
  });

  it('gives a refused message back and still sends the next one', async () => {
    await start([claude()]);
    refused.add('A');
    const a = holdNow('c1', 'A');
    holdNow('c1', 'B');
    sendNow(heldOf('c1')[1].id);
    await settle();
    expect(sent()).toEqual(['A', 'B']);
    expect(getDraft('c1').text).toBe('A');
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error' });
    expect(app.toasts.at(-1)?.text).toContain('Claude Code s’est arrêté');
    expect(a.id).not.toBe(heldOf('c1')[0]?.id);
  });
});
