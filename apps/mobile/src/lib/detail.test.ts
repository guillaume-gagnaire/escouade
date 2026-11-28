// What an agent stopped for, whole (MO-5, M4), and what the phone keeps of a space (M13): never a command.

import { beforeEach, describe, expect, it } from 'vitest';
import type { ConvItem, PendingRequest, PermissionItem } from '$shared/lib/types';
import { glance } from '../test/fake-socket';
import { detailOf, inputText } from './detail';
import { draftOf, forgetKept, keepDraft, keepGlance, keptGlances, keptSeen, keepSeen } from './kept';

const request = (over: Partial<PendingRequest> = {}): PendingRequest => ({
  id: 'p1',
  kind: 'permission',
  tool: 'Bash',
  arg: 'docker compose up -d db',
  description: 'Démarre la base de test',
  reason: null,
  questions: [],
  cut: false,
  defaultNo: false,
  ...over,
});

const long = Array.from({ length: 30 }, (_, i) => `echo ${i}`).join('\n');
const items: ConvItem[] = [
  { id: 't1', kind: 'text', text: 'Les fixtures sont prêtes : je démarre la base.', streaming: false } as ConvItem,
  {
    id: 'p1',
    kind: 'permission',
    toolUseId: 'u1',
    toolName: 'Bash',
    input: { command: long },
    canAlways: false,
    defaultNo: false,
    decision: null,
    ts: 1,
  } as ConvItem,
];

describe('the request whole', () => {
  it('takes what the line carries when it was not cut', () => {
    expect(detailOf(request(), null)).toEqual({ text: 'docker compose up -d db', shell: true, whole: true, plan: null, context: null });
  });

  it('never says whole a request the line carries cut', () => {
    expect(detailOf(request({ arg: 'echo 0…', cut: true }), null).whole).toBe(false);
  });

  it('reads the command whole from its card in the conversation, and what the agent wrote before', () => {
    const d = detailOf(request({ arg: 'echo 0…', cut: true }), items);
    expect(d).toMatchObject({ text: long, shell: true, whole: true, context: 'Les fixtures sont prêtes : je démarre la base.' });
  });

  it('a plan: its text from the card', () => {
    const plan: ConvItem[] = [{ ...(items[1] as PermissionItem), toolName: 'ExitPlanMode', input: { plan: '# Plan\n\n1. Test' } }];
    expect(detailOf(request({ tool: 'ExitPlanMode', arg: '' }), plan).plan).toBe('# Plan\n\n1. Test');
  });

  it('shows what a tool other than a shell acts on', () => {
    expect(inputText({ toolName: 'Write', input: { file_path: 'src/a.ts', content: 'x' } })).toEqual({
      text: 'src/a.ts\n\nx',
      shell: false,
    });
    expect(inputText({ toolName: 'WebFetch', input: { url: 'https://x' } })).toEqual({ text: 'https://x', shell: false });
  });
});

describe('what the phone keeps (M13)', () => {
  beforeEach(() => localStorage.clear());

  it('keeps the last glance of a space without what an agent asks', () => {
    const g = glance({
      waiting: 1,
      agents: [
        {
          id: 'a',
          projectId: 'p',
          name: 'tests-e2e',
          status: 'waiting',
          activity: 'Lance npm test',
          lastActivity: 5,
          request: request({ arg: 'curl -H "Authorization: x" …' }),
        },
      ],
      asks: [{ id: 'k', projectId: 'p', title: 'Choisis la direction', createdAt: 1 }],
    });
    keepGlance('travail', g as never, 42);
    const kept = keptGlances().travail;
    expect(kept.at).toBe(42);
    expect(kept.glance.waiting).toBe(1);
    expect(kept.glance.asks[0].title).toBe('Choisis la direction');
    expect(kept.glance.agents[0]).toMatchObject({
      name: 'tests-e2e',
      activity: null,
      request: { kind: 'permission', tool: 'Bash', arg: '' },
    });
    expect(JSON.stringify(localStorage)).not.toContain('curl');
    expect(JSON.stringify(localStorage)).not.toContain('Démarre');
  });

  it('keeps the drafts until they go, and forgets everything of a space removed', () => {
    keepDraft('travail:talk:a', 'et relance seulement les avoirs');
    expect(draftOf('travail:talk:a')).toBe('et relance seulement les avoirs');
    keepGlance('travail', glance() as never, 1);
    forgetKept('travail');
    expect(draftOf('travail:talk:a')).toBe('');
    expect(keptGlances().travail).toBeUndefined();
    keepDraft('x', 'y');
    keepDraft('x', '  ');
    expect(draftOf('x')).toBe('');
  });

  it('keeps the lines seen, the newest 400', () => {
    keepSeen(Object.fromEntries(Array.from({ length: 450 }, (_, i) => [`k${i}`, 'm'])));
    const seen = keptSeen();
    expect(Object.keys(seen)).toHaveLength(400);
    expect(seen.k449).toBe('m');
    expect(seen.k0).toBeUndefined();
  });
});
