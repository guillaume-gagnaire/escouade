// The page of a launch inside Escouade (ESC-311): where it opens by the project's setting, what its events change,
// when it is hidden under the window's overlays, where it goes in the window, its devtools' shortcut.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import {
  boundsOf,
  closePage,
  coveredAt,
  detach,
  devpages,
  devtoolsKey,
  onPageEvent,
  openLaunchPage,
  pagesOpenOf,
  pageTitle,
  showPage,
  STALLED_MS,
} from './devpage.svelte';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { Agent, TestRecipe } from './types';

const RECIPE: TestRecipe = {
  prepare: [],
  processes: [
    { name: 'api', command: 'node api.js', dir: '', env: {}, url: 'http://localhost:4110/health' },
    { name: 'web', command: 'node web.js', dir: '', env: {}, url: 'http://localhost:4111' },
  ],
  open: 'http://localhost:4111/users?search=marie@',
};
let A: Agent;

beforeEach(() => {
  setLang('fr');
  A = agent({ id: 'a7', name: 'pagination-users', recipe: RECIPE });
  resetApp({ agents: [A] });
  devpages.pages = {};
  devpages.shown = {};
  app.modal = null;
});
afterEach(() => vi.useRealTimers());

describe('where a launch’s page opens', () => {
  it('in the agent by default, for a project saved before the setting too', () => {
    expect(pagesOpenOf(project())).toBe('agent');
    expect(pagesOpenOf(project({ pagesOpen: 'window' }))).toBe('window');
    expect(pagesOpenOf(undefined)).toBe('agent');
  });

  it('opens in the agent’s view, its tab named after the worktree and the process that serves it', async () => {
    const b = fakeBackend();
    expect(await openLaunchPage(A, RECIPE.open)).toBe('agent');
    expect(b.called('devpage_open').map((c) => c.args)).toEqual([
      { agentId: 'a7', title: 'pagination-users · web', url: RECIPE.open, detached: false, bounds: null },
    ]);
    expect(b.called('open_url')).toHaveLength(0);
    expect(devpages.pages.a7).toMatchObject({ phase: 'loading', detached: false, url: RECIPE.open });
    expect(devpages.shown.a7).toBe(true);
  });

  it('takes the place of the test’s window, as the browser came to the front before', async () => {
    fakeBackend();
    app.modal = { kind: 'testLaunch', agentId: 'a7' };
    await openLaunchPage(A, RECIPE.open);
    expect(app.modal).toBeNull();
    expect(app.ui.selectedAgent.p1).toBe('a7');
    expect(app.ui.view).toBe('project');
  });

  it('leaves the user where they are when they look elsewhere', async () => {
    fakeBackend();
    app.agents.a8 = agent({ id: 'a8' });
    app.ui.selectedAgent.p1 = 'a8';
    await openLaunchPage(A, RECIPE.open);
    expect(app.ui.selectedAgent.p1).toBe('a8');
    // Its view « Page » waits for them.
    expect(devpages.shown.a7).toBe(true);
  });

  it('in the project’s « Pages » window when the project says so', async () => {
    app.projects[0].pagesOpen = 'window';
    const b = fakeBackend();
    expect(await openLaunchPage(A, RECIPE.open)).toBe('window');
    expect(b.called('devpage_open')[0].args.detached).toBe(true);
    expect(devpages.shown.a7).toBeUndefined();
  });

  it('in the system’s browser, as before, when the project says so', async () => {
    app.projects[0].pagesOpen = 'system';
    const b = fakeBackend();
    expect(await openLaunchPage(A, RECIPE.open)).toBe('system');
    expect(b.called('open_url').map((c) => c.args.url)).toEqual([RECIPE.open]);
    expect(b.called('devpage_open')).toHaveLength(0);
    expect(devpages.pages.a7).toBeUndefined();
  });

  it('names the tab after the agent alone when no process serves the address', () => {
    expect(pageTitle(A, 'http://localhost:9999/')).toBe('pagination-users');
    expect(pageTitle(A, 'pas une adresse')).toBe('pagination-users');
  });
});

