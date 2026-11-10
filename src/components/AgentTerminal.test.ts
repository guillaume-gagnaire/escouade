import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// The size of the place the interface is shown in, as `roomOf` measures it (jsdom has no layout).
interface Room {
  width: number;
  height: number;
}

// xterm.js has no place in jsdom: the terminals are counted, not drawn.
const terms = vi.hoisted(() => ({
  opened: [] as string[],
  /** Whether each start was asked for by the user (« Relancer »), in the order of the starts. */
  manual: [] as boolean[],
  /** The size each interface was started at (the `room` openAgentTerminal got). */
  rooms: [] as (Room | null | undefined)[],
  mounted: [] as (string | null)[],
  disposed: [] as string[],
  held: new Set<string>(),
  /** What the backend’s kill of a terminal is waiting on: settled by default. */
  kill: Promise.resolve() as Promise<void>,
  /** While set, a start is not over: the interface is still starting. */
  gate: null as Promise<void> | null,
  /** What `roomOf` measures, and the elements it was asked to measure. */
  room: null as Room | null,
  measured: [] as HTMLElement[],
  /** How many times a terminal was fitted to its box, and whether fitting it throws (a box that is hidden). */
  fits: 0,
  failFit: false,
  /** What the dictation typed into each terminal. */
  typed: [] as [string, string][],
}));
vi.mock('../lib/terminals', () => ({
  openAgentTerminal: async (agentId: string, room?: Room | null, manual = false) => {
    const id = `t${terms.opened.length + 1}`;
    terms.opened.push(agentId);
    terms.manual.push(manual);
    terms.rooms.push(room);
    terms.held.add(id);
    if (terms.gate) await terms.gate;
    return { id, projectId: '', name: 'x', shell: 'claude' };
  },
  roomOf: (el: HTMLElement) => {
    terms.measured.push(el);
    return terms.room;
  },
  getXTerm: (id: string) =>
    terms.held.has(id)
      ? {
          fit: {
            fit() {
              terms.fits++;
              if (terms.failFit) throw new Error('not measurable');
            },
          },
        }
      : undefined,
  mountTerminal: (id: string, el: HTMLElement | null) => terms.mounted.push(el ? id : null),
  typeIntoTerminal: async (id: string, text: string) => {
    terms.typed.push([id, text]);
  },
  disposeTerminal: (id: string) => {
    terms.disposed.push(id);
    terms.held.delete(id);
    return terms.kill;
  },
}));

import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { Agent, TuiExit } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import { queueViewCommand, takeViewCommand } from '../lib/interactive';
import AgentTerminal from './AgentTerminal.svelte';

beforeEach(() => {
  terms.opened = [];
  terms.manual = [];
  terms.rooms = [];
  terms.mounted = [];
  terms.disposed = [];
  terms.held.clear();
  terms.kill = Promise.resolve();
  terms.gate = null;
  terms.room = { width: 1124, height: 932 };
  terms.measured = [];
  terms.fits = 0;
  terms.failFit = false;
  terms.typed = [];
  setLang('fr');
});

afterEach(() => {
  vi.useRealTimers();
});

/** Lets what is under way (promises, effects, the events that follow) come to its end. */
async function settle() {
  for (let i = 0; i < 4; i++) {
    await tick();
    await new Promise((r) => setTimeout(r, 0));
  }
}

const exit = (over: Partial<TuiExit> = {}): TuiExit => ({ code: 1, quick: true, sessionLost: false, ...over });

type Rendered = { rerender: (props: { agent: Agent }) => Promise<void> };

/**
 * The interface `term` stops, as the backend tells it: the terminal's exit first, then the agent, replaced by a new
 * object (as the app does with each update of it) with no terminal any more and how it ended.
 */
async function stops(r: Rendered, a: Agent, term: string, tuiExit: TuiExit | null, code: number | null = tuiExit?.code ?? 0) {
  app.exitedTerms[term] = code;
  await settle();
  await r.rerender({ agent: { ...a, terminalId: null, tuiExit } });
  await settle();
}

const QUICK = 'Claude Code s’est arrêté dès son lancement (code 1). Regarde sa sortie ci-dessus, puis relance-le.';
const NO_SESSION = 'Session Claude introuvable : l’interface repart en nouvelle session.';

