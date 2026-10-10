import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { QuestionItem } from '../../lib/types';
import { answerClock, conversationHost, settle } from '../../test/conversation';
import { fakeBackend, resetApp } from '../../test/ipc';
import QuestionCard from './QuestionCard.svelte';

const item = (over: Partial<QuestionItem> = {}): QuestionItem => ({
  kind: 'question',
  id: 'req-1',
  toolUseId: 't1',
  ts: 1,
  answers: null,
  questions: [
    {
      question: 'Quelle base de données ?',
      header: 'Base',
      multiSelect: false,
      options: [
        { label: 'PostgreSQL', description: 'Relationnelle' },
        { label: 'SQLite', description: 'Embarquée' },
      ],
    },
  ],
  ...over,
});

describe('QuestionCard', () => {
  beforeEach(() => resetApp());

  it('answers a single question with one click', async () => {
    const backend = fakeBackend();
    render(QuestionCard, { item: item(), agentId: 'a1', pending: true });
    expect(screen.getByText('Claude attend ta réponse')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'SQLite' }));
    expect(backend.called('answer_question')).toEqual([
      { cmd: 'answer_question', args: { id: 'a1', requestId: 'req-1', answers: { 'Quelle base de données ?': 'SQLite' } } },
    ]);
  });

  it('collects several answers before validating, multi-select joined with commas', async () => {
    const backend = fakeBackend();
    const multi = item({
      questions: [
        { question: 'Base ?', header: 'Base', options: [{ label: 'PG' }, { label: 'SQLite' }] },
        {
          question: 'Outils ?',
          header: 'Outils',
          multiSelect: true,
          options: [{ label: 'ESLint' }, { label: 'Prettier' }, { label: 'Vitest' }],
        },
      ],
    });
    render(QuestionCard, { item: multi, agentId: 'a1', pending: true });
    const validate = screen.getByRole('button', { name: 'Valider' });
    expect(validate).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'PG' }));
    await userEvent.click(screen.getByRole('button', { name: /ESLint/ }));
    await userEvent.click(screen.getByRole('button', { name: /Vitest/ }));
    await userEvent.click(screen.getByRole('button', { name: /ESLint/ })); // toggled off again
    await userEvent.click(screen.getByRole('button', { name: /Prettier/ }));
    expect(backend.called('answer_question')).toHaveLength(0);
    await userEvent.click(validate);
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Base ?': 'PG', 'Outils ?': 'Vitest, Prettier' });
  });

  it('shows the recorded answer once answered', () => {
    render(QuestionCard, { item: item({ answers: { 'Quelle base de données ?': 'PostgreSQL' } }), agentId: 'a1', pending: false });
    expect(screen.getByText('→ PostgreSQL')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'SQLite' })).not.toBeInTheDocument();
  });

  it('says when the question was never answered', () => {
    render(QuestionCard, { item: item(), agentId: 'a1', pending: false });
    expect(screen.getByText('Question restée sans réponse')).toBeInTheDocument();
  });
});

describe('QuestionCard edge cases', () => {
  it('renders options that share a label', () => {
    fakeBackend();
    const dup = item({ questions: [{ question: 'Q ?', options: [{ label: 'Oui' }, { label: 'Oui' }] }] });
    render(QuestionCard, { item: dup, agentId: 'a1', pending: true });
    expect(screen.getAllByRole('button', { name: 'Oui' })).toHaveLength(2);
  });
});

