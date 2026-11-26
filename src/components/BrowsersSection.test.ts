// « Navigateurs » in the sidebar (ESC-657): one row per browser, on the model of « Terminaux »: its dot, its ×, its menu,
// « + » that asks an address, the row selected, shown alone or in a pane of the split screen.
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { devpages, type PageState } from '../lib/devpage.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import BrowsersSection from './BrowsersSection.svelte';

const page = (over: Partial<PageState>): PageState => ({
  url: 'http://localhost:3000/',
  title: 'localhost:3000',
  phase: 'ready',
  projectId: 'p1',
  by: { kind: 'user' },
  opened: 1,
  ...over,
});

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [agent({ id: 'a7', name: 'pagination-users' })] });
  app.ui.panes = {};
  devpages.pages = {
    'browser:b1': page({ docTitle: 'Admin — demo', opened: 1 }),
    a7: page({
      url: 'http://localhost:4111/users',
      title: 'pagination-users · web',
      phase: 'loading',
      by: { kind: 'agent', id: 'a7' },
      opened: 2,
    }),
    'browser:b2': page({ url: 'http://localhost:9000/', title: 'localhost:9000', phase: 'stalled', opened: 3 }),
    // Another project's: not a row here.
    'browser:b3': page({ projectId: 'p2', opened: 4 }),
  };
});

const rows = () => [...document.querySelectorAll<HTMLElement>('.row-item.browser')];

describe('the section « Navigateurs »', () => {
  it('has a row per browser of the project, the oldest first, named by its title, its agent or its host', () => {
    fakeBackend();
    render(BrowsersSection, { project: project() });
    const section = screen.getByRole('region', { name: 'Navigateurs' });
    expect(within(section).getByText('3')).toBeTruthy();
    expect(rows().map((r) => r.querySelector('.nm')?.textContent)).toEqual(['Admin — demo', 'pagination-users · web', 'localhost:9000']);
    // Its dot says its page's state, to the eye and to a screen reader.
    expect(
      rows().map((r) =>
        r
          .querySelector('.d')
          ?.className.replace(/svelte-\S+/g, '')
          .trim(),
      ),
    ).toEqual(['d on', 'd wait', 'd bad']);
    expect(rows().map((r) => r.querySelector('.sr')?.textContent)).toEqual(['chargée', 'charge', 'ne répond pas']);
  });

  it('says when there is none', () => {
    devpages.pages = {};
    fakeBackend();
    render(BrowsersSection, { project: project() });
    expect(screen.getByText('Aucun navigateur ouvert')).toBeTruthy();
  });

  it('shows a browser alone by its row, selected; Ctrl+click puts it beside what is on screen', async () => {
    fakeBackend();
    app.ui.selectedAgent = { p1: 'a7' };
    render(BrowsersSection, { project: project() });
    await fireEvent.click(rows()[0]);
    expect(app.pageShown).toBe('browser:b1');
    await tick();
    expect(rows()[0].classList).toContain('sel');
    expect(rows()[0].getAttribute('aria-current')).toBe('true');
    await fireEvent.click(rows()[2], { ctrlKey: true });
    expect(app.panes?.root).toMatchObject({
      dir: 'row',
      a: { leaf: { kind: 'page', id: 'browser:b1' } },
      b: { leaf: { kind: 'page', id: 'browser:b2' } },
    });
    expect(app.panes?.active).toBe('page:browser:b2');
    await tick();
    // The other one shown in a pane: ringed, not selected.
    expect(rows()[0].classList).toContain('inpane');
    expect(rows()[2].classList).toContain('sel');
  });

  it('offers beside, below, reload, the address, the system’s browser and close in its menu', async () => {
    const b = fakeBackend();
    render(BrowsersSection, { project: project() });
    await fireEvent.contextMenu(rows()[1]);
    expect(menu.open?.items.filter((i) => !i.separator).map((i) => i.label)).toEqual([
      'Ouvrir à côté',
      'Ouvrir en dessous',
      'Recharger',
      'Copier l’adresse',
      'Ouvrir dans le navigateur du système',
      'Fermer le navigateur',
    ]);
    menu.open?.items.find((i) => i.label === 'Recharger')?.onClick?.();
    menu.open?.items.find((i) => i.label === 'Ouvrir dans le navigateur du système')?.onClick?.();
    expect(b.called('devpage_go').map((c) => [c.args.agentId, c.args.action])).toEqual([['a7', 'reload']]);
    expect(b.called('open_url').map((c) => c.args.url)).toEqual(['http://localhost:4111/users']);
  });

  it('closes a browser by its ×', async () => {
    const b = fakeBackend();
    render(BrowsersSection, { project: project() });
    await fireEvent.click(within(rows()[2]).getByRole('button', { name: 'Fermer le navigateur' }));
    expect(devpages.pages['browser:b2']).toBeUndefined();
    expect(b.called('devpage_close').map((c) => c.args.agentId)).toEqual(['browser:b2']);
    await tick();
    expect(rows()).toHaveLength(2);
  });

  it('asks an address by « + », opens a browser there, and shows it', async () => {
    const b = fakeBackend();
    render(BrowsersSection, { project: project() });
    await fireEvent.click(screen.getByRole('button', { name: 'Nouveau navigateur' }));
    await tick();
    const field = screen.getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
    expect(document.activeElement).toBe(field);
    await fireEvent.input(field, { target: { value: 'localhost:5173/login' } });
    await fireEvent.submit(field.closest('form')!);
    await vi.waitFor(() => expect(b.called('devpage_open')).toHaveLength(1));
    expect(b.called('devpage_open')[0].args).toMatchObject({ url: 'http://localhost:5173/login', projectId: 'p1' });
    await vi.waitFor(() => expect(screen.queryByRole('textbox', { name: 'Adresse du nouveau navigateur' })).toBeNull());
    expect(app.pageShown).toMatch(/^browser:/);
    expect(rows()).toHaveLength(4);
  });

  it('keeps the field, and opens nothing, for an address it refuses; Échap leaves it', async () => {
    const b = fakeBackend();
    const toast = vi.spyOn(app, 'toast');
    render(BrowsersSection, { project: project() });
    await fireEvent.click(screen.getByRole('button', { name: 'Nouveau navigateur' }));
    await tick();
    const field = screen.getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
    await fireEvent.input(field, { target: { value: 'file:///etc/passwd' } });
    await fireEvent.submit(field.closest('form')!);
    await tick();
    expect(b.called('devpage_open')).toHaveLength(0);
    expect(toast).toHaveBeenCalledWith('Seule une adresse http:// ou https:// s’ouvre dans un navigateur d’Escouade.', 'error');
    expect(screen.getByRole('textbox', { name: 'Adresse du nouveau navigateur' })).toBeTruthy();
    await fireEvent.keyDown(field, { key: 'Escape' });
    await tick();
    expect(screen.queryByRole('textbox', { name: 'Adresse du nouveau navigateur' })).toBeNull();
  });

  it('in English too', () => {
    setLang('en');
    fakeBackend();
    render(BrowsersSection, { project: project() });
    expect(screen.getByRole('region', { name: 'Browsers' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'New browser' })).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Close the browser' })).toHaveLength(3);
  });
});

