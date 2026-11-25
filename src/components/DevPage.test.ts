// The view of a browser (ESC-657; the agent's view « Page » before, ESC-311): its bar, what its buttons and its shortcut
// ask the backend, its states said in French and in English, and the page hidden when it cannot show; a browser opened
// alone, without an agent; where the devtools go on this system.
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { closePage, devpages, inPane, newBrowser, openPageBeside, showPage } from '../lib/devpage.svelte';
import { menu } from '../lib/menu.svelte';
import { MODES_KEY } from '../lib/devpage-key';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { flows } from '../lib/test-launch.svelte';
import type { Agent, LaunchState } from '../lib/types';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import DevPage from './DevPage.svelte';
import Pane from './panes/Pane.svelte';

let A: Agent;
const URL1 = 'http://localhost:41731/users?search=marie@';

/** Its launch: a process running (`up`), or ended. */
function launch(up: boolean) {
  app.launches['test:a7:run:0'] = {
    status: up ? 'running' : 'crashed',
    ptyId: up ? 't1' : null,
    name: 'web',
    stopping: false,
    code: up ? null : 1,
    startedAt: 0,
  } as LaunchState;
  flows.all.a7 = {
    lines: [{ id: 'test:a7:run:0', label: 'web', state: up ? 'ready' : 'failed', detail: '', launchId: 'test:a7:run:0' }],
    phase: up ? 'ready' : 'failed',
    error: null,
    opened: URL1,
    where: 'agent',
  };
}

beforeEach(() => {
  setLang('fr');
  A = agent({ id: 'a7', name: 'pagination-users' });
  resetApp({ agents: [A] });
  app.launches = {};
  flows.all = {};
  devpages.pages = {
    a7: {
      url: URL1,
      title: 'pagination-users · web',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'agent', id: 'a7' },
      opened: 1,
    },
  };
});

describe('the bar of a launch’s page', () => {
  it('has back, forward, reload, its address, whose page it is, devtools and the system’s browser; no « Pages » window', () => {
    launch(true);
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    for (const name of ['Retour', 'Avancer', 'Recharger', 'Ouvrir dans le navigateur du système'])
      expect(screen.getByRole('button', { name })).toBeTruthy();
    // The « Pages » window is gone (ESC-657).
    expect(screen.queryByRole('button', { name: 'Ouvrir dans une fenêtre à part' })).toBeNull();
    expect(screen.getByRole('textbox', { name: 'Adresse de la page' })).toHaveProperty('value', URL1);
    expect(screen.getByText('pagination-users · web')).toBeTruthy();
    expect(screen.getByRole('button', { name: /Outils de développement de cette page \(F12\)/ })).toBeTruthy();
  });

  it('asks the backend for each action, on this agent’s page', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    await fireEvent.click(screen.getByRole('button', { name: 'Retour' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Avancer' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Recharger' }));
    await fireEvent.click(screen.getByRole('button', { name: /Outils de développement/ }));
    await fireEvent.click(screen.getByRole('button', { name: 'Ouvrir dans le navigateur du système' }));
    expect(b.called('devpage_go').map((c) => [c.args.agentId, c.args.action])).toEqual([
      ['a7', 'back'],
      ['a7', 'forward'],
      ['a7', 'reload'],
    ]);
    expect(b.called('devpage_devtools').map((c) => c.args.agentId)).toEqual(['a7']);
    expect(b.called('open_url').map((c) => c.args.url)).toEqual([URL1]);
  });

  it('goes to an address typed, its scheme taken from the page when left out', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    const field = screen.getByRole('textbox', { name: 'Adresse de la page' });
    await fireEvent.input(field, { target: { value: 'localhost:41731/factures' } });
    await fireEvent.submit(field.closest('form')!);
    expect(b.called('devpage_go').map((c) => c.args)).toEqual([{ agentId: 'a7', action: 'to', to: 'http://localhost:41731/factures' }]);
  });

  it('opens the devtools by F12 and Ctrl+Shift+I', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    await fireEvent.keyDown(window, { key: 'F12' });
    await fireEvent.keyDown(window, { key: 'I', code: 'KeyI', ctrlKey: true, shiftKey: true });
    expect(b.called('devpage_devtools')).toHaveLength(2);
  });
});