describe('QuestionCard keyboard', () => {
  beforeEach(() => resetApp());
  // What a test added around the card (a terminal) must not keep the focus for the next one.
  afterEach(() => document.querySelectorAll('.xterm').forEach((n) => n.remove()));
  answerClock();

  const alt = (digit: string) => userEvent.keyboard(`{Alt>}${digit}{/Alt}`);
  const ctrlEnter = () => userEvent.keyboard('{Control>}{Enter}{/Control}');
  /** The card, just drawn: its keys still wait. */
  const draw = (over: Partial<QuestionItem> = {}, props: { pending?: boolean; current?: boolean } = {}) =>
    render(QuestionCard, {
      target: conversationHost(),
      props: { item: item(over), agentId: 'a1', pending: true, ...props },
    });
  /** The card, drawn a moment ago: its keys answer. */
  const show = (over: Partial<QuestionItem> = {}, props: { pending?: boolean; current?: boolean } = {}) => {
    const shown = draw(over, props);
    settle();
    return shown;
  };

  const [base, tools, target] = [
    { question: 'Base ?', header: 'Base', options: [{ label: 'PG' }, { label: 'SQLite' }] },
    {
      question: 'Outils ?',
      header: 'Outils',
      multiSelect: true,
      options: [{ label: 'ESLint' }, { label: 'Prettier' }, { label: 'Vitest' }],
    },
    { question: 'Cible ?', header: 'Cible', options: [{ label: 'Web' }, { label: 'Desktop' }] },
  ];

  it('picks option n of a single question with Alt+n, which answers it', async () => {
    const backend = fakeBackend();
    show();
    await alt('2');
    expect(backend.called('answer_question')).toEqual([
      { cmd: 'answer_question', args: { id: 'a1', requestId: 'req-1', answers: { 'Quelle base de données ?': 'SQLite' } } },
    ]);
  });

  it('answers nothing from the keys just after it comes, the message field included', async () => {
    const backend = fakeBackend();
    draw();
    vi.advanceTimersByTime(100);
    await alt('2');
    const keydown = new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(keydown);
    await tick();
    expect(keydown.defaultPrevented).toBe(true);
    expect(backend.called('answer_question')).toHaveLength(0);
    vi.advanceTimersByTime(500);
    await alt('2');
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Quelle base de données ?': 'SQLite' });
  });

  it('reads the digit from the physical key, as on an AZERTY keyboard', async () => {
    const backend = fakeBackend();
    show();
    const keydown = new KeyboardEvent('keydown', { key: '&', code: 'Digit1', altKey: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(keydown);
    await waitFor(() => expect(backend.called('answer_question')).toHaveLength(1));
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Quelle base de données ?': 'PostgreSQL' });
    expect(keydown.defaultPrevented).toBe(true);
  });

  it('ignores a digit without an option, and leaves the key alone', async () => {
    const backend = fakeBackend();
    show();
    const keydown = new KeyboardEvent('keydown', { key: '3', code: 'Digit3', altKey: true, bubbles: true, cancelable: true });
    document.body.dispatchEvent(keydown);
    await tick();
    expect(keydown.defaultPrevented).toBe(false);
    expect(backend.called('answer_question')).toHaveLength(0);
  });

  it('ticks and unticks the options of a multiple-choice question, validated with Ctrl+Enter once answered', async () => {
    const backend = fakeBackend();
    show({ questions: [tools] });
    await ctrlEnter(); // nothing ticked yet
    await alt('1');
    await alt('3');
    await alt('1'); // unticked again
    await alt('2');
    expect(screen.getByRole('button', { name: /Vitest/ })).toHaveTextContent('☑');
    expect(screen.getByRole('button', { name: /ESLint/ })).toHaveTextContent('☐');
    expect(backend.called('answer_question')).toHaveLength(0);
    await ctrlEnter();
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Outils ?': 'Vitest, Prettier' });
  });

  it('goes through several questions: a single choice moves on to the next unanswered one', async () => {
    const backend = fakeBackend();
    show({ questions: [base, target] });
    await alt('1'); // Base: PG
    await ctrlEnter(); // Cible is not answered yet
    expect(backend.called('answer_question')).toHaveLength(0);
    await alt('2'); // Cible: Desktop
    await ctrlEnter();
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Base ?': 'PG', 'Cible ?': 'Desktop' });
  });

  it('stays on a multiple-choice question until Ctrl+Enter moves on from it', async () => {
    const backend = fakeBackend();
    show({ questions: [tools, target] });
    await alt('1');
    await alt('2'); // still Outils
    await ctrlEnter(); // answered but not complete: on to Cible
    expect(backend.called('answer_question')).toHaveLength(0);
    await alt('1'); // Cible: Web
    await ctrlEnter();
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Outils ?': 'ESLint, Prettier', 'Cible ?': 'Web' });
  });

  it('follows a click: the keys go to the first question still unanswered', async () => {
    const backend = fakeBackend();
    show({ questions: [base, target] });
    await userEvent.click(screen.getByRole('button', { name: 'Desktop' }));
    await alt('2'); // back on Base: SQLite
    await ctrlEnter();
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Base ?': 'SQLite', 'Cible ?': 'Desktop' });
  });

  it('does nothing once answered, nor for a question that is not the first to wait', async () => {
    const backend = fakeBackend();
    const { unmount } = show({ answers: { 'Quelle base de données ?': 'PostgreSQL' } }, { pending: false });
    await alt('1');
    await ctrlEnter();
    unmount();
    show({}, { current: false });
    await alt('1');
    expect(backend.called('answer_question')).toHaveLength(0);
  });

  it('does nothing from a terminal, behind a dialog, or with AltGr (Ctrl+Alt)', async () => {
    const backend = fakeBackend();
    show();
    await userEvent.keyboard('{Control>}{Alt>}1{/Alt}{/Control}');
    app.modal = { kind: 'settings' };
    await alt('1');
    app.modal = null;
    document.body.insertAdjacentHTML('beforeend', '<div class="xterm"><textarea aria-label="terminal"></textarea></div>');
    screen.getByLabelText('terminal').focus();
    await alt('1');
    expect(backend.called('answer_question')).toHaveLength(0);
  });

  it('shows the digits on the options of the question the keys are for, and the key of the validation', async () => {
    fakeBackend();
    show({ questions: [base, tools] });
    const pg = screen.getByRole('button', { name: 'PG' });
    expect(pg).toHaveTextContent('Alt+1');
    expect(pg).toHaveAttribute('aria-keyshortcuts', 'Alt+1');
    expect(screen.getByRole('button', { name: 'SQLite' })).toHaveAttribute('aria-keyshortcuts', 'Alt+2');
    expect(screen.getByRole('button', { name: /ESLint/ })).not.toHaveAttribute('aria-keyshortcuts');
    const validate = screen.getByRole('button', { name: 'Valider' });
    expect(validate).not.toHaveAttribute('aria-keyshortcuts');
    await alt('1'); // on to Outils
    expect(screen.getByRole('button', { name: /ESLint/ })).toHaveAttribute('aria-keyshortcuts', 'Alt+1');
    expect(screen.getByRole('button', { name: 'PG' })).not.toHaveAttribute('aria-keyshortcuts');
    await alt('2');
    expect(validate).toHaveTextContent('Ctrl+Entrée');
    expect(validate).toHaveAttribute('aria-keyshortcuts', 'Control+Enter');
  });

  it('shows no key on a question the keyboard does not answer', () => {
    show({}, { current: false });
    expect(screen.queryByText('Alt+1')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'PostgreSQL' })).not.toHaveAttribute('aria-keyshortcuts');
  });

  it('shows the digit without changing the name of the option', () => {
    show();
    expect(screen.getByRole('button', { name: 'PostgreSQL' })).toHaveTextContent('Alt+1');
  });

  it('gives the focus back to the message field once answered while it was on the card', async () => {
    fakeBackend();
    show();
    screen.getByRole('button', { name: 'SQLite' }).focus();
    const focus = app.focusComposer;
    await alt('1');
    await waitFor(() => expect(app.focusComposer).toBe(focus + 1));
  });
});

