// The browsers of a project (ESC-657; the page of a launch before, ESC-311): where a launch's page opens by the project's
// setting, a browser opened alone by « + », what their events change, when they are hidden under the window's overlays,
// where they go in the window, the devtools' shortcut.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import {
  boundsOf,
  browserAddress,
  browsersOf,
  closePage,
  coveredAt,
  devpages,
  devtoolsKey,
  onPageEvent,
  newBrowser,
  openLaunchPage,
  pagesOpenOf,
  pageTitle,
  rowName,
  showPage,
  STALLED_MS,
} from './devpage.svelte';
import { deviceFit, loadModes, MODES_KEY, modeKey, nextMode } from './devpage-key';
import { cycleMode, modeOf, setMode } from './devpage.svelte';
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
  app.modal = null;
});
afterEach(() => vi.useRealTimers());

describe('where a launch’s page opens', () => {
  it('in the agent by default, for a project saved before the setting too', () => {
    expect(pagesOpenOf(project())).toBe('agent');
    // « Dans une fenêtre à part » of before: the « Pages » window is gone, the browser's row opens (ESC-657).
    expect(pagesOpenOf(project({ pagesOpen: 'window' }))).toBe('agent');
    expect(pagesOpenOf(project({ pagesOpen: 'system' }))).toBe('system');
    expect(pagesOpenOf(undefined)).toBe('agent');
  });

  it('opens in the agent’s view, its tab named after the worktree and the process that serves it', async () => {
    const b = fakeBackend();
    expect(await openLaunchPage(A, RECIPE.open)).toBe('agent');
    expect(b.called('devpage_open').map((c) => c.args)).toEqual([
      { agentId: 'a7', title: 'pagination-users · web', url: RECIPE.open, bounds: null },
    ]);
    expect(b.called('open_url')).toHaveLength(0);
    expect(devpages.pages.a7).toMatchObject({
      phase: 'loading',
      url: RECIPE.open,
      projectId: 'p1',
      by: { kind: 'agent', id: 'a7' },
    });
    // Its browser's row, named after the worktree and the process (ESC-657).
    expect(browsersOf('p1').map(([key, p]) => [key, rowName(p)])).toEqual([['a7', 'pagination-users · web']]);
  });

  it('takes the place of the test’s window, as the browser came to the front before', async () => {
    fakeBackend();
    app.modal = { kind: 'testLaunch', agentId: 'a7' };
    await openLaunchPage(A, RECIPE.open);
    expect(app.modal).toBeNull();
    expect(app.ui.view).toBe('project');
    // Its browser's row is shown, as a terminal just opened is (ESC-657).
    expect(app.selectedPage.p1).toBe('a7');
    expect(app.pageShown).toBe('a7');
  });

  it('shows its row when the user was looking at the agent', async () => {
    fakeBackend();
    app.ui.selectedAgent.p1 = 'a7';
    await openLaunchPage(A, RECIPE.open);
    expect(app.pageShown).toBe('a7');
  });

  it('leaves the user where they are when they look elsewhere', async () => {
    fakeBackend();
    app.agents.a8 = agent({ id: 'a8' });
    app.ui.selectedAgent.p1 = 'a8';
    await openLaunchPage(A, RECIPE.open);
    expect(app.ui.selectedAgent.p1).toBe('a8');
    // Its row waits for them.
    expect(app.pageShown).toBeNull();
    expect(browsersOf('p1')).toHaveLength(1);
  });

  it('in its row when the project said « Dans une fenêtre à part » before: the « Pages » window is gone', async () => {
    app.projects[0].pagesOpen = 'window';
    const b = fakeBackend();
    expect(await openLaunchPage(A, RECIPE.open)).toBe('agent');
    expect(b.called('devpage_open')[0].args).not.toHaveProperty('detached');
    expect(browsersOf('p1').map(([k]) => k)).toEqual(['a7']);
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

  it('is shown by its row', async () => {
    app.selectAgent('a7');
    showPage('a7');
    expect(app.pageShown).toBe('a7');
    // Its agent is no longer the one on screen: the browser is, as a terminal would be.
    expect(app.agentShown).toBeNull();
  });

  it('closes with its agent, and with its ticket done', () => {
    const b = fakeBackend();
    showPage('a7');
    closePage('a7');
    expect(devpages.pages.a7).toBeUndefined();
    expect(app.selectedPage.p1).toBeNull();
    expect(b.called('devpage_close').map((c) => c.args.agentId)).toEqual(['a7']);
    // A page no longer there: nothing shows it.
    showPage('a7');
    expect(app.pageShown).toBeNull();
  });
});

describe('a browser opened alone (ESC-657)', () => {
  it('takes an http or https address, http:// when it has none, as the launches’ pages do', () => {
    expect(browserAddress('localhost:3000')).toEqual({ url: 'http://localhost:3000/' });
    expect(browserAddress(' localhost:3000/users?q=1 ')).toEqual({ url: 'http://localhost:3000/users?q=1' });
    expect(browserAddress('https://app.localhost:8443/a')).toEqual({ url: 'https://app.localhost:8443/a' });
    expect(browserAddress('example.org')).toEqual({ url: 'http://example.org/' });
    for (const typed of ['javascript:alert(1)', 'file:///etc/passwd', 'mailto:a@b.c', 'data:text/html,x', 'tauri://localhost/'])
      expect(browserAddress(typed), typed).toEqual({
        error: 'Seule une adresse http:// ou https:// s’ouvre dans un navigateur d’Escouade.',
      });
    expect(browserAddress('  ')).toEqual({ error: 'Tape une adresse.' });
    expect(browserAddress('http://')).toEqual({ error: 'Ce n’est pas une adresse (localhost:3000, par exemple).' });
  });

  it('opens a row of its own in the project, keyed apart from the agents, and shows it', async () => {
    const b = fakeBackend();
    const key = await newBrowser('p1', 'localhost:3000/admin');
    expect(key).toMatch(/^browser:[a-z0-9]+$/);
    expect(b.called('devpage_open').map((c) => c.args)).toEqual([
      { agentId: key, title: 'localhost:3000', url: 'http://localhost:3000/admin', bounds: null, projectId: 'p1' },
    ]);
    expect(devpages.pages[key!]).toMatchObject({ projectId: 'p1', by: { kind: 'user' } });
    expect(app.pageShown).toBe(key);
    // Its row says its host until the page gives its title.
    expect(rowName(devpages.pages[key!])).toBe('localhost:3000');
    onPageEvent({ agentId: key!, phase: 'title', url: 'http://localhost:3000/admin', title: 'Admin — demo' });
    expect(rowName(devpages.pages[key!])).toBe('Admin — demo');
  });

  it('does not answer when its first load lasts more than 20 s (its first « started » came before its row)', async () => {
    vi.useFakeTimers();
    fakeBackend();
    const key = (await newBrowser('p1', 'localhost:3000/lent'))!;
    vi.advanceTimersByTime(STALLED_MS - 1);
    expect(devpages.pages[key].phase).toBe('loading');
    vi.advanceTimersByTime(1);
    expect(devpages.pages[key].phase).toBe('stalled');
  });

  it('takes what the page already is at its opening: its events may have come before its row', async () => {
    fakeBackend({
      devpage_open: (x) => ({
        agentId: x.agentId,
        projectId: 'p1',
        title: x.title,
        url: x.url,
        loaded: true,
        docTitle: 'Tableau — demo',
      }),
    });
    const key = (await newBrowser('p1', 'localhost:3000'))!;
    expect(devpages.pages[key]).toMatchObject({ phase: 'ready', docTitle: 'Tableau — demo' });
    expect(rowName(devpages.pages[key])).toBe('Tableau — demo');
  });

  it('keeps what its page said before its row existed, and gives it to the row', async () => {
    let key = '';
    fakeBackend({
      devpage_open: (x) => {
        key = x.agentId;
        // The page loads and names itself while the window waits for the answer.
        onPageEvent({ agentId: key, phase: 'finished', url: x.url });
        onPageEvent({ agentId: key, phase: 'title', url: x.url, title: 'Boutique — demo' });
        // Something that opens or refuses is never done later.
        onPageEvent({ agentId: key, phase: 'outside', url: 'https://ailleurs.example/' });
        return { agentId: key, projectId: 'p1', title: x.title, url: x.url };
      },
    });
    expect(await newBrowser('p1', 'localhost:3000')).toBe(key);
    expect(devpages.pages[key]).toMatchObject({ phase: 'ready', docTitle: 'Boutique — demo' });
  });

  it('lists the rows of the project alone, the oldest first', async () => {
    fakeBackend();
    const one = await newBrowser('p1', 'localhost:3001');
    await openLaunchPage(A, RECIPE.open);
    const two = await newBrowser('p1', 'localhost:3002');
    expect(browsersOf('p1').map(([k]) => k)).toEqual([one, 'a7', two]);
    expect(browsersOf('p2')).toEqual([]);
  });

  it('opens nothing for an address it refuses, and says why', async () => {
    const b = fakeBackend();
    const toast = vi.spyOn(app, 'toast');
    expect(await newBrowser('p1', 'javascript:alert(1)')).toBeNull();
    expect(b.called('devpage_open')).toHaveLength(0);
    expect(toast).toHaveBeenCalledWith('Seule une adresse http:// ou https:// s’ouvre dans un navigateur d’Escouade.', 'error');
  });

  it('closes by its row, its pane with it', async () => {
    const b = fakeBackend();
    const key = (await newBrowser('p1', 'localhost:3000'))!;
    closePage(key);
    expect(devpages.pages[key]).toBeUndefined();
    expect(app.pageShown).toBeNull();
    expect(b.called('devpage_close').map((c) => c.args.agentId)).toEqual([key]);
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

describe('the widths of a page: full, tablet, mobile', () => {
  beforeEach(() => {
    localStorage.removeItem(MODES_KEY);
    devpages.modes = {};
  });

  it('centres a tablet (768) or a phone (390) in the room, narrowed to a narrower room', () => {
    expect(deviceFit('full', 1100)).toEqual({ width: 1100, left: 0, shrunk: false });
    expect(deviceFit('tablet', 1100)).toEqual({ width: 768, left: 166, shrunk: false });
    expect(deviceFit('mobile', 1100)).toEqual({ width: 390, left: 355, shrunk: false });
    expect(deviceFit('tablet', 600)).toEqual({ width: 600, left: 0, shrunk: true });
  });

  it('goes round them by Ctrl+Shift+M (Cmd+Shift+M on macOS)', () => {
    expect(nextMode('full')).toBe('tablet');
    expect(nextMode('tablet')).toBe('mobile');
    expect(nextMode('mobile')).toBe('full');
    const key = (init: KeyboardEventInit) => new KeyboardEvent('keydown', init);
    expect(modeKey(key({ key: 'M', code: 'KeyM', ctrlKey: true, shiftKey: true }), false)).toBe(true);
    expect(modeKey(key({ key: 'M', code: 'KeyM', metaKey: true, shiftKey: true }), true)).toBe(true);
    expect(modeKey(key({ key: 'm', ctrlKey: true }), false)).toBe(false);
    expect(modeKey(key({ key: 'M', ctrlKey: true, shiftKey: true }), true)).toBe(false);
  });

  it('keeps the width chosen for the page (worktree · process), from one launch to the next', async () => {
    let b = fakeBackend();
    await openLaunchPage(A, RECIPE.open);
    expect(modeOf('a7')).toBe('full');
    setMode('a7', 'mobile');
    expect(b.called('devpage_mode').map((c) => c.args)).toEqual([{ agentId: 'a7', mode: 'mobile' }]);
    expect(loadModes()).toEqual({ 'pagination-users · web': 'mobile' });
    cycleMode('a7');
    expect(modeOf('a7')).toBe('full');
    expect(loadModes()).toEqual({});
    setMode('a7', 'tablet');
    // Launched again (its page closed, the window started anew): the same width, laid out by it.
    closePage('a7');
    devpages.modes = loadModes();
    b = fakeBackend();
    await openLaunchPage(A, RECIPE.open);
    expect(modeOf('a7')).toBe('tablet');
    expect(b.called('devpage_mode').map((c) => c.args)).toEqual([{ agentId: 'a7', mode: 'tablet' }]);
  });

  it('goes to the next width by the shortcut pressed in the page', async () => {
    fakeBackend();
    await openLaunchPage(A, RECIPE.open);
    onPageEvent({ agentId: 'a7', phase: 'responsive', url: '' });
    expect(modeOf('a7')).toBe('tablet');
  });

  it('shows the page at the width of an agent’s capture for its time, then gives it back its own, never kept (ESC-670)', async () => {
    const b = fakeBackend();
    await openLaunchPage(A, RECIPE.open);
    setMode('a7', 'tablet');
    onPageEvent({ agentId: 'a7', phase: 'width', url: 'mobile' });
    expect(modeOf('a7')).toBe('mobile');
    expect(loadModes()).toEqual({ 'pagination-users · web': 'tablet' });
    onPageEvent({ agentId: 'a7', phase: 'width', url: '' });
    expect(modeOf('a7')).toBe('tablet');
    expect(b.called('devpage_mode').map((c) => c.args.mode)).toEqual(['tablet', 'mobile', 'tablet']);
    // A width it does not know gives it back its own.
    onPageEvent({ agentId: 'a7', phase: 'width', url: 'watch' });
    expect(modeOf('a7')).toBe('tablet');
  });

  it('keeps the width an agent drives it to, the bar following; a driving change moves nothing (ESC-679)', async () => {
    const b = fakeBackend();
    await openLaunchPage(A, RECIPE.open);
    onPageEvent({ agentId: 'a7', phase: 'mode', url: 'mobile' });
    expect(modeOf('a7')).toBe('mobile');
    expect(loadModes()).toEqual({ 'pagination-users · web': 'mobile' });
    expect(b.called('devpage_mode').map((c) => c.args.mode)).toEqual(['mobile']);
    // A width it does not know changes nothing; nor does a gesture's news.
    onPageEvent({ agentId: 'a7', phase: 'mode', url: 'watch' });
    onPageEvent({ agentId: 'a7', phase: 'drive', url: '' });
    expect(modeOf('a7')).toBe('mobile');
    onPageEvent({ agentId: 'a7', phase: 'mode', url: 'full' });
    expect(modeOf('a7')).toBe('full');
    expect(loadModes()).toEqual({});
  });
});
