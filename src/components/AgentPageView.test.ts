// The view of an agent's page (AP-3, direction A): its bar (title, agent, ticket, template, « v2 ▾ » and its versions, the
// widths, « … »), the trust mention of the driving band, « La page veut ouvrir… », the version the user went back to; the
// page placed in its room by `agentpage_place`, in French and in English.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { agentpages } from '../lib/agentpages.svelte';
import { devpages } from '../lib/devpage.svelte';
import { drives } from '../lib/drives.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { AgentPageRow } from '../lib/types';
import { agent, fakeBackend, resetApp, ticket } from '../test/ipc';
import AgentPageView from './AgentPageView.svelte';

function row(over: Partial<AgentPageRow> = {}): AgentPageRow {
  return {
    page: 'p-12',
    projectId: 'p1',
    agentId: 'a7',
    agentName: 'pagination',
    ticketId: 't41',
    title: 'Revue avant fusion · POR-41',
    template: 'review',
    versions: [
      { n: 1, at: Date.now() - 3_600_000 },
      { n: 2, at: Date.now() },
    ],
    shown: 2,
    open: true,
    quiet: false,
    loaded: true,
    ...over,
  };
}

beforeEach(() => {
  setLang('fr');
  drives.rows = {};
  resetApp({ agents: [agent({ id: 'a7', name: 'pagination' })], tickets: [ticket({ id: 't41', key: 'POR-41' })] });
  agentpages.info = { 'p-12': row() };
  devpages.pages = {
    'page:p-12': {
      url: '',
      title: 'Revue avant fusion · POR-41',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'page', id: 'a7', page: 'p-12' },
      opened: 1,
    },
  };
});

