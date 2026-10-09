import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { PermissionItem } from '../../lib/types';
import { conversationHost } from '../../test/conversation';
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
});

describe('PermissionCard keyboard', () => {
  beforeEach(() => {
    resetApp();
    app.focusPending = null;
  });
  // What a test added around the card (a terminal, a sidebar) must not keep the focus for the next one.
  afterEach(() => document.querySelectorAll('.xterm, nav').forEach((n) => n.remove()));

  const ctrl = (keys: string) => userEvent.keyboard(`{Control>}${keys}{/Control}`);
  const show = (over: Partial<PermissionItem> = {}, props: { pending?: boolean; current?: boolean } = {}) =>
    render(PermissionCard, {
      target: conversationHost(),
      props: { item: item(over), agentId: 'a1', pending: true, cwd: 'C:\\code', ...props },
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

  it('gives the focus back to the message field once answered with the keyboard from the card', async () => {
    fakeBackend();
    show();
    screen.getByRole('group', { name: 'Claude demande une autorisation' }).focus();
    const focus = app.focusComposer;
    await ctrl('{Enter}');
    expect(app.focusComposer).toBe(focus + 1);
  });

  it('takes the focus when Ctrl+J brings its agent in', async () => {
    fakeBackend();
    app.focusPending = 'a1';
    show();
    const card = screen.getByRole('group', { name: 'Claude demande une autorisation' });
    await waitFor(() => expect(card).toHaveFocus());
    expect(app.focusPending).toBeNull();
  });

  it('leaves the focus alone when Ctrl+J is for another agent or another request', async () => {
    fakeBackend();
    app.focusPending = 'a2';
    const { unmount } = show();
    await tick();
    expect(screen.getByRole('group', { name: 'Claude demande une autorisation' })).not.toHaveFocus();
    expect(app.focusPending).toBe('a2');
    unmount();
    app.focusPending = 'a1';
    show({}, { current: false });
    await tick();
    expect(screen.getByRole('group', { name: 'Claude demande une autorisation' })).not.toHaveFocus();
  });
});