describe('the agent in Claude Code’s interface', () => {
  it('starts its interface once and shows it', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    expect(terms.opened).toEqual([a.id]);
  });

  it('types the interactive command the composer sent it to once its interface takes it (ESC-401)', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    let n = 0;
    const b = fakeBackend({
      send_message: () => {
        // Not ready yet the first time, as the backend says of an interface that is starting.
        if (n++ === 0) throw new Error('pas encore prête');
      },
    });
    queueViewCommand(a.id, '/hooks');
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(b.called('send_message')).toHaveLength(2), { timeout: 3000 });
    expect(b.called('send_message').map((c) => c.args.text)).toEqual(['/hooks', '/hooks']);
    // Once only.
    expect(takeViewCommand(a.id)).toBeUndefined();
  });

  it('shows the one this window holds already rather than starting another', async () => {
    terms.held.add('t9');
    const a = agent({ terminal: true, terminalId: 't9' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t9'));
    expect(terms.opened).toEqual([]);
  });

  it('offers to start it again once it stopped', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    app.exitedTerms['t1'] = 0;
    await userEvent.click(await screen.findByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.disposed).toEqual(['t1']);
    expect(screen.queryByText('Claude Code s’est arrêté.')).toBeNull();
  });

  it('opens the new interface only once the old one is stopped: the backend would attach to one still running', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    // Its kill is not done yet when « Relancer » is clicked.
    let stopped!: () => void;
    terms.kill = new Promise<void>((resolve) => (stopped = resolve));
    app.exitedTerms['t1'] = 0;
    await userEvent.click(await screen.findByRole('button', { name: 'Relancer' }));
    expect(terms.disposed).toEqual(['t1']);
    // Nothing is opened while it runs, however long it takes.
    await new Promise((r) => setTimeout(r, 50));
    expect(terms.opened).toEqual([a.id]);
    expect(screen.getByText('Démarrage de Claude Code…')).toBeInTheDocument();
    stopped();
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.opened).toEqual([a.id, a.id]);
  });

  it('starts only one new interface when another restart is asked while the old one is being stopped', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    let stopped!: () => void;
    terms.kill = new Promise<void>((resolve) => (stopped = resolve));
    app.exitedTerms['t1'] = 0;
    await userEvent.click(await screen.findByRole('button', { name: 'Relancer' }));
    // « Outils d’Escouade au prochain démarrage » asks for one too.
    app.relaunched[a.id] = 1;
    await new Promise((r) => setTimeout(r, 20));
    stopped();
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    await new Promise((r) => setTimeout(r, 50));
    expect(terms.disposed).toEqual(['t1']);
    expect(terms.opened).toEqual([a.id, a.id]);
  });

  it('starts it again when first asked from elsewhere, the count not there before', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    app.relaunched[a.id] = 1;
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.disposed).toEqual(['t1']);
  });

  it('starts it again when asked from elsewhere (« Outils d’Escouade au prochain démarrage »), never at its first showing', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    // Asked before this view showed it: not asked of this one.
    app.relaunched[a.id] = 1;
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    expect(terms.opened).toEqual([a.id]);
    app.relaunched[a.id] = 2;
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.disposed).toEqual(['t1']);
    expect(terms.opened).toEqual([a.id, a.id]);
    // What the user asked, from the banner: not what the view does of itself.
    expect(terms.manual).toEqual([false, true]);
  });

  it('asks for it as the user’s when « Relancer » is clicked, and as nobody’s at its first showing', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    app.exitedTerms['t1'] = 0;
    await userEvent.click(await screen.findByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.manual).toEqual([false, true]);
  });
});