describe('QuestionCard in English', () => {
  beforeEach(() => resetApp());

  it('waits for the answer in English, and validates several answers with a button of its own', async () => {
    setLang('en');
    const backend = fakeBackend();
    const two = item({
      questions: [
        { question: 'Which database?', header: 'Database', multiSelect: false, options: [{ label: 'PG' }, { label: 'SQLite' }] },
        { question: 'Which tools?', header: 'Tools', multiSelect: true, options: [{ label: 'ESLint' }, { label: 'Vitest' }] },
      ],
    });
    render(QuestionCard, { item: two, agentId: 'a1', pending: true });
    expect(screen.getByRole('group', { name: 'Claude is waiting for your answer' })).toBeInTheDocument();
    expect(screen.getByText('or answer freely in the field below')).toBeInTheDocument();
    const submit = screen.getByRole('button', { name: 'Submit' });
    expect(submit).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'PG' }));
    await userEvent.click(screen.getByRole('button', { name: /ESLint/ }));
    expect(submit).toBeEnabled();
    expect(submit).toHaveTextContent('Ctrl+Enter');
    await userEvent.click(submit);
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Which database?': 'PG', 'Which tools?': 'ESLint' });
  });

  it('says in English that a question was left unanswered', () => {
    setLang('en');
    render(QuestionCard, { item: item(), agentId: 'a1', pending: false });
    expect(screen.getByText('Question left unanswered')).toBeInTheDocument();
  });
});