describe('an agent’s page in « Navigateurs » (AP-3)', () => {
  it('is a row with the page’s icon, its title, its last version; its point while it waits; its menu and its × close its row alone', async () => {
    const { agentpages } = await import('../lib/agentpages.svelte');
    agentpages.info = {
      'p-12': {
        page: 'p-12',
        projectId: 'p1',
        agentId: 'a7',
        agentName: 'pagination-users',
        ticketId: null,
        title: 'Revue avant fusion',
        template: 'review',
        versions: [
          { n: 1, at: 0 },
          { n: 2, at: 1 },
        ],
        shown: 2,
        open: true,
        quiet: false,
        loaded: true,
      },
    };
    devpages.pages = {
      'page:p-12': page({ url: '', title: 'Revue avant fusion', by: { kind: 'page', id: 'a7', page: 'p-12' }, unseen: true, opened: 5 }),
    };
    const b = fakeBackend();
    render(BrowsersSection, { project: project() });
    const r = rows()[0];
    expect(r.querySelector('.nm')?.textContent).toBe('Revue avant fusion');
    expect(r.querySelector('.d')).toBeNull();
    expect(r.querySelector('.pg.unseen')).toBeTruthy();
    expect(r.querySelector('[data-testid="page-row-version"]')?.textContent).toBe('v2');
    expect(r.textContent).toContain('Nouvelle page : elle attend que tu l’ouvres');
    await fireEvent.contextMenu(r);
    expect(menu.open?.items.filter((i) => !i.separator).map((i) => i.label)).toEqual([
      'Ouvrir à côté',
      'Ouvrir en dessous',
      'Fermer la page (elle reste gardée)',
    ]);
    await fireEvent.click(within(r).getByRole('button', { name: 'Fermer la page (elle reste gardée)' }));
    expect(b.called('agentpage_close_row')).toEqual([{ cmd: 'agentpage_close_row', args: { page: 'p-12' } }]);
    expect(b.called('devpage_close')).toEqual([]);
    await tick();
    expect(rows()).toEqual([]);
  });
});