// ESC-106: the app replaces the agent by a new object with each update of it, and the interface that stopped sets its
// terminal to null: the view looked at the object, started a new interface, which stopped, and so on, for ever.
describe('an agent replaced while its interface is stopped', () => {
  it('does not start another interface for it', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    app.exitedTerms['t1'] = 1;
    await r.rerender({ agent: { ...a, terminalId: null, status: 'idle', lastActivity: 99 } });
    await settle();
    expect(terms.opened).toEqual([a.id]);
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
  });

  it('does not, however many times it is replaced', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    app.exitedTerms['t1'] = 1;
    for (let i = 0; i < 5; i++) {
      await r.rerender({ agent: { ...a, terminalId: null, tokens: i, tuiExit: exit({ code: i }) } });
      await settle();
    }
    expect(terms.opened).toEqual([a.id]);
  });

  it('does not either while its interface runs: the agent is replaced by each update of it', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    for (let i = 0; i < 3; i++) await r.rerender({ agent: { ...a, terminalId: 't1', tokens: i } });
    await settle();
    expect(terms.opened).toEqual([a.id]);
  });

  it('starts the interface of another agent, though, when given one', async () => {
    const a = agent({ id: 'a1', terminal: true });
    const b = agent({ id: 'a2', terminal: true });
    resetApp({ projects: [project()], agents: [a, b] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await r.rerender({ agent: b });
    await waitFor(() => expect(terms.opened).toEqual(['a1', 'a2']));
  });
});

describe('an interface that stops', () => {
  it('says so, once it stopped after some time: nothing but « Relancer »', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit({ code: 0, quick: false }));
    expect(screen.getByText('Claude Code s’est arrêté.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
    expect(terms.opened).toHaveLength(1);
  });

  it('tells it stopped as soon as it started, with its code, and starts nothing by itself', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit({ code: 1 }));
    expect(screen.getByText(QUICK)).toBeInTheDocument();
    expect(screen.queryByText('Claude Code s’est arrêté.')).toBeNull();
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
    expect(terms.opened).toHaveLength(1);
    expect(terms.disposed).toEqual([]);
  });

  it('tells it without a code when it has none', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit({ code: null }), null);
    expect(
      screen.getByText('Claude Code s’est arrêté dès son lancement. Regarde sa sortie ci-dessus, puis relance-le.'),
    ).toBeInTheDocument();
  });

  it('is written in English when the interface is', async () => {
    setLang('en');
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit({ code: 2 }));
    expect(
      screen.getByText('Claude Code stopped as soon as it started (code 2). Read its output above, then restart it.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Restart' })).toBeInTheDocument();
  });

  it('is the user’s « Relancer » that starts it again after it stopped as soon as it started', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit());
    await userEvent.click(screen.getByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.manual).toEqual([false, true]);
    expect(screen.queryByText(QUICK)).toBeNull();
  });

  it('waits for the agent to know how it ended before it acts on it: the terminal’s exit comes first', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    app.exitedTerms['t1'] = 1;
    await settle();
    // Only the exit so far: « Relancer » is there, nothing starts by itself.
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
    expect(terms.opened).toHaveLength(1);
    await r.rerender({ agent: { ...a, terminalId: null, tuiExit: exit({ sessionLost: true }) } });
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.opened).toHaveLength(2);
  });
});

