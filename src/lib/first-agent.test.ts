import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { firstTurn, starters } from './first-agent';
import type { ConvItem } from './types';

const user = (text = 'Explique-moi ce dépôt en cinq lignes.'): ConvItem => ({
  kind: 'user',
  id: 'u1',
  text,
  images: 0,
  ts: 1,
  queued: false,
});
const turn = (over: object = {}): ConvItem =>
  ({
    kind: 'turn',
    id: 't1',
    ts: 2,
    durationMs: 1000,
    cost: 0,
    tokens: 10,
    isError: false,
    interrupted: false,
    error: null,
    ...over,
  }) as ConvItem;
const permission: ConvItem = {
  kind: 'permission',
  id: 'p1',
  toolUseId: 'tu1',
  toolName: 'Bash',
  input: { command: 'npm test' },
} as unknown as ConvItem;

describe('the first agent’s turn (ESC-505)', () => {
  it('waits for the user’s message: nothing is sent by itself', () => {
    expect(firstTurn(agent({ status: 'idle', prompts: 0 }), []).state).toBe('compose');
  });

  it('says a message sent that waits for the agent’s worktree is gone, not still to send', () => {
    const waiting = [{ text: 'Lance les tests et dis-moi ce qui casse.' }];
    expect(firstTurn(agent({ status: 'idle', prompts: 0 }), [], waiting)).toMatchObject({
      state: 'preparing',
      sent: 'Lance les tests et dis-moi ce qui casse.',
    });
    expect(firstTurn(agent({ status: 'idle', prompts: 0, setup: '1/2 · npm ci' }), []).state).toBe('preparing');
  });

  it('follows the turn: live, a permission waiting, then done', () => {
    const sent = [user()];
    expect(firstTurn(agent({ status: 'running', prompts: 1 }), sent).state).toBe('running');
    const waiting = firstTurn(agent({ status: 'waiting', prompts: 1, pending: ['p1'] }), [...sent, permission]);
    expect(waiting.state).toBe('permission');
    const done = firstTurn(agent({ status: 'idle', prompts: 1 }), [...sent, permission, turn()]);
    expect(done).toEqual({ state: 'done', error: null, until: null, sent: 'Explique-moi ce dépôt en cinq lignes.' });
  });

  it('says why a turn stopped short: the usage limit, offline, not signed in, anything else', () => {
    const sent = [user()];
    const limited = firstTurn(agent({ status: 'idle', prompts: 1, resumeAt: 1_800_000_000_000 }), [
      ...sent,
      turn({ isError: true, limited: true }),
    ]);
    expect(limited).toMatchObject({ state: 'limit', until: 1_800_000_000_000 });
    expect(firstTurn(agent({ status: 'idle', prompts: 1 }), [...sent, turn({ limited: true })]).state).toBe('limit');
    const offline = firstTurn(agent({ status: 'error', prompts: 1 }), [
      ...sent,
      turn({ isError: true, error: 'API Error: Connection error.' }),
    ]);
    expect(offline).toMatchObject({ state: 'offline', sent: 'Explique-moi ce dépôt en cinq lignes.' });
    expect(
      firstTurn(agent({ status: 'error', prompts: 1 }), [
        ...sent,
        turn({ isError: true, error: 'getaddrinfo ENOTFOUND api.anthropic.com' }),
      ]).state,
    ).toBe('offline');
    expect(
      firstTurn(agent({ status: 'error', prompts: 1 }), [...sent, turn({ isError: true, error: 'Invalid API key · Please run /login' })])
        .state,
    ).toBe('signedOut');
    expect(firstTurn(agent({ status: 'idle', prompts: 1 }), [...sent, turn({ isError: true, signIn: true })]).state).toBe('signedOut');
    expect(
      firstTurn(agent({ status: 'error', prompts: 1 }), [...sent, turn({ isError: true, error: 'Le modèle a planté' })]),
    ).toMatchObject({
      state: 'failed',
      error: 'Le modèle a planté',
    });
  });
});

describe('the messages proposed (ESC-505)', () => {
  it('proposes the tests only when the project has some that can run, the demonstration starting with them', () => {
    expect(starters(false)).toEqual(['explain', 'bug']);
    expect(starters(true)).toEqual(['explain', 'tests', 'bug']);
    expect(starters(true, true)).toEqual(['tests', 'explain', 'bug']);
    expect(starters(false, true)).toEqual(['explain', 'bug']);
  });
});
