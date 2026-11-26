// An agent opens a browser (ESC-678, AB-6): the band « veut ouvrir » of its conversation for an address outside the
// project's launch (choice C3), the row it opened, shown where it wants while the user looks at its conversation and
// waiting otherwise, one pane per agent, and what its closing does to the row.
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync, tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import {
  answerOpen,
  devpages,
  inPane,
  loadOpenAsks,
  onBrowserOpened,
  onPageEvent,
  openAsksOf,
  type PageState,
} from '../lib/devpage.svelte';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { BrowserOpenAsk, BrowserOpened, PageView } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import BrowsersSection from './BrowsersSection.svelte';
import OpenAskBand from './OpenAskBand.svelte';

const ASK: BrowserOpenAsk = {
  id: 'o-1',
  agent: 'a7',
  projectId: 'p1',
  origin: 'https://docs.stripe.com',
  url: 'https://docs.stripe.com/api',
  wish: 'beside',
};

const view = (over: Partial<PageView> = {}): PageView => ({
  agentId: 'browser:t1',
  projectId: 'p1',
  title: 'localhost:4100',
  url: 'http://localhost:4100/',
  loaded: false,
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

/** The user looks at the conversation of `id`, alone in the work area. */
function lookAt(id: string | null) {
  app.ui.activeProject = 'p1';
  app.ui.view = 'project';
  app.board.p1 = false;
  app.selectedPage.p1 = null;
  app.selectedTerm.p1 = null;
  if (id) app.ui.selectedAgent.p1 = id;
  else app.openBoard('p1');
}

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [agent({ id: 'a7', name: 'esc-412' }), agent({ id: 'a8', name: 'esc-413' })] });
  app.ui.panes = {};
  devpages.pages = {};
  devpages.asks = [];
});

describe('the band « veut ouvrir »', () => {
  it('says which agent wants to open which host, outside the launch, and how many more wait', () => {
    render(OpenAskBand, { asks: [ASK, { ...ASK, id: 'o-2', origin: 'https://example.com' }], onanswer: () => {} });
    const band = screen.getByTestId('open-ask-band');
    expect(band).toHaveTextContent(
      'esc-412 veut ouvrir docs.stripe.com (hors du lancement du projet, dans un navigateur d’Escouade) et 1 autre demande',
    );
    expect(within(band).getByRole('button', { name: 'Permettre une fois' })).toBeInTheDocument();
    expect(within(band).getByRole('button', { name: 'Refuser' })).toBeInTheDocument();
  });

  it('shows nothing without a request, and reads in English', () => {
    const { unmount } = render(OpenAskBand, { asks: [], onanswer: () => {} });
    expect(screen.queryByTestId('open-ask-band')).not.toBeInTheDocument();
    unmount();
    setLang('en');
    render(OpenAskBand, { asks: [ASK], onanswer: () => {} });
    expect(screen.getByTestId('open-ask-band')).toHaveTextContent('esc-412 wants to open docs.stripe.com');
    expect(screen.getByRole('button', { name: 'Allow once' })).toBeInTheDocument();
    setLang('fr');
  });

  it('comes when the backend says an agent asked, each agent its own, and the answer goes to the backend', async () => {
    let held: BrowserOpenAsk[] = [];
    const b = fakeBackend({
      devpage_open_asks: () => held,
      devpage_open_answer: () => {
        held = [];
        return held;
      },
    });
    held = [ASK];
    onPageEvent({ agentId: 'a7', phase: 'opens', url: '' });
    await waitFor(() => expect(openAsksOf('a7')).toEqual([ASK]));
    expect(openAsksOf('a8')).toEqual([]);
    render(OpenAskBand, { asks: openAsksOf('a7'), onanswer: answerOpen });
    await userEvent.click(screen.getByRole('button', { name: 'Permettre une fois' }));
    expect(b.called('devpage_open_answer').map((c) => c.args)).toEqual([{ id: 'o-1', allow: true }]);
    await waitFor(() => expect(devpages.asks).toEqual([]));
    await loadOpenAsks();
    expect(devpages.asks).toEqual([]);
  });
});