describe('the view of an agent’s page', () => {
  it('names the page, its agent, its ticket and its template, its last version, and says who wrote it', () => {
    fakeBackend();
    render(AgentPageView, { pageKey: 'page:p-12' });
    expect(screen.getByRole('region', { name: 'Page de pagination : Revue avant fusion · POR-41' })).toBeTruthy();
    const bar = document.querySelector('.bar')!;
    expect(bar.textContent).toContain('Revue avant fusion · POR-41');
    expect(bar.textContent).toContain('pagination');
    expect(bar.textContent).toContain('POR-41');
    expect(bar.textContent).toContain('Revue');
    expect(screen.getByRole('button', { name: 'Versions de la page' }).textContent).toBe('v2');
    expect(document.querySelector('[data-testid="drive-band"]')?.textContent).toContain(
      'Écrite par pagination · hors réseau, sans accès à Escouade ni à tes fichiers',
    );
    // No address bar, no back nor reload: the page is a copy, not a site.
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Recharger' })).toBeNull();
  });

  it('lists its versions, the last first, and shows the one chosen', async () => {
    const b = fakeBackend({ agentpage_version: (a: { n: number }) => row({ shown: a.n }) });
    render(AgentPageView, { pageKey: 'page:p-12' });
    await fireEvent.click(screen.getByRole('button', { name: 'Versions de la page' }));
    const items = menu.open?.items ?? [];
    expect(items.map((i) => [i.label, i.checked])).toEqual([
      ['Version 2', true],
      ['Version 1', false],
    ]);
    expect(items[0].hint).toBe('la dernière');
    items[1].onClick?.();
    await vi.waitFor(() => expect(agentpages.info['p-12']?.shown).toBe(1));
    expect(b.called('agentpage_version')).toEqual([{ cmd: 'agentpage_version', args: { page: 'p-12', n: 1 } }]);
    // An older version: said, amber chip, and one click back to the last.
    expect(screen.getByRole('button', { name: 'Versions de la page' }).textContent).toBe('v1');
    expect(screen.getByRole('status').textContent).toContain('Tu regardes la v1 ; la dernière est la v2.');
    await fireEvent.click(screen.getByRole('button', { name: 'Revenir à la v2' }));
    expect(b.called('agentpage_version').at(-1)?.args).toEqual({ page: 'p-12', n: 2 });
  });

  it('is shown at a tablet’s or a phone’s width, in the window alone (no launch’s width kept)', async () => {
    const b = fakeBackend();
    render(AgentPageView, { pageKey: 'page:p-12' });
    await fireEvent.click(screen.getByRole('radio', { name: 'Mobile' }));
    expect(devpages.pages['page:p-12'].width).toBe('mobile');
    expect(screen.getByRole('radio', { name: 'Mobile' }).getAttribute('aria-checked')).toBe('true');
    expect(b.called('devpage_mode')).toEqual([]);
  });

  it('places its page by agentpage_place, and hides it when it goes', async () => {
    const b = fakeBackend();
    const view = render(AgentPageView, { pageKey: 'page:p-12' });
    await new Promise((r) => requestAnimationFrame(() => r(null)));
    await tick();
    view.unmount();
    expect(b.called('agentpage_place').at(-1)?.args).toMatchObject({ page: 'p-12', bounds: null, placement: 'hidden' });
    expect(b.called('devpage_place')).toEqual([]);
  });

  it('offers the web address the page asked for, opened by the user’s gesture alone', async () => {
    const b = fakeBackend();
    devpages.pages['page:p-12'].wants = 'https://example.com/doc';
    render(AgentPageView, { pageKey: 'page:p-12' });
    expect(screen.getByRole('alert').textContent).toContain('La page veut ouvrir https://example.com/doc');
    await fireEvent.click(screen.getByRole('button', { name: 'Ouvrir dans le navigateur du système' }));
    expect(b.called('agentpage_outside')).toEqual([{ cmd: 'agentpage_outside', args: { page: 'p-12', follow: true } }]);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('« … » puts it beside or below, and closes the page', async () => {
    const b = fakeBackend();
    render(AgentPageView, { pageKey: 'page:p-12' });
    await fireEvent.click(screen.getByRole('button', { name: 'Plus d’actions sur la page' }));
    const labels = (menu.open?.items ?? []).filter((i) => !i.separator).map((i) => i.label);
    expect(labels).toContain('Fermer la page');
    menu.open?.items.find((i) => i.label === 'Fermer la page')?.onClick?.();
    expect(b.called('agentpage_close_row')).toEqual([{ cmd: 'agentpage_close_row', args: { page: 'p-12' } }]);
    expect(devpages.pages['page:p-12']).toBeUndefined();
  });

  it('speaks English', () => {
    setLang('en');
    fakeBackend();
    render(AgentPageView, { pageKey: 'page:p-12' });
    expect(screen.getByRole('button', { name: 'Versions of the page' })).toBeTruthy();
    expect(document.querySelector('[data-testid="drive-band"]')?.textContent).toContain('Written by pagination');
    expect(document.querySelector('.bar')?.textContent).toContain('Review');
  });
});

describe('an agent’s page its agent drives (AB-8, ESC-680)', () => {
  const pins = [
    { n: 1, what: 'Défile jusqu’à « Critères »', y: 0, height: 2400, view: 700 },
    { n: 2, what: 'Met en évidence « Critère 3 »', y: 840, height: 2400, view: 700 },
  ];
  const driving = (over = {}) => ({
    browser: 'page:p-12',
    agent: 'a7',
    hand: 'agent' as const,
    stopped: false,
    step: 2,
    what: 'Met en évidence « Critère 3 »',
    waiting: false,
    passwordAsks: [],
    pins,
    ...over,
  });

  it('says beside the trust mention who drives, its last gesture with its number, and draws the rail of its pins', async () => {
    const b = fakeBackend({
      browser_drive_hand: () => driving({ hand: 'user' }),
      browser_drive_place: () => ({ y: 840, height: 2400, view: 700 }),
    });
    drives.rows['page:p-12'] = driving();
    render(AgentPageView, { pageKey: 'page:p-12' });
    const band = document.querySelector('[data-testid="drive-band"]')!;
    expect(band.textContent).toContain('Écrite par pagination');
    expect(band.textContent).toContain('pagination pilote');
    expect(band.querySelector('[data-testid="drive-last"]')?.textContent).toContain('Met en évidence « Critère 3 »');
    expect([...document.querySelectorAll('[data-testid="drive-rail"] [data-pin]')].map((p) => p.textContent)).toEqual(['1', '2']);
    await fireEvent.click(screen.getByRole('button', { name: 'Reprendre la main' }));
    expect(b.called('browser_drive_hand').map((c) => [c.args.key, c.args.hand])).toEqual([['page:p-12', 'take']]);
    await vi.waitFor(() => expect(screen.getByText('Tu as la main')).toBeTruthy());
    await fireEvent.click(screen.getByRole('button', { name: 'La rendre' }));
    expect(b.called('browser_drive_hand').map((c) => c.args.hand)).toEqual(['take', 'give']);
  });

  it('has no rail while nothing was driven, the trust mention alone', () => {
    fakeBackend();
    render(AgentPageView, { pageKey: 'page:p-12' });
    expect(document.querySelector('[data-testid="drive-rail"]')).toBeNull();
    expect(document.querySelector('[data-testid="drive-hand"]')).toBeNull();
  });
});