describe('a session Claude Code did not find', () => {
  it('starts the interface again by itself, once, in the new session the backend made', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit({ sessionLost: true }));
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.opened).toEqual([a.id, a.id]);
    expect(terms.manual).toEqual([false, false]);
    expect(terms.disposed).toEqual(['t1']);
    // It runs: nothing to tell any more.
    expect(screen.queryByRole('button', { name: 'Relancer' })).toBeNull();
  });

  it('says it while it starts it again', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    let open!: () => void;
    terms.gate = new Promise<void>((resolve) => (open = resolve));
    await stops(r, a, 't1', exit({ sessionLost: true }));
    expect(terms.opened).toHaveLength(2);
    expect(screen.getByText(NO_SESSION)).toBeInTheDocument();
    expect(screen.queryByText('Démarrage de Claude Code…')).toBeNull();
    open();
    await waitFor(() => expect(terms.mounted).toContain('t2'));
  });

  it('says it in English too', async () => {
    setLang('en');
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    let open!: () => void;
    terms.gate = new Promise<void>((resolve) => (open = resolve));
    await stops(r, a, 't1', exit({ sessionLost: true }));
    expect(screen.getByText('Claude session not found: the interface restarts in a new session.')).toBeInTheDocument();
    open();
    await waitFor(() => expect(terms.mounted).toContain('t2'));
  });

  it('takes the note back a few seconds after the new interface is there', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    app.exitedTerms['t1'] = 1;
    await tick();
    await r.rerender({ agent: { ...a, terminalId: null, tuiExit: exit({ sessionLost: true }) } });
    for (let i = 0; i < 8; i++) await tick();
    expect(terms.mounted).toContain('t2');
    expect(screen.getByText(NO_SESSION)).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(10_000);
    await tick();
    expect(screen.queryByText(NO_SESSION)).toBeNull();
  });

  it('never starts it again by itself a second time: one start more, then the message and « Relancer »', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    // Each interface stops as soon as it starts, its session not found.
    const lost = exit({ sessionLost: true });
    await stops(r, a, 't1', lost);
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    await stops(r, a, 't2', lost);
    // And the agent is replaced again, and again.
    for (let i = 0; i < 5; i++) {
      await r.rerender({ agent: { ...a, terminalId: null, tokens: i, tuiExit: { ...lost } } });
      await settle();
    }
    expect(terms.opened).toHaveLength(2);
    expect(terms.manual).toEqual([false, false]);
    expect(screen.getByText(QUICK)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
    expect(screen.queryByText(NO_SESSION)).toBeNull();
  });

  it('goes back to the normal message when the second interface stops after some time', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await stops(r, a, 't1', exit({ sessionLost: true }));
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    await stops(r, a, 't2', exit({ code: 0, quick: false }));
    expect(screen.getByText('Claude Code s’est arrêté.')).toBeInTheDocument();
    expect(terms.opened).toHaveLength(2);
  });

  it('may start itself again after the user’s « Relancer »: that one is a new try', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    const lost = exit({ sessionLost: true });
    await stops(r, a, 't1', lost);
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    await stops(r, a, 't2', lost);
    await userEvent.click(screen.getByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(terms.mounted).toContain('t3'));
    expect(terms.manual).toEqual([false, false, true]);
    await stops(r, a, 't3', lost);
    await waitFor(() => expect(terms.mounted).toContain('t4'));
    expect(terms.manual).toEqual([false, false, true, false]);
    await stops(r, a, 't4', lost);
    expect(terms.opened).toHaveLength(4);
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
  });

  it('is not started again for the sake of what was told before: only the stop of the interface it shows counts', async () => {
    // The session was lost the time before; this interface runs.
    const a = agent({ terminal: true, tuiExit: exit({ sessionLost: true }) });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    await r.rerender({ agent: { ...a, terminalId: 't1', tokens: 3, tuiExit: exit({ sessionLost: true }) } });
    await settle();
    expect(terms.opened).toEqual([a.id]);
    expect(screen.queryByRole('button', { name: 'Relancer' })).toBeNull();
  });

  it('shows the failure of that start, with « Relancer », should the backend refuse it', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const r = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    terms.gate = Promise.reject(new Error('refused'));
    terms.gate.catch(() => {});
    await stops(r, a, 't1', exit({ sessionLost: true }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Claude Code n’a pas démarré');
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
    expect(screen.queryByText(NO_SESSION)).toBeNull();
    expect(terms.opened).toHaveLength(2);
  });
});

describe('the size Claude Code’s interface starts at', () => {
  /** The box the interface is shown in. */
  const box = () => document.querySelector('.box') as HTMLElement;

  it('is that of its box: the interface draws on the rows of its pseudo-console, so it starts at the size it will have', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    terms.room = { width: 1124, height: 932 };
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    expect(terms.rooms).toEqual([{ width: 1124, height: 932 }]);
    // It is the box that was measured, once: not some other element, and not again at each render.
    expect(terms.measured).toEqual([box()]);
  });

  it('starts all the same when its box has no size yet (not laid out, or hidden)', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    terms.room = null;
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    expect(terms.opened).toEqual([a.id]);
    expect(terms.rooms).toEqual([null]);
  });

  it('is measured again for the interface started again: the box may have another size by then', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    terms.room = { width: 800, height: 600 };
    app.exitedTerms['t1'] = 0;
    await userEvent.click(await screen.findByRole('button', { name: 'Relancer' }));
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    expect(terms.rooms).toEqual([
      { width: 1124, height: 932 },
      { width: 800, height: 600 },
    ]);
  });

  it('is not asked for an interface this window holds already', async () => {
    terms.held.add('t9');
    const a = agent({ terminal: true, terminalId: 't9' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t9'));
    expect(terms.rooms).toEqual([]);
  });
});

