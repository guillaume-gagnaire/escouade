// The card of a page an agent showed (show_page, AP-3; docs/AGENT-PAGES.md § 4.4): in place of its call's row, with its
// states — opening, shown or updated (beside, or waiting in « Navigateurs »), a newer version since, closed (« Rouvrir »),
// no longer kept, the pages off — in French and in English; a failed call keeps its row.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { agentpages, pageCall } from '../../lib/agentpages.svelte';
import { devpages } from '../../lib/devpage.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { AgentPageRow, ToolItem } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import PageCard from './PageCard.svelte';

const A = agent({ id: 'a7', name: 'pagination' });

function call(over: Partial<ToolItem> = {}, answer: object | null = { page: 'p-12', version: 1, opened: 'beside' }): ToolItem {
  return {
    kind: 'tool',
    id: 't1',
    ts: 0,
    name: 'mcp__escouade__show_page',
    input: { title: 'Revue avant fusion · POR-41', template: 'review', data: {} },
    status: answer ? 'ok' : 'running',
    result: answer
      ? { text: JSON.stringify({ browser: 'page:p-12', ask: null, bytes: 1, files: 1, ...answer }), isError: false }
      : undefined,
    ...over,
  } as ToolItem;
}

function row(over: Partial<AgentPageRow> = {}): AgentPageRow {
  return {
    page: 'p-12',
    projectId: 'p1',
    agentId: 'a7',
    agentName: 'pagination',
    ticketId: null,
    title: 'Revue avant fusion · POR-41',
    template: 'review',
    versions: [{ n: 1, at: 0 }],
    shown: 1,
    open: true,
    quiet: false,
    loaded: true,
    ...over,
  };
}

const card = () => document.querySelector<HTMLElement>('[data-page-card]');

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [A] });
  app.ui.panes = {};
  agentpages.info = {};
  devpages.pages = {};
});

describe('the card of a page', () => {
  it('reads a call of show_page, and none that failed or is another tool', () => {
    expect(pageCall(call())).toMatchObject({ page: 'p-12', version: 1, opened: 'beside', template: 'review' });
    expect(pageCall(call({}, null))).toMatchObject({ page: null, title: 'Revue avant fusion · POR-41' });
    expect(pageCall(call({ status: 'error', result: { text: 'Le titre…', isError: true } }))).toBeNull();
    expect(pageCall(call({ name: 'mcp__escouade__get_page' }))).toBeNull();
    expect(pageCall(call({}, { page: '../x' }))).toBeNull();
  });

  it('says the page opens while the call runs', () => {
    fakeBackend();
    render(PageCard, { item: call({}, null), agent: A });
    expect(card()?.dataset.state).toBe('running');
    expect(card()?.textContent).toContain('Ouverture de la page…');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('shown beside the conversation: its title, its agent, its template, its version, « Afficher »', async () => {
    fakeBackend({ agentpage_info: () => row() });
    devpages.pages['page:p-12'] = {
      url: '',
      title: 'x',
      phase: 'ready',
      projectId: 'p1',
      by: { kind: 'page', id: 'a7', page: 'p-12' },
      opened: 1,
    };
    app.selectedPage.p1 = 'page:p-12';
    render(PageCard, { item: call(), agent: A });
    await vi.waitFor(() => expect(card()?.textContent).toContain('à côté'));
    expect(screen.getByRole('group', { name: 'Page montrée par pagination' })).toBeTruthy();
    expect(card()?.textContent).toContain('Revue avant fusion · POR-41');
    expect(card()?.textContent).toContain('Page de pagination · modèle Revue · v1 · à côté');
    expect(card()?.textContent).toContain('Montrée');
    expect(screen.getByRole('button', { name: 'Afficher' })).toBeTruthy();
  });

  it('a later version: « Mise à jour », and a newer one since says so; its row waiting in « Navigateurs »', async () => {
    fakeBackend({
      agentpage_info: () =>
        row({
          versions: [
            { n: 1, at: 0 },
            { n: 2, at: 1 },
            { n: 3, at: 2 },
          ],
          shown: 3,
        }),
    });
    render(PageCard, { item: call({}, { page: 'p-12', version: 2, opened: 'waiting' }), agent: A });
    await vi.waitFor(() => expect(card()?.textContent).toContain('v3 depuis'));
    expect(card()?.dataset.state).toBe('waiting');
    expect(card()?.textContent).toContain('Mise à jour');
    expect(card()?.textContent).toContain('v2 · attend dans Navigateurs');
  });

  it('closed: « Rouvrir » opens its row again, beside its agent’s conversation', async () => {
    const b = fakeBackend({
      agentpage_info: () => row({ open: false }),
      agentpage_reopen: () => row({ open: true }),
    });
    app.ui.view = 'project';
    app.ui.activeProject = 'p1';
    render(PageCard, { item: call(), agent: A });
    await vi.waitFor(() => expect(card()?.dataset.state).toBe('closed'));
    expect(card()?.textContent).toContain('fermée');
    await fireEvent.click(screen.getByRole('button', { name: 'Rouvrir' }));
    await vi.waitFor(() => expect(devpages.pages['page:p-12']).toBeTruthy());
    expect(b.called('agentpage_reopen')).toEqual([{ cmd: 'agentpage_reopen', args: { page: 'p-12' } }]);
    expect(devpages.pages['page:p-12']?.by).toEqual({ kind: 'page', id: 'a7', page: 'p-12' });
    expect(app.panes?.root).toBeTruthy();
  });

  it('no longer kept: says why, nothing to open', async () => {
    fakeBackend({ agentpage_info: () => null });
    render(PageCard, { item: call(), agent: A });
    await vi.waitFor(() => expect(card()?.dataset.state).toBe('erased'));
    expect(card()?.textContent).toContain('plus gardée');
    expect(card()?.textContent).toContain('Cette page n’est plus gardée');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('the pages off: says where they turn back on', async () => {
    fakeBackend({ agentpage_info: () => row() });
    app.settings = { ...app.settings, agentPages: false };
    render(PageCard, { item: call(), agent: A });
    await tick();
    expect(card()?.dataset.state).toBe('off');
    expect(card()?.textContent).toContain('Les pages des agents sont coupées (Réglages › Agents).');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('speaks English', async () => {
    setLang('en');
    fakeBackend({ agentpage_info: () => row({ open: false }) });
    render(PageCard, { item: call(), agent: A });
    await vi.waitFor(() => expect(card()?.textContent).toContain('Page by pagination · Review template · v1 · closed'));
    expect(screen.getByRole('button', { name: 'Reopen' })).toBeTruthy();
  });
});