describe('whose conversation is on screen', () => {
  it('is told to the backend while its tools of the browsers answer, and never otherwise', async () => {
    const b = fakeBackend();
    app.settings = { ...app.settings, mcpEnabled: false };
    lookAt('a7');
    flushSync();
    await tick();
    expect(b.called('devpage_looking')).toEqual([]);
    app.settings = { ...app.settings, mcpEnabled: true, agentBrowsers: true };
    flushSync();
    await waitFor(() => expect(b.called('devpage_looking').at(-1)?.args).toEqual({ agents: ['a7'] }));
    lookAt(null);
    flushSync();
    await waitFor(() => expect(b.called('devpage_looking').at(-1)?.args).toEqual({ agents: [] }));
    app.settings = { ...app.settings, agentBrowsers: false };
    flushSync();
    const said = b.called('devpage_looking').length;
    lookAt('a7');
    flushSync();
    await tick();
    expect(b.called('devpage_looking')).toHaveLength(said);
    app.settings = { ...app.settings, mcpEnabled: false, agentBrowsers: true };
  });
});

describe('a browser an agent opened', () => {
  it('is a row of its project, shown beside the agent’s conversation while the user looks at it', async () => {
    fakeBackend();
    lookAt('a7');
    flushSync();
    onBrowserOpened(opened());
    await tick();
    const p = devpages.pages['browser:t1'] as PageState;
    expect(p).toMatchObject({ projectId: 'p1', opener: 'a7', driver: 'a7', by: { kind: 'user' } });
    expect(p.waiting).toBeUndefined();
    expect(inPane('browser:t1')).toBe(true);
    expect(app.shownAgents).toContain('a7');
  });

  it('waits in « Navigateurs » while the user looks elsewhere, and opens under the conversation once they look at it', async () => {
    fakeBackend();
    lookAt(null);
    flushSync();
    onBrowserOpened(opened({ wish: 'below', place: 'waiting' }));
    await tick();
    expect(devpages.pages['browser:t1'].waiting).toBe('below');
    expect(inPane('browser:t1')).toBe(false);
    render(BrowsersSection, { project: project() });
    const row = document.querySelector('.row-item.browser[data-browser="browser:t1"]')!;
    expect(row).toHaveClass('waits');
    expect(row).toHaveTextContent('esc-412 veut te montrer la page');
    lookAt('a7');
    flushSync();
    await waitFor(() => expect(devpages.pages['browser:t1'].waiting).toBeUndefined());
    expect(inPane('browser:t1')).toBe(true);
  });

  it('keeps one pane per agent: another browser it opens takes the place of the first', async () => {
    fakeBackend();
    lookAt('a7');
    flushSync();
    onBrowserOpened(opened());
    await tick();
    onBrowserOpened(opened({ view: view({ agentId: 'browser:t2', url: 'http://localhost:4100/b' }) }));
    await tick();
    expect(inPane('browser:t2')).toBe(true);
    expect(inPane('browser:t1')).toBe(false);
    expect(devpages.pages['browser:t1']).toBeTruthy();
  });

  it('quiet: its row alone; closed by its agent: gone; no longer driven: an ordinary row', async () => {
    fakeBackend();
    lookAt('a7');
    flushSync();
    onBrowserOpened(opened({ wish: 'quiet', place: 'quiet' }));
    await tick();
    expect(inPane('browser:t1')).toBe(false);
    expect(devpages.pages['browser:t1'].waiting).toBeUndefined();
    onBrowserOpened(opened({ view: view({ agentId: 'run:p1:web', opener: undefined }), wish: 'quiet', place: 'quiet' }));
    expect(devpages.pages['run:p1:web'].by).toEqual({ kind: 'command', id: 'web' });
    onPageEvent({ agentId: 'run:p1:web', phase: 'undriven', url: '' });
    expect(devpages.pages['run:p1:web'].driver).toBeUndefined();
    onPageEvent({ agentId: 'browser:t1', phase: 'closed', url: '' });
    expect(devpages.pages['browser:t1']).toBeUndefined();
  });
});
