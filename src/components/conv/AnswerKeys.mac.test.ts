import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The cards on macOS: Cmd instead of Ctrl, written the Mac way.
vi.mock('../../lib/platform', async (original) => {
  const real = await original<typeof import('../../lib/platform')>();
  return {
    ...real,
    IS_MAC: true,
    primaryKey: (e: KeyboardEvent, mac = true) => real.primaryKey(e, mac),
    keyLabel: (shortcut: string, mac = true) => real.keyLabel(shortcut, mac),
  };
});

import type { PermissionItem, QuestionItem } from '../../lib/types';
import { answerClock, conversationHost, settle } from '../../test/conversation';
import { fakeBackend, resetApp } from '../../test/ipc';
import PermissionCard from './PermissionCard.svelte';
import QuestionCard from './QuestionCard.svelte';

const permission: PermissionItem = {
  kind: 'permission',
  id: 'req-2',
  toolUseId: 't2',
  toolName: 'Bash',
  input: { command: 'rm -rf build' },
  canAlways: true,
  defaultNo: false,
  decision: null,
  ts: 1,
};
const question: QuestionItem = {
  kind: 'question',
  id: 'req-1',
  toolUseId: 't1',
  ts: 1,
  answers: null,
  questions: [
    { question: 'Base ?', multiSelect: false, options: [{ label: 'PG' }, { label: 'SQLite' }] },
    { question: 'Cible ?', multiSelect: false, options: [{ label: 'Web' }, { label: 'Desktop' }] },
  ],
};

describe('answering from the keyboard on macOS', () => {
  beforeEach(() => resetApp());
  answerClock();

  it('allows with Cmd+Enter and "always" with Cmd+Shift+Enter, not with Ctrl', async () => {
    const backend = fakeBackend();
    render(PermissionCard, { target: conversationHost(), props: { item: permission, agentId: 'a1', pending: true, cwd: 'C:\\code' } });
    settle();
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    expect(backend.called('answer_permission')).toHaveLength(0);
    await userEvent.keyboard('{Meta>}{Shift>}{Enter}{/Shift}{/Meta}');
    expect(backend.called('answer_permission')[0].args.decision).toBe('always');
  });

  it('writes the keys with ⌘ on the buttons', () => {
    fakeBackend();
    render(PermissionCard, { target: conversationHost(), props: { item: permission, agentId: 'a1', pending: true, cwd: 'C:\\code' } });
    const allow = screen.getByRole('button', { name: 'Autoriser' });
    expect(allow).toHaveTextContent('⌘Entrée');
    expect(allow).toHaveAttribute('aria-keyshortcuts', 'Meta+Enter');
    expect(screen.getByRole('button', { name: 'Toujours autoriser' })).toHaveTextContent('⇧⌘Entrée');
  });

  it('validates a question with Cmd+Enter once it is answered, and picks its options with Alt+digit', async () => {
    const backend = fakeBackend();
    render(QuestionCard, { target: conversationHost(), props: { item: question, agentId: 'a1', pending: true } });
    settle();
    await userEvent.keyboard('{Alt>}1{/Alt}{Alt>}2{/Alt}');
    expect(screen.getByRole('button', { name: 'Valider' })).toHaveTextContent('⌘Entrée');
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    expect(backend.called('answer_question')).toHaveLength(0);
    await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Base ?': 'PG', 'Cible ?': 'Desktop' });
  });
});