describe('the bar, drawn with the app’s controls (ESC-419)', () => {
  it('uses the icon buttons, buttons, segments and field of the rest of the app', () => {
    launch(true);
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    for (const name of ['Retour', 'Avancer', 'Recharger']) expect(screen.getByRole('button', { name }).classList).toContain('icon-btn');
    for (const name of ['Ouvrir dans le navigateur du système', 'Plus d’actions sur la page'])
      expect([...screen.getByRole('button', { name }).classList]).toEqual(expect.arrayContaining(['btn', 'icon-only']));
    expect(screen.getByRole('button', { name: /Outils de développement/ }).classList).toContain('btn');
    expect(screen.getByRole('radiogroup', { name: 'Largeur de la page' }).classList).toContain('segmented');
    expect(screen.getByRole('radio', { name: 'Pleine largeur' }).classList).toContain('on');
    expect(screen.getByRole('textbox', { name: 'Adresse de la page' }).closest('form')?.classList).toContain('field');
  });
});

describe('in a pane of the split screen (ESC-419)', () => {
  beforeEach(() => {
    app.ui.selectedAgent = { p1: 'a7' };
    app.ui.panes = {};
  });

  it('goes beside its conversation from « … », which shows its messages again; its pane closes back to one', async () => {
    launch(true);
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    await fireEvent.click(screen.getByRole('button', { name: 'Plus d’actions sur la page' }));
    menu.open?.items.find((i) => i.label === 'Ouvrir à côté')?.onClick?.();
    expect(app.panes?.root).toEqual({
      dir: 'row',
      ratio: 0.5,
      a: { leaf: { kind: 'agent', id: 'a7' } },
      b: { leaf: { kind: 'page', id: 'a7' } },
    });
    expect(app.panes?.active).toBe('page:a7');
    expect(inPane('a7')).toBe(true);
    // Its row asked again: its pane becomes the active one, nothing else moves.
    app.activatePane('agent:a7');
    showPage('a7');
    expect(app.panes?.active).toBe('page:a7');
    expect(app.pageShown).toBe('a7');
    // Its pane closed: its conversation alone.
    app.closePaneOf('page:a7');
    expect(app.panes).toBeNull();
    expect(app.agent?.id).toBe('a7');
  });

  it('goes below; dropped in the middle of its conversation, the two swap; alone, it fills the main area', () => {
    fakeBackend();
    openPageBeside('a7', 'bottom');
    expect(app.panes?.root).toMatchObject({ dir: 'col', b: { leaf: { kind: 'page', id: 'a7' } } });
    app.dropInPane({ kind: 'agent', id: 'a7' }, 'center', { kind: 'page', id: 'a7' });
    expect(app.panes?.root).toMatchObject({ a: { leaf: { kind: 'page', id: 'a7' } }, b: { leaf: { kind: 'agent', id: 'a7' } } });
    app.closePaneOf('page:a7');
    expect(app.panes).toBeNull();
    // Its conversation alone: the page dropped in its place is shown alone, as a terminal would be.
    app.dropInPane({ kind: 'agent', id: 'a7' }, 'center', { kind: 'page', id: 'a7' });
    expect(app.panes).toBeNull();
    expect(app.pageShown).toBe('a7');
  });

  it('leaves its pane when it closes', async () => {
    fakeBackend();
    openPageBeside('a7', 'right');
    expect(app.panes).not.toBeNull();
    closePage('a7');
    expect(app.ui.panes?.p1).toBeUndefined();
  });

  it('names its pane, closes it from « … », and gives the keys to the active pane only', async () => {
    launch(true);
    const b = fakeBackend();
    openPageBeside('a7', 'right');
    render(Pane, { item: { kind: 'page', id: 'a7' }, project: app.projects[0] });
    expect(screen.getByRole('region', { name: 'Volet : Page de pagination-users · web' })).toBeTruthy();
    // « … » there closes its pane rather than opening it again.
    await fireEvent.click(screen.getByRole('button', { name: 'Plus d’actions sur la page' }));
    expect(menu.open?.items.map((i) => i.label)).toEqual(['Fermer le volet', '', 'Copier l’adresse']);
    await fireEvent.keyDown(window, { key: 'F12' });
    expect(b.called('devpage_devtools')).toHaveLength(1);
    app.activatePane('agent:a7');
    await tick();
    await fireEvent.keyDown(window, { key: 'F12' });
    expect(b.called('devpage_devtools')).toHaveLength(1);
  });
});

