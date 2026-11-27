// The screen (AB-9, docs/AGENT-BROWSERS.md § 5.5): an agent's browser opens beside its conversation only while the user
// looks at it, and the conversation stays the active pane; elsewhere its row waits and opens when they come back; one
// pane per agent; a pane the user closed is never opened again until they have left the conversation and come back;
// the window's own moves are no close of theirs; « Laisser piloter par… » lends a browser the user opened alone.
import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { flushSync, tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { devpages, dismissals, dismissed, inPane, lendBrowser, onBrowserOpened, onPageEvent, type PageState } from '../lib/devpage.svelte';
import { setLang } from '../lib/i18n';
import { menu, type MenuItem } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { BrowserOpened, PageView } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import BrowsersSection from './BrowsersSection.svelte';

const view = (over: Partial<PageView> = {}): PageView => ({
  agentId: 'browser:t1',
  projectId: 'p1',
  title: 'localhost:4100',
  url: 'http://localhost:4100/',
  loaded: true,
  opener: 'a7',
  driver: 'a7',
  ...over,
});
const opened = (over: Partial<BrowserOpened> = {}): BrowserOpened => ({
  view: view(),
  agent: 'a7',
  wish: 'beside',
  place: 'beside',
  ...over,
});

/** The user looks at the conversation of `id`, alone in the work area; null: the board. */
function lookAt(id: string | null) {
  app.ui.activeProject = 'p1';
  app.ui.view = 'project';
  app.board.p1 = false;
  app.selectedPage.p1 = null;
  app.selectedTerm.p1 = null;
  if (id) app.ui.selectedAgent.p1 = id;
  else app.openBoard('p1');
  flushSync();
}

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [agent({ id: 'a7', name: 'esc-412' }), agent({ id: 'a8', name: 'esc-413' })] });
  app.settings = { ...app.settings, mcpEnabled: true, agentBrowsers: true };
  app.ui.panes = {};
  devpages.pages = {};
  devpages.asks = [];
  // The rows of the test before leave the screen now: no close of the user's.
  flushSync();
  for (const k of Object.keys(dismissals)) delete dismissals[k];
});

describe('beside the conversation, only while the user looks at it', () => {
  it('opens beside it, the conversation staying the active pane', async () => {
    fakeBackend();
    lookAt('a7');
    onBrowserOpened(opened());
    await tick();
    expect(inPane('browser:t1')).toBe(true);
    expect(app.panes?.active).toBe('agent:a7');
  });

  it('never takes the screen elsewhere: the user left before the window heard of it, the row waits', async () => {
    fakeBackend();
    lookAt('a7');
    // The backend saw them on a7's conversation; they went to the board meanwhile.
    app.openBoard('p1');
    onBrowserOpened(opened({ wish: 'below', place: 'below' }));
    await tick();
    expect(inPane('browser:t1')).toBe(false);
    expect(app.boardOn).toBe(true);
    expect(devpages.pages['browser:t1'].waiting).toBe('below');
    // Another agent's conversation is elsewhere too.
    lookAt('a8');
    expect(inPane('browser:t1')).toBe(false);
    expect(app.shownAgents).toEqual(['a8']);
    // Back on a7's: it opens under it.
    lookAt('a7');
    await tick();
    expect(inPane('browser:t1')).toBe(true);
    expect(devpages.pages['browser:t1'].waiting).toBeUndefined();
  });

  it('keeps one pane per agent, the window’s move no close of the user’s', async () => {
    const b = fakeBackend();
    lookAt('a7');
    onBrowserOpened(opened());
    await tick();
    onBrowserOpened(opened({ view: view({ agentId: 'browser:t2' }) }));
    await tick();
    flushSync();
    expect(inPane('browser:t2')).toBe(true);
    expect(inPane('browser:t1')).toBe(false);
    expect(dismissed('a7')).toBe(false);
    expect(b.called('devpage_dismiss')).toEqual([]);
  });
});