describe('a page’s life', () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    fakeBackend();
    await openLaunchPage(A, RECIPE.open);
  });

  it('is loaded, then does not answer when a load lasts more than 20 s', () => {
    onPageEvent({ agentId: 'a7', phase: 'started', url: 'http://localhost:4111/users' });
    vi.advanceTimersByTime(STALLED_MS - 1);
    expect(devpages.pages.a7.phase).toBe('loading');
    onPageEvent({ agentId: 'a7', phase: 'finished', url: 'http://localhost:4111/users' });
    expect(devpages.pages.a7).toMatchObject({ phase: 'ready', url: 'http://localhost:4111/users' });
    onPageEvent({ agentId: 'a7', phase: 'started', url: 'http://localhost:4111/lent' });
    vi.advanceTimersByTime(STALLED_MS);
    expect(devpages.pages.a7.phase).toBe('stalled');
  });

  it('sends another origin to the system’s browser, and says so', () => {
    const b = fakeBackend();
    const toast = vi.spyOn(app, 'toast');
    onPageEvent({ agentId: 'a7', phase: 'outside', url: 'https://github.com/gagnaire/escouade' });
    expect(b.called('open_url').map((c) => c.args.url)).toEqual(['https://github.com/gagnaire/escouade']);
    expect(toast).toHaveBeenCalledWith('github.com s’est ouvert dans ton navigateur : ici, seul localhost:4111 s’affiche.', 'info');
  });

  it('switches between the conversation and the page, and goes to its window and back', async () => {
    showPage('a7', false);
    expect(devpages.shown.a7).toBe(false);
    showPage('a7', true);
    expect(devpages.shown.a7).toBe(true);
    await detach('a7', true);
    expect(devpages.pages.a7.detached).toBe(true);
    expect(devpages.shown.a7).toBe(false);
    await detach('a7', false);
    expect(devpages.pages.a7.detached).toBe(false);
    expect(devpages.shown.a7).toBe(true);
  });

  it('closes with its agent, and with its ticket done', () => {
    const b = fakeBackend();
    closePage('a7');
    expect(devpages.pages.a7).toBeUndefined();
    expect(devpages.shown.a7).toBeUndefined();
    expect(b.called('devpage_close').map((c) => c.args.agentId)).toEqual(['a7']);
    // A page no longer there: its view cannot show it.
    showPage('a7', true);
    expect(devpages.shown.a7).toBeUndefined();
  });
});

describe('the page in the window', () => {
  const rect = (left: number, top: number, width: number, height: number) =>
    ({ left, top, width, height, right: left + width, bottom: top + height }) as DOMRect;

  it('goes where its view is, in the window’s pixels at the interface’s zoom', () => {
    expect(boundsOf(rect(330, 112, 1100, 740), 100)).toEqual({ x: 330, y: 112, width: 1100, height: 740 });
    expect(boundsOf(rect(330, 112, 1100, 740), 125)).toEqual({ x: 413, y: 140, width: 1375, height: 925 });
  });

  it('is hidden under a menu, a list, a dialog or the palette that crosses it, not under one elsewhere', () => {
    document.body.innerHTML = '<div role="menu" id="m"></div><div role="dialog" id="d"></div><span role="tooltip"></span>';
    const place = (id: string, r: DOMRect) => ((document.getElementById(id) as HTMLElement).getBoundingClientRect = () => r);
    const page = rect(330, 112, 1100, 740);
    place('m', rect(10, 10, 200, 300));
    place('d', rect(0, 0, 0, 0));
    expect(coveredAt(page)).toBe(false);
    place('m', rect(300, 100, 200, 300));
    expect(coveredAt(page)).toBe(true);
    place('m', rect(10, 10, 200, 300));
    place('d', rect(500, 300, 400, 200));
    expect(coveredAt(page)).toBe(true);
    document.body.innerHTML = '';
  });
});

describe('the devtools’ shortcut', () => {
  const key = (init: KeyboardEventInit) => new KeyboardEvent('keydown', init);
  it('is F12 everywhere, Cmd+Opt+I on macOS, Ctrl+Shift+I on Windows and Linux', () => {
    expect(devtoolsKey(key({ key: 'F12' }), false)).toBe(true);
    expect(devtoolsKey(key({ key: 'F12' }), true)).toBe(true);
    expect(devtoolsKey(key({ key: 'i', code: 'KeyI', metaKey: true, altKey: true }), true)).toBe(true);
    expect(devtoolsKey(key({ key: 'I', code: 'KeyI', ctrlKey: true, shiftKey: true }), false)).toBe(true);
    // Option+I types a character on macOS: the code says the key.
    expect(devtoolsKey(key({ key: 'ˆ', code: 'KeyI', metaKey: true, altKey: true }), true)).toBe(true);
    expect(devtoolsKey(key({ key: 'i', ctrlKey: true }), false)).toBe(false);
    expect(devtoolsKey(key({ key: 'i', ctrlKey: true, shiftKey: true }), true)).toBe(false);
    expect(devtoolsKey(key({ key: 'F11' }), false)).toBe(false);
  });
});
