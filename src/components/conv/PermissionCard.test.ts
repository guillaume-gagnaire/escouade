import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { PermissionItem } from '../../lib/types';
import { answerClock, conversationHost, settle } from '../../test/conversation';
import { fakeBackend, resetApp } from '../../test/ipc';
import PermissionCard from './PermissionCard.svelte';

const item = (over: Partial<PermissionItem> = {}): PermissionItem => ({
  kind: 'permission',
  id: 'req-2',
  toolUseId: 't2',
  toolName: 'Bash',
  input: { command: 'rm -rf build' },
  reason: 'Commande destructive',
  canAlways: true,
  defaultNo: false,
  decision: null,
  ts: 1,
  ...over,
});

describe('PermissionCard', () => {
  beforeEach(() => resetApp());

  it('shows the command and the reason, and allows it', async () => {
    const backend = fakeBackend();
    render(PermissionCard, { item: item(), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.getByText('rm -rf build')).toBeInTheDocument();
    expect(screen.getByText('Commande destructive')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Autoriser' }));
    expect(backend.called('answer_permission')[0].args).toEqual({ id: 'a1', requestId: 'req-2', decision: 'allow', message: null });
  });

  it('offers "always" only when Claude suggests a rule', async () => {
    const backend = fakeBackend();
    const { unmount } = render(PermissionCard, { item: item(), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    await userEvent.click(screen.getByRole('button', { name: 'Toujours autoriser' }));
    expect(backend.called('answer_permission')[0].args.decision).toBe('always');
    unmount();
    render(PermissionCard, { item: item({ canAlways: false }), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.queryByRole('button', { name: 'Toujours autoriser' })).not.toBeInTheDocument();
  });

  it('renders a proposed plan and keeps planning on refusal', async () => {
    const backend = fakeBackend();
    render(PermissionCard, {
      item: item({ toolName: 'ExitPlanMode', input: { plan: '## Plan\n\n1. Écrire les tests' } }),
      agentId: 'a1',
      pending: true,
      cwd: 'C:\\code',
    });
    expect(screen.getByRole('heading', { name: 'Plan' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Continuer à planifier' }));
    const args = backend.called('answer_permission')[0].args;
    expect(args.decision).toBe('deny');
    expect(args.message).toMatch(/planifier/);
  });

  it('summarizes the decision once answered', () => {
    render(PermissionCard, {
      item: item({ decision: 'deny', message: 'utilise npm run clean' }),
      agentId: 'a1',
      pending: false,
      cwd: 'C:\\code',
    });
    expect(screen.getByText(/Refusé/)).toBeInTheDocument();
    expect(screen.getByText(/utilise npm run clean/)).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('makes the refusal its main button when Claude Code would refuse it by default', () => {
    fakeBackend();
    const { unmount } = render(PermissionCard, { item: item(), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.getByRole('button', { name: 'Autoriser' })).toHaveClass('primary');
    expect(screen.getByRole('button', { name: 'Refuser' })).not.toHaveClass('primary');
    unmount();
    const { unmount: gone } = render(PermissionCard, { item: item({ defaultNo: true }), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.getByRole('button', { name: 'Refuser' })).toHaveClass('primary');
    expect(screen.getByRole('button', { name: 'Autoriser' })).not.toHaveClass('primary');
    gone();
    render(PermissionCard, {
      item: item({ defaultNo: true, toolName: 'ExitPlanMode', input: { plan: '## Plan' } }),
      agentId: 'a1',
      pending: true,
      cwd: 'C:\\code',
    });
    expect(screen.getByRole('button', { name: 'Continuer à planifier' })).toHaveClass('primary');
    expect(screen.getByRole('button', { name: 'Approuver le plan' })).not.toHaveClass('primary');
  });
});

describe('PermissionCard of a tool of Escouade', () => {
  beforeEach(() => resetApp());

  /** Claude Code asks for a tool of Escouade's own server. */
  const escouade = (input: Record<string, unknown>, over: Partial<PermissionItem> = {}) =>
    item({
      toolName: 'mcp__escouade__create_ticket',
      title: 'Claude wants to use escouade - create_ticket',
      input,
      reason: undefined,
      ...over,
    });
  const terms = () => screen.getAllByRole('term').map((t) => t.textContent);
  const values = () => screen.getAllByRole('definition').map((d) => d.textContent);

  it('names the tool in words, then gives each argument in clear, one per line, its lines kept', async () => {
    const backend = fakeBackend();
    render(PermissionCard, {
      item: escouade({
        project: 'demo',
        title: 'Corriger la connexion',
        description: 'Le jeton expire.\nIl faut le renouveler.',
        criteria: ['Le test passe', 'La doc suit'],
      }),
      agentId: 'a1',
      pending: true,
      cwd: 'C:\\code',
    });
    expect(screen.getByText('Escouade · Créer un ticket')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Ce que Claude donne à l’outil' })).toBeInTheDocument();
    expect(terms()).toEqual(['project :', 'title :', 'description :', 'criteria :']);
    expect(values()).toEqual(['demo', 'Corriger la connexion', 'Le jeton expire.\nIl faut le renouveler.', 'Le test passe\nLa doc suit']);
    // Said once: Claude Code's title for the request says nothing the badge does not.
    expect(screen.queryByText(/Claude wants to use/)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Autoriser' }));
    expect(backend.called('answer_permission')[0].args).toEqual({ id: 'a1', requestId: 'req-2', decision: 'allow', message: null });
  });

  it('cuts a long value at 2,000 characters', () => {
    fakeBackend();
    render(PermissionCard, { item: escouade({ description: 'a'.repeat(2400) }), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    const [value] = values();
    expect(value).toBe(`${'a'.repeat(2000)}…`);
  });

  it('says how many characters a cut value hides, and to read the whole request in the conversation', () => {
    fakeBackend();
    const { unmount } = render(PermissionCard, {
      item: escouade({ title: 'court', description: 'a'.repeat(2400) }),
      agentId: 'a1',
      pending: true,
      cwd: 'C:\\code',
    });
    expect(
      screen.getByText('400 caractères de plus ne sont pas montrés : lis la demande entière dans la conversation.'),
    ).toBeInTheDocument();
    // Only the value that was cut says it.
    expect(screen.getAllByText(/de plus/)).toHaveLength(1);
    unmount();
    render(PermissionCard, { item: escouade({ description: 'a'.repeat(2001) }), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.getByText('1 caractère de plus n’est pas montré : lis la demande entière dans la conversation.')).toBeInTheDocument();
  });

  it('sums up its decision with the tool in words and what it acted on', () => {
    render(PermissionCard, {
      item: escouade({ project: 'demo', title: 'Corriger la connexion' }, { decision: 'allow' }),
      agentId: 'a1',
      pending: false,
      cwd: 'C:\\code',
    });
    expect(screen.getByText(/Escouade · Créer un ticket/)).toHaveTextContent('Corriger la connexion');
  });

  it('reads in English', () => {
    setLang('en');
    fakeBackend();
    render(PermissionCard, {
      item: escouade({ project: 'demo', message: 'Write the docs' }, { toolName: 'mcp__escouade__create_agent' }),
      agentId: 'a1',
      pending: true,
      cwd: 'C:\\code',
    });
    expect(screen.getByText('Escouade · Start an agent')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'What Claude gives the tool' })).toBeInTheDocument();
    expect(terms()).toEqual(['project:', 'message:']);
  });

  it('says what a cut value hides in English, its count grouped', () => {
    setLang('en');
    fakeBackend();
    render(PermissionCard, { item: escouade({ description: 'a'.repeat(3500) }), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.getByText('1,500 more characters are not shown: read the whole request in the conversation.')).toBeInTheDocument();
  });
});

describe('PermissionCard keyboard', () => {
  beforeEach(() => resetApp());
  // What a test added around the card (a terminal, a sidebar) must not keep the focus for the next one.
  afterEach(() => document.querySelectorAll('.xterm, nav').forEach((n) => n.remove()));
  answerClock();

  const ctrl = (keys: string) => userEvent.keyboard(`{Control>}${keys}{/Control}`);
  /** The card, just drawn: its keys still wait. */
  const draw = (over: Partial<PermissionItem> = {}, props: { pending?: boolean; current?: boolean } = {}) =>
    render(PermissionCard, {
      target: conversationHost(),
      props: { item: item(over), agentId: 'a1', pending: true, cwd: 'C:\\code', ...props },
    });
  /** The card, drawn a moment ago: its keys answer. */
  const show = (over: Partial<PermissionItem> = {}, props: { pending?: boolean; current?: boolean } = {}) => {
    const shown = draw(over, props);
    settle();
    return shown;
  };

  it('answers nothing from the keys just after it comes: a Ctrl+Enter meant to send the message does not allow it unread', async () => {
    const backend = fakeBackend();
    draw();
    vi.advanceTimersByTime(100);
    const keydown = new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(keydown);
    await ctrl('{Shift>}{Enter}{/Shift}');
    expect(backend.called('answer_permission')).toHaveLength(0);
    // Not given to the message field either, which would send the text typed there as a refusal.
    expect(keydown.defaultPrevented).toBe(true);
    vi.advanceTimersByTime(500);
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')).toEqual([
      { cmd: 'answer_permission', args: { id: 'a1', requestId: 'req-2', decision: 'allow', message: null } },
    ]);
  });

  it('waits again once it becomes the request the keys answer, the one before it answered', async () => {
    const backend = fakeBackend();
    const { rerender } = draw({}, { current: false });
    settle();
    await rerender({ current: true });
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')).toHaveLength(0);
    settle();
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'req-2', decision: 'allow' });
  });

  it('allows with Ctrl+Enter', async () => {
    const backend = fakeBackend();
    show();
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')).toEqual([
      { cmd: 'answer_permission', args: { id: 'a1', requestId: 'req-2', decision: 'allow', message: null } },
    ]);
  });

  it('always allows with Ctrl+Shift+Enter, when Claude suggests a rule', async () => {
    const backend = fakeBackend();
    const { unmount } = show();
    await ctrl('{Shift>}{Enter}{/Shift}');
    expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'req-2', decision: 'always' });
    unmount();
    show({ canAlways: false });
    await ctrl('{Shift>}{Enter}{/Shift}');
    expect(backend.called('answer_permission')).toHaveLength(1);
  });

  it('allows a request Claude Code would refuse by default with its button only, not with the keys', async () => {
    const backend = fakeBackend();
    show({ defaultNo: true });
    await ctrl('{Enter}');
    await ctrl('{Shift>}{Enter}{/Shift}');
    expect(backend.called('answer_permission')).toHaveLength(0);
    expect(screen.queryByText('Ctrl+Entrée')).not.toBeInTheDocument();
    expect(screen.queryByText('Ctrl+Maj+Entrée')).not.toBeInTheDocument();
    const allow = screen.getByRole('button', { name: 'Autoriser' });
    expect(allow).not.toHaveAttribute('aria-keyshortcuts');
    await userEvent.click(allow);
    expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'req-2', decision: 'allow' });
  });

  it('holds Enter in the message field a moment once it comes, as the keys that answer, a request refused by default too', async () => {
    fakeBackend();
    const host = conversationHost();
    host.insertAdjacentHTML('beforeend', '<div class="composer"><textarea aria-label="message"></textarea></div>');
    render(PermissionCard, { target: host, props: { item: item({ defaultNo: true }), agentId: 'a1', pending: true, cwd: 'C:\\code' } });
    const enter = (over: KeyboardEventInit = {}) => {
      const keydown = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true, ...over });
      screen.getByLabelText('message').dispatchEvent(keydown);
      return keydown.defaultPrevented;
    };
    expect(enter()).toBe(true);
    // A new line, and an input method confirming its text, go in.
    expect(enter({ shiftKey: true })).toBe(false);
    expect(enter({ isComposing: true })).toBe(false);
    settle();
    expect(enter()).toBe(false);
  });

  it('approves a plan the same way', async () => {
    const backend = fakeBackend();
    show({ toolName: 'ExitPlanMode', input: { plan: '## Plan' } });
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')[0].args).toMatchObject({ decision: 'allow', message: null });
  });

  it('does nothing once answered, nor for a request that is not the first to wait', async () => {
    const backend = fakeBackend();
    const { unmount } = show({ decision: 'allow' }, { pending: false });
    await ctrl('{Enter}');
    unmount();
    show({}, { current: false });
    await ctrl('{Enter}');
    await ctrl('{Shift>}{Enter}{/Shift}');
    expect(backend.called('answer_permission')).toHaveLength(0);
  });

  it('does nothing from a terminal or from outside the conversation', async () => {
    const backend = fakeBackend();
    show();
    document.body.insertAdjacentHTML(
      'beforeend',
      '<div class="xterm"><textarea aria-label="terminal"></textarea></div><nav><button>Nouvel agent</button></nav>',
    );
    screen.getByLabelText('terminal').focus();
    await ctrl('{Enter}');
    screen.getByRole('button', { name: 'Nouvel agent' }).focus();
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')).toHaveLength(0);
  });

  it('does nothing behind a dialog', async () => {
    const backend = fakeBackend();
    show();
    app.modal = { kind: 'settings' };
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')).toHaveLength(0);
  });

  it('ignores AltGr (Ctrl+Alt) and Enter alone', async () => {
    const backend = fakeBackend();
    show();
    await userEvent.keyboard('{Control>}{Alt>}{Enter}{/Alt}{/Control}');
    await userEvent.keyboard('{Enter}');
    await userEvent.keyboard('{Shift>}{Enter}{/Shift}');
    expect(backend.called('answer_permission')).toHaveLength(0);
  });

  it('answers once while the answer is on its way', async () => {
    const backend = fakeBackend({ answer_permission: () => new Promise(() => {}) });
    show();
    await ctrl('{Enter}');
    await ctrl('{Enter}');
    expect(backend.called('answer_permission')).toHaveLength(1);
  });

  it('shows the keys on the buttons without changing their names', () => {
    show();
    const allow = screen.getByRole('button', { name: 'Autoriser' });
    expect(allow).toHaveTextContent('Ctrl+Entrée');
    expect(allow).toHaveAttribute('aria-keyshortcuts', 'Control+Enter');
    const always = screen.getByRole('button', { name: 'Toujours autoriser' });
    expect(always).toHaveTextContent('Ctrl+Maj+Entrée');
    expect(always).toHaveAttribute('aria-keyshortcuts', 'Control+Shift+Enter');
    expect(screen.getByRole('button', { name: 'Refuser' })).not.toHaveAttribute('aria-keyshortcuts');
  });

  it('shows no key for "always" when Claude suggests no rule, nor on a request that is not the first', () => {
    const { unmount } = show({ canAlways: false });
    expect(screen.queryByText('Ctrl+Maj+Entrée')).not.toBeInTheDocument();
    expect(screen.getByText('Ctrl+Entrée')).toBeInTheDocument();
    unmount();
    show({}, { current: false });
    expect(screen.queryByText('Ctrl+Entrée')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Autoriser' })).not.toHaveAttribute('aria-keyshortcuts');
  });

  it('gives the focus back to the message field once answered while it was on the card', async () => {
    fakeBackend();
    show();
    screen.getByRole('button', { name: 'Refuser' }).focus();
    const focus = app.focusComposer;
    await ctrl('{Enter}');
    expect(app.focusComposer).toBe(focus + 1);
  });

  it('leaves the focus where it is when it was not on the card', async () => {
    const backend = fakeBackend();
    show();
    const focus = app.focusComposer;
    await ctrl('{Enter}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    await new Promise((r) => setTimeout(r)); // lets the answer settle
    expect(app.focusComposer).toBe(focus);
  });
});

describe('PermissionCard in English', () => {
  beforeEach(() => resetApp());

  it('asks in English, with the keys written as in English', () => {
    setLang('en');
    render(PermissionCard, { item: item(), agentId: 'a1', pending: true, cwd: 'C:\\code' });
    expect(screen.getByRole('group', { name: 'Claude is asking for permission' })).toBeInTheDocument();
    const allow = screen.getByRole('button', { name: 'Allow' });
    expect(allow).toHaveTextContent('Ctrl+Enter');
    expect(screen.getByRole('button', { name: 'Always allow' })).toHaveTextContent('Ctrl+Shift+Enter');
    expect(screen.getByRole('button', { name: 'Deny' })).toBeInTheDocument();
    expect(screen.getByText('To deny and explain what to do instead, write it in the field below.')).toBeInTheDocument();
  });

  it('proposes a plan in English, and tells Claude to keep planning in English when it is refused', async () => {
    setLang('en');
    const backend = fakeBackend();
    render(PermissionCard, {
      item: item({ toolName: 'ExitPlanMode', input: { plan: '## Plan\n\n1. Write the tests' } }),
      agentId: 'a1',
      pending: true,
      cwd: 'C:\\code',
    });
    expect(screen.getByRole('group', { name: 'Claude proposes a plan' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Approve the plan' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Approve and accept edits' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Keep planning' }));
    expect(backend.called('answer_permission')[0].args.message).toBe('Keep planning: the plan doesn’t work for me yet.');
  });

  it('summarizes the decision in English once answered', () => {
    setLang('en');
    const answered = (over: Partial<PermissionItem>) => ({ item: item(over), agentId: 'a1', pending: false, cwd: 'C:\\code' });
    const { unmount } = render(PermissionCard, answered({ decision: 'deny', message: 'use npm run clean' }));
    expect(screen.getByText(/✕ Denied/)).toBeInTheDocument();
    unmount();
    const { unmount: next } = render(PermissionCard, answered({ decision: 'always' }));
    expect(screen.getByText(/✓ Always allowed/)).toBeInTheDocument();
    next();
    render(PermissionCard, answered({ decision: null }));
    expect(screen.getByText(/Request canceled/)).toBeInTheDocument();
  });
});