describe('its states', () => {
  it('says the page does not answer, with reload and the logs', async () => {
    launch(true);
    devpages.pages.a7.phase = 'stalled';
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    expect(screen.getByText('localhost:41731 ne répond pas')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Recharger' })).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Voir les logs' })).toBeTruthy();
  });

  it('says the launch stopped, with « Relancer », and hides the page', async () => {
    launch(false);
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    expect(screen.getByText('Le lancement s’est arrêté')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeTruthy();
    await tick();
    await new Promise((r) => requestAnimationFrame(r));
    expect(b.called('devpage_place').at(-1)?.args).toEqual({ agentId: 'a7', bounds: null });
  });

  it('says the launch is starting again', () => {
    launch(true);
    flows.all.a7.phase = 'running';
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    expect(screen.getByText('Le lancement démarre…')).toBeTruthy();
  });

  it('in English too', () => {
    setLang('en');
    launch(false);
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    expect(screen.getByText('The launch stopped')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Launch again' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Open in the system browser' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Developer tools of this page/ })).toBeTruthy();
  });
});

describe('its widths', () => {
  /** The room of the page measures `width` × 700 at (330, 112). */
  function room(width: number) {
    const el = document.querySelector('.room') as HTMLElement;
    el.getBoundingClientRect = () => ({ left: 330, top: 112, width, height: 700, right: 330 + width, bottom: 812 }) as DOMRect;
  }
  const frame = () => new Promise((r) => requestAnimationFrame(r));

  beforeEach(() => {
    localStorage.removeItem(MODES_KEY);
    devpages.modes = {};
  });

  it('has a selector of three, full width at first, that places the page at a tablet’s width, centred', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    room(1100);
    const widths = screen.getByRole('radiogroup', { name: 'Largeur de la page' });
    expect(screen.getByRole('radio', { name: 'Pleine largeur' })).toHaveAttribute('aria-checked', 'true');
    expect(widths.querySelectorAll('[role="radio"]')).toHaveLength(3);
    await fireEvent.click(screen.getByRole('radio', { name: 'Tablette' }));
    await tick();
    await frame();
    expect(screen.getByRole('radio', { name: 'Tablette' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByText('768 px')).toBeTruthy();
    expect(b.called('devpage_mode').at(-1)?.args).toEqual({ agentId: 'a7', mode: 'tablet' });
    expect(b.called('devpage_place').at(-1)?.args.bounds).toEqual({ x: 496, y: 112, width: 768, height: 700 });
  });

  it('narrows a phone to a narrower room, and says so', async () => {
    launch(true);
    devpages.modes['pagination-users · web'] = 'mobile';
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    room(320);
    window.dispatchEvent(new Event('resize'));
    await frame();
    await tick();
    expect(screen.getByText('Rétréci à 320 px')).toBeTruthy();
    expect(b.called('devpage_place').at(-1)?.args.bounds).toEqual({ x: 330, y: 112, width: 320, height: 700 });
  });

  it('goes round the widths by Ctrl+Shift+M', async () => {
    launch(true);
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    await fireEvent.keyDown(window, { key: 'M', code: 'KeyM', ctrlKey: true, shiftKey: true });
    expect(screen.getByRole('radio', { name: 'Tablette' })).toHaveAttribute('aria-checked', 'true');
    await fireEvent.keyDown(window, { key: 'M', code: 'KeyM', ctrlKey: true, shiftKey: true });
    expect(screen.getByRole('radio', { name: 'Mobile' })).toHaveAttribute('aria-checked', 'true');
  });

  it('in « … » when narrow, with the other secondary buttons', async () => {
    launch(true);
    const b = fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    // What the bar's container query folds when narrow (jsdom draws no container query).
    for (const sel of ['.fwd', '.modes', '.system']) (document.querySelector(sel) as HTMLElement).style.display = 'none';
    await fireEvent.click(screen.getByRole('button', { name: 'Plus d’actions sur la page' }));
    const labels = menu.open?.items.filter((i) => !i.separator).map((i) => i.label);
    expect(labels).toEqual([
      'Avancer',
      'Pleine largeur',
      'Tablette',
      'Mobile',
      'Ouvrir à côté',
      'Ouvrir en dessous',
      'Copier l’adresse',
      'Ouvrir dans le navigateur du système',
    ]);
    expect(menu.open?.items.find((i) => i.label === 'Pleine largeur')?.checked).toBe(true);
    menu.open?.items.find((i) => i.label === 'Mobile')?.onClick?.();
    expect(b.called('devpage_mode').at(-1)?.args).toEqual({ agentId: 'a7', mode: 'mobile' });
  });

  it('in « … » when wide: only what the bar does not show', async () => {
    launch(true);
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    await fireEvent.click(screen.getByRole('button', { name: 'Plus d’actions sur la page' }));
    expect(menu.open?.items.map((i) => i.label)).toEqual(['Ouvrir à côté', 'Ouvrir en dessous', '', 'Copier l’adresse']);
  });

  it('in English too', () => {
    setLang('en');
    launch(true);
    devpages.modes['pagination-users · web'] = 'tablet';
    fakeBackend();
    render(DevPage, { pageKey: 'a7' });
    expect(screen.getByRole('radiogroup', { name: 'Page width' })).toBeTruthy();
    for (const name of ['Full width', 'Tablet', 'Mobile']) expect(screen.getByRole('radio', { name })).toBeTruthy();
  });
});

describe('a browser opened alone (ESC-657)', () => {
  const B = 'browser:k1';
  beforeEach(() => {
    devpages.pages[B] = {
      url: 'http://localhost:3000/admin',
      title: 'localhost:3000',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'user' },
      opened: 2,
    };
  });

  it('has the same bar, without an agent’s chip or its launch’s states', () => {
    fakeBackend();
    render(DevPage, { pageKey: B });
    for (const name of ['Retour', 'Avancer', 'Recharger', 'Ouvrir dans le navigateur du système'])
      expect(screen.getByRole('button', { name })).toBeTruthy();
    expect(screen.getByRole('textbox', { name: 'Adresse de la page' })).toHaveProperty('value', 'http://localhost:3000/admin');
    expect(screen.getByRole('radiogroup', { name: 'Largeur de la page' })).toBeTruthy();
    expect(screen.queryByText('localhost:3000')).toBeNull();
    // No launch to wait for: never « Le lancement s’est arrêté ».
    expect(screen.queryByText('Le lancement s’est arrêté')).toBeNull();
  });

  it('puts the widths and the devtools beside the address, then the system’s browser', () => {
    fakeBackend();
    render(DevPage, { pageKey: B });
    const bar = document.querySelector('.bar')!;
    const order = [...bar.querySelectorAll('input, [role="radiogroup"], button')].map(
      (el) => el.getAttribute('aria-label') ?? el.getAttribute('role'),
    );
    const at = (name: string) => order.findIndex((x) => x?.startsWith(name));
    expect(at('Adresse de la page')).toBeLessThan(at('Largeur de la page'));
    expect(at('Largeur de la page')).toBeLessThan(at('Outils de développement'));
    expect(at('Outils de développement')).toBeLessThan(at('Ouvrir dans le navigateur du système'));
  });

  it('says where its devtools open on this system, and opens them on the user’s gesture alone', async () => {
    const b = fakeBackend();
    render(DevPage, { pageKey: B });
    const button = screen.getByRole('button', { name: /Outils de développement de cette page/ });
    const where = document.getElementById(button.getAttribute('aria-describedby')!)!;
    expect(where.textContent).toMatch(/^Dans (leur propre fenêtre|la page, en bas)/);
    expect(b.called('devpage_devtools')).toHaveLength(0);
    await fireEvent.click(button);
    expect(b.called('devpage_devtools').map((c) => c.args.agentId)).toEqual([B]);
  });

  it('asks the backend for its own page, and goes beside what is on screen', async () => {
    const b = fakeBackend();
    app.ui.panes = {};
    app.ui.selectedAgent = { p1: 'a7' };
    render(DevPage, { pageKey: B });
    await fireEvent.click(screen.getByRole('button', { name: 'Recharger' }));
    expect(b.called('devpage_go').map((c) => c.args)).toEqual([{ agentId: B, action: 'reload', to: null }]);
    openPageBeside(B, 'right');
    expect(app.panes?.root).toMatchObject({ a: { leaf: { kind: 'agent', id: 'a7' } }, b: { leaf: { kind: 'page', id: B } } });
  });

  it('shown alone, offers nothing to go beside in « … »; an agent’s page goes beside its conversation', async () => {
    fakeBackend();
    app.ui.panes = {};
    app.selectPage(B);
    render(DevPage, { pageKey: B });
    await fireEvent.click(screen.getByRole('button', { name: 'Plus d’actions sur la page' }));
    expect(menu.open?.items.map((i) => i.label)).toEqual(['Copier l’adresse']);
    // The page of a launch shown alone: beside its agent's conversation.
    app.selectPage('a7');
    openPageBeside('a7', 'bottom');
    expect(app.panes?.root).toMatchObject({
      dir: 'col',
      a: { leaf: { kind: 'agent', id: 'a7' } },
      b: { leaf: { kind: 'page', id: 'a7' } },
    });
  });

  it('is made by « + » with its project, and shown alone', async () => {
    const b = fakeBackend();
    const key = await newBrowser('p1', 'localhost:5173');
    expect(b.called('devpage_open')[0].args).toMatchObject({ projectId: 'p1', url: 'http://localhost:5173/' });
    expect(app.pageShown).toBe(key);
    expect(app.singleItem).toEqual({ kind: 'page', id: key });
  });
});
