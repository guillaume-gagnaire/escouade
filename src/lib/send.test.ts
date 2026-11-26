import { afterEach, describe, expect, it } from 'vitest';
import { agent, fakeBackend, project, SETTINGS, BARE_ENGINE, CLAUDE_ENGINE } from '../test/ipc';
import type { DraftAttachment } from './attachments';
import { conversationOf, dropConversation } from './conversations.svelte';
import { forgetDraft, getDraft } from './drafts';
import { cancelHeld, heldOf } from './outbox.svelte';
import { outgoing, routeOf, submitDraft } from './send.svelte';
import { app } from './state.svelte';
import type { Agent, ConvItem, InitialState, QuestionItem } from './types';

const question = (id: string, text: string): QuestionItem =>
  ({
    kind: 'question',
    id,
    toolUseId: `tu-${id}`,
    questions: [{ question: text, options: [] }],
    answers: null,
    ts: 1,
  }) as unknown as QuestionItem;

const file = (name: string): DraftAttachment => ({ kind: 'image', name, mediaType: 'image/png', data: 'AA==', size: 1 });

/** The app started on these agents; `items`: the conversation of each; `delay`: the delay of a message typed mid-turn. */
async function start(agents: Agent[], opts: { items?: ConvItem[]; delay?: number } & Record<string, unknown> = {}) {
  const { items = [], delay = 0, ...handlers } = opts;
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
    engines: [CLAUDE_ENGINE, BARE_ENGINE],
  };
  const backend = fakeBackend({
    subscribe: () => initial,
    get_conversation: () => items,
    ...(handlers as Record<string, (args: any) => unknown>),
  });
  await app.init();
  for (const a of agents) await conversationOf(a.id).load();
  return backend;
}

describe('submitDraft', () => {
  afterEach(() => {
    for (const h of heldOf('a1')) cancelHeld(h.id);
    dropConversation('a1');
    forgetDraft('a1');
  });

  it('answers the question waiting with the text typed', async () => {
    const backend = await start([agent({ id: 'a1', status: 'waiting', pending: ['q1'] })], { items: [question('q1', 'Q ?')] });
    const s = submitDraft(app.agents.a1, 'a1', 'Oui', [file('a.png')]);
    await s.done;
    expect(backend.called('answer_question')[0].args).toMatchObject({ id: 'a1', requestId: 'q1', answers: { 'Q ?': 'Oui' } });
    expect(backend.called('send_message')).toHaveLength(0);
    expect(s.cleared).toBe('text');
  });

  it('sends a message otherwise, files and all', async () => {
    const backend = await start([agent({ id: 'a1' })]);
    const s = submitDraft(app.agents.a1, 'a1', 'Bonjour', []);
    await s.done;
    expect(backend.called('send_message')[0].args).toMatchObject({ id: 'a1', text: 'Bonjour' });
    expect(s.cleared).toBe('all');
  });

  it('holds a message typed mid-turn when the delay is on', async () => {
    const backend = await start([agent({ id: 'a1', status: 'running' })], { delay: 10 });
    expect(routeOf(app.agents.a1, 'x', undefined)).toEqual({ kind: 'hold', mode: 'delay' });
    submitDraft(app.agents.a1, 'a1', 'x', []);
    expect(heldOf('a1')).toHaveLength(1);
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('gives the text back when the message is refused', async () => {
    await start([agent({ id: 'a1' })], {
      send_message: () => {
        throw 'refusé';
      },
    });
    await submitDraft(app.agents.a1, 'a1', 'Bonjour', []).done;
    expect(getDraft('a1').text).toBe('Bonjour');
  });

  it('sends nothing, and says so, while a message already waits for its worktree', async () => {
    const backend = await start([agent({ id: 'a1' })]);
    outgoing.a1 = 'worktree';
    try {
      expect(submitDraft(app.agents.a1, 'a1', 'encore', []).cleared).toBe('none');
      expect(app.toasts.map((x) => x.text)).toContain('Un message attend déjà la fin de la préparation.');
      expect(backend.called('send_message')).toHaveLength(0);
    } finally {
      delete outgoing.a1;
    }
  });

  it("sends a second message after the first, never beside it, without waiting for another agent's", async () => {
    let release!: () => void;
    const order: string[] = [];
    await start([agent({ id: 'a1' }), agent({ id: 'a2' })], {
      send_message: (args: { id: string; text: string }) => {
        order.push(`${args.id}:${args.text}`);
        return args.text === 'un' ? new Promise<void>((r) => (release = r)) : null;
      },
    });
    const first = submitDraft(app.agents.a1, 'a1', 'un', []);
    const second = submitDraft(app.agents.a1, 'a1', 'deux', []);
    const other = submitDraft(app.agents.a2, 'a2', 'autre', []);
    expect(second.cleared).toBe('all');
    await other.done;
    expect(order).toEqual(['a1:un', 'a2:autre']);
    release();
    await Promise.all([first.done, second.done]);
    expect(order).toEqual(['a1:un', 'a2:autre', 'a1:deux']);
    dropConversation('a2');
    forgetDraft('a2');
  });
});