describe('the interface following its box', () => {
  /** The observers the component made, with what they watch; `fire` is the WebView telling the box changed. */
  const observers: FakeObserver[] = [];
  class FakeObserver {
    targets: Element[] = [];
    disconnected = false;
    constructor(private callback: ResizeObserverCallback) {
      observers.push(this);
    }
    observe(el: Element) {
      this.targets.push(el);
    }
    unobserve() {}
    disconnect() {
      this.disconnected = true;
    }
    /** A disconnected observer reports nothing, as in a browser. */
    fire() {
      if (!this.disconnected) this.callback([], this as unknown as ResizeObserver);
    }
  }

  beforeEach(() => {
    observers.length = 0;
    // The setup’s observer watches nothing: this one can be set off by hand.
    vi.stubGlobal('ResizeObserver', FakeObserver);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  async function shown() {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    const view = render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    return view;
  }

  it('watches its box, once the interface is shown in it', async () => {
    await shown();
    expect(observers).toHaveLength(1);
    expect(observers[0].targets).toEqual([document.querySelector('.box')]);
    // Nothing is fitted before the box changes: the interface started at its size.
    expect(terms.fits).toBe(0);
  });

  it('fits the interface when the box takes its size after it was shown, once for each change', async () => {
    await shown();
    observers[0].fire();
    expect(terms.fits).toBe(1);
    observers[0].fire();
    observers[0].fire();
    expect(terms.fits).toBe(3);
  });

  it('stops watching the box when the view goes: no fit after', async () => {
    const view = await shown();
    observers[0].fire();
    expect(terms.fits).toBe(1);
    view.unmount();
    expect(observers[0].disconnected).toBe(true);
    observers[0].fire();
    expect(terms.fits).toBe(1);
    expect(terms.mounted.at(-1)).toBeNull();
  });

  it('does not stop for a box it cannot measure (hidden), nor for an interface that is gone', async () => {
    await shown();
    terms.failFit = true;
    expect(() => observers[0].fire()).not.toThrow();
    terms.failFit = false;
    terms.held.delete('t1');
    expect(() => observers[0].fire()).not.toThrow();
    // Gone: nothing to fit, so the one count of the failed attempt is all there is.
    expect(terms.fits).toBe(1);
  });
});

describe('dictation in Claude Code’s interface', () => {
  it('types the text dictated into the terminal, as a paste, and never sends it', async () => {
    const a = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    const backend = fakeBackend({
      voice_start: () => ({ kind: 'listening', session: 9 }),
      voice_stop: () => ' Explique ce test ',
    });
    render(AgentTerminal, { agent: a });
    await waitFor(() => expect(terms.mounted).toContain('t1'));
    const mic = screen.getByRole('button', { name: 'Dicter' });
    await fireEvent.pointerDown(mic, { button: 0 });
    await fireEvent.pointerUp(mic, { button: 0 });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Finir la dictée' })).toBeInTheDocument());
    await fireEvent.pointerDown(screen.getByRole('button', { name: 'Finir la dictée' }), { button: 0 });
    await waitFor(() => expect(terms.typed).toEqual([['t1', 'Explique ce test']]));
    expect(backend.called('voice_stop')[0].args).toEqual({ session: 9, target: 'terminal' });
    expect(backend.called('term_write')).toHaveLength(0);
  });
});

describe('the band above the harness’s terminal', () => {
  it('says in amber that Codex asks about the trust of the folder, until it no longer does', async () => {
    let asks = true;
    const a = agent({ terminal: true, engine: 'codex' });
    resetApp({ projects: [project()], agents: [a] });
    const backend = fakeBackend({ tui_asks_trust: () => asks });
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(AgentTerminal, { agent: a });
    const band = await screen.findByRole('status');
    expect(band).toHaveClass('trust');
    expect(band.textContent).toContain('Codex demande s’il peut faire confiance au dossier de l’agent.');
    expect(band.textContent).toContain('sans confiance, il s’arrête');
    expect(band.querySelector('[data-icon="shield"]')).not.toBeNull();
    expect(backend.called('tui_asks_trust')[0].args).toEqual({ id: a.id });
    // Answered in the terminal: the band goes, and the view asks no more.
    asks = false;
    await vi.advanceTimersByTimeAsync(1600);
    await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
    const n = backend.called('tui_asks_trust').length;
    await vi.advanceTimersByTimeAsync(5000);
    expect(backend.called('tui_asks_trust')).toHaveLength(n);
  });

  it('says it for Copilot CLI too, in its words', async () => {
    const a = agent({ terminal: true, engine: 'copilot' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ tui_asks_trust: () => true });
    render(AgentTerminal, { agent: a });
    const band = await screen.findByRole('status');
    expect(band.textContent).toContain('Copilot CLI demande s’il peut faire confiance au dossier de l’agent.');
    expect(band.textContent).toContain('Escouade ne répond pas à ta place');
  });

  it('says it for Cursor CLI too, with the answer its interface waits for', async () => {
    const a = agent({ terminal: true, engine: 'cursor' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ tui_asks_trust: () => true });
    render(AgentTerminal, { agent: a });
    const band = await screen.findByRole('status');
    expect(band).toHaveClass('trust');
    expect(band.textContent).toContain('Cursor CLI demande s’il peut faire confiance au dossier de l’agent.');
    expect(band.textContent).toContain('« Trust this workspace »');
    expect(band.textContent).toContain('Escouade ne répond pas à ta place');
  });

  it('says in Cursor’s view that its following is limited, and what Escouade does not know, once the folder is trusted', async () => {
    let asks = true;
    const a = agent({ terminal: true, engine: 'cursor' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ tui_asks_trust: () => asks });
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(AgentTerminal, { agent: a });
    await screen.findByRole('status');
    expect(screen.queryByRole('note')).not.toBeInTheDocument();
    asks = false;
    await vi.advanceTimersByTimeAsync(1600);
    const note = await screen.findByRole('note');
    expect(note).toHaveClass('limited');
    expect(note.querySelector('b')?.textContent).toBe('Suivi limité');
    expect(note.querySelector('[data-icon="info"]')).not.toBeNull();
    // What it sees, then what it does not know.
    expect(note.textContent).toContain('Escouade voit tes messages, les réponses de Cursor et la fin de chaque tour');
    expect(note.textContent).toContain('mais pas ses permissions ni ses questions en attente');
    expect(note.textContent).toContain('ni le résultat de ses outils, ni les tokens');
    // Its conversation apart, which Escouade links to the agent's.
    expect(note.textContent).toContain('Escouade la relie à celle de l’agent');
  });

  it('says it in English too', async () => {
    setLang('en');
    const a = agent({ terminal: true, engine: 'cursor' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ tui_asks_trust: () => false });
    render(AgentTerminal, { agent: a });
    const note = await screen.findByRole('note');
    expect(note.textContent?.trim()).toBe(
      'Limited following: Escouade sees your messages, Cursor’s replies and the end of each turn, but not its permissions or questions waiting, nor the results of its tools, nor the tokens: look at the terminal. This conversation is Cursor’s own: Escouade links it to the agent’s.',
    );
  });

  it('says in OpenCode’s view that Escouade keeps the thread, and asks nothing of Claude Code’s', async () => {
    const oc = agent({ terminal: true, engine: 'opencode' });
    resetApp({ projects: [project()], agents: [oc] });
    const backend = fakeBackend();
    const { unmount } = render(AgentTerminal, { agent: oc });
    const note = await screen.findByRole('note');
    expect(note.querySelector('b')?.textContent).toBe('Escouade garde le fil');
    expect(note.textContent).toContain('Revenir à la vue Escouade n’arrête que l’interface.');
    unmount();
    const cc = agent({ terminal: true });
    resetApp({ projects: [project()], agents: [cc] });
    render(AgentTerminal, { agent: cc });
    await waitFor(() => expect(terms.mounted).toContain('t2'));
    await settle();
    expect(screen.queryByRole('note')).not.toBeInTheDocument();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(backend.called('tui_asks_trust')).toHaveLength(0);
  });

  it('is written in English when the interface is', async () => {
    setLang('en');
    const a = agent({ terminal: true, engine: 'opencode' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend();
    render(AgentTerminal, { agent: a });
    const note = await screen.findByRole('note');
    expect(note.textContent?.trim()).toBe(
      'Escouade keeps the thread: the conversation, the card, the notifications and the cost stay up to date while you work here. Going back to the Escouade view only stops the interface.',
    );
  });
});