describe('nothing opened again after the user closed it', () => {
  it('holds after its pane’s ×, until the user leaves the conversation and comes back', async () => {
    const b = fakeBackend();
    lookAt('a7');
    onBrowserOpened(opened());
    await tick();
    app.closePaneOf('page:browser:t1');
    flushSync();
    expect(dismissed('a7')).toBe(true);
    await waitFor(() => expect(b.called('devpage_dismiss').map((c) => c.args)).toEqual([{ agent: 'a7' }]));
    // The agent shows it again (the backend answered before it heard): nothing opens, nothing waits.
    onBrowserOpened(opened());
    await tick();
    expect(inPane('browser:t1')).toBe(false);
    expect(devpages.pages['browser:t1'].waiting).toBeUndefined();
    // Still on its conversation, then on another's: still held.
    lookAt('a8');
    expect(dismissed('a7')).toBe(true);
    // Back on it: lifted, and the next opening goes beside it.
    lookAt('a7');
    expect(dismissed('a7')).toBe(false);
    onBrowserOpened(opened());
    await tick();
    expect(inPane('browser:t1')).toBe(true);
  });

  it('holds after its row’s ×, and after going back to one pane', async () => {
    fakeBackend();
    render(BrowsersSection, { project: project() });
    lookAt('a7');
    onBrowserOpened(opened());
    await tick();
    const row = document.querySelector<HTMLElement>('.row-item.browser[data-browser="browser:t1"]')!;
    await fireEvent.click(row.querySelector<HTMLElement>('button[aria-label="Fermer le navigateur"]')!);
    flushSync();
    expect(devpages.pages['browser:t1']).toBeUndefined();
    expect(dismissed('a7')).toBe(true);
    lookAt(null);
    lookAt('a7');
    expect(dismissed('a7')).toBe(false);
    onBrowserOpened(opened({ view: view({ agentId: 'browser:t2' }) }));
    await tick();
    expect(inPane('browser:t2')).toBe(true);
    app.unsplit();
    flushSync();
    expect(app.shownAgents).toEqual(['a7']);
    expect(dismissed('a7')).toBe(true);
  });

  it('is no close when its agent closes it, nor when the user goes back to the conversation from it shown alone', async () => {
    const b = fakeBackend();
    lookAt('a7');
    onBrowserOpened(opened());
    await tick();
    onPageEvent({ agentId: 'browser:t1', phase: 'closed', url: '' });
    flushSync();
    expect(devpages.pages['browser:t1']).toBeUndefined();
    expect(dismissed('a7')).toBe(false);
    onBrowserOpened(opened({ view: view({ agentId: 'browser:t3' }), wish: 'full', place: 'full' }));
    await tick();
    flushSync();
    expect(app.pageShown).toBe('browser:t3');
    lookAt('a7');
    expect(dismissed('a7')).toBe(false);
    expect(b.called('devpage_dismiss')).toEqual([]);
  });

  it('tells the backend nothing while the tools of the browsers do not answer', async () => {
    const b = fakeBackend();
    app.settings = { ...app.settings, agentBrowsers: false };
    lookAt('a7');
    onBrowserOpened(opened());
    await tick();
    app.closePaneOf('page:browser:t1');
    flushSync();
    expect(dismissed('a7')).toBe(true);
    expect(b.called('devpage_dismiss')).toEqual([]);
  });
});

describe('« Laisser piloter par… »', () => {
  const mine = (over: Partial<PageState> = {}): PageState => ({
    url: 'https://example.com/',
    title: 'example.com',
    phase: 'ready',
    projectId: 'p1',
    by: { kind: 'user' },
    opened: 1,
    ...over,
  });
  const lend = (): MenuItem | undefined => menu.open?.items.find((i) => i.label === 'Laisser piloter par…');

  it('lends a browser the user opened alone to a live agent of its project, or to nobody', async () => {
    const b = fakeBackend({
      browser_drive_state: () => ({
        browser: 'browser:u1',
        hand: 'agent',
        stopped: false,
        step: 0,
        what: '',
        waiting: false,
        passwordAsks: [],
        pins: [],
      }),
    });
    app.agents.a9 = agent({ id: 'a9', name: 'archivé', archived: true });
    app.agents.z1 = agent({ id: 'z1', name: 'ailleurs', projectId: 'p2' });
    devpages.pages = { 'browser:u1': mine() };
    render(BrowsersSection, { project: project() });
    const row = document.querySelector<HTMLElement>('.row-item.browser[data-browser="browser:u1"]')!;
    await fireEvent.contextMenu(row);
    expect(lend()?.submenu?.map((i) => i.label)).toEqual(['esc-412', 'esc-413']);
    lend()!.submenu![1].onClick!();
    await waitFor(() => expect(devpages.pages['browser:u1'].driver).toBe('a8'));
    expect(b.called('devpage_lend').map((c) => c.args)).toEqual([{ key: 'browser:u1', agent: 'a8' }]);
    // Its menu ticks the one it is lent to, and offers « Personne ».
    await fireEvent.contextMenu(row);
    expect(lend()?.submenu?.find((i) => i.checked)?.label).toBe('esc-413');
    lend()!.submenu!.find((i) => i.label === 'Personne')!.onClick!();
    await waitFor(() => expect(devpages.pages['browser:u1'].driver).toBeUndefined());
    expect(b.called('devpage_lend').at(-1)?.args).toEqual({ key: 'browser:u1', agent: null });
    // Nothing moved on screen.
    expect(inPane('browser:u1')).toBe(false);
  });

  it('is offered on a browser the user opened alone, never on an agent’s tab nor its launch', async () => {
    fakeBackend();
    devpages.pages = {
      'browser:t1': mine({ opener: 'a7', driver: 'a7' }),
      a7: mine({ by: { kind: 'agent', id: 'a7' }, opened: 2 }),
    };
    render(BrowsersSection, { project: project() });
    for (const key of ['browser:t1', 'a7']) {
      await fireEvent.contextMenu(document.querySelector<HTMLElement>(`.row-item.browser[data-browser="${key}"]`)!);
      expect(lend()).toBeUndefined();
    }
  });

  it('says a refusal of the backend, and lends nothing then', async () => {
    fakeBackend({
      devpage_lend: () => {
        throw 'Cet agent n’est pas un agent actif du projet de ce navigateur.';
      },
    });
    devpages.pages = { 'browser:u1': mine() };
    await lendBrowser('browser:u1', 'a7');
    expect(devpages.pages['browser:u1'].driver).toBeUndefined();
  });
});
