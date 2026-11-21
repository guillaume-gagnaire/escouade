// The filter of the box « À toi » by type (ESC-578), in its queue: « Tout » then each type that waits with its number,
// several at once, kept on this machine, cleared in one gesture; the groups and their order stay, the rail's count does
// not move, what it hides is said quietly; the keyboard and a screen reader.
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { FILTER_KEY } from '../../lib/inbox/filter';
import { inbox } from '../../lib/inbox/inbox.svelte';
import type { InboxItem, InboxViews } from '../../lib/inbox/views';
import { inboxViews } from '../../lib/inbox/views.svelte';
import { app } from '../../lib/state.svelte';
import type { WindowAsk } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import { windowAsk } from '../../test/inbox';
import InboxQueue from './InboxQueue.svelte';
import InboxScreen from './InboxScreen.svelte';

vi.mock('../../lib/previews/pdf', () => ({ PdfPasswordError: class extends Error {}, openPdf: async () => null }));

const NOW = Date.now();
const MIN = 60_000;

function view(over: Partial<InboxItem> = {}): InboxItem {
  return {
    key: 'decision:p1:a',
    source: 'decision',
    group: 'earlier',
    projectId: 'p1',
    projectName: 'portail',
    private: false,
    title: 'Trois échéances',
    since: NOW - 40 * MIN,
    agentId: null,
    agentName: null,
    ticketId: null,
    ticketKey: null,
    snoozedUntil: null,
    request: null,
    column: null,
    blocked: null,
    conflict: false,
    partial: false,
    decision: 'a',
    files: 0,
    number: null,
    url: null,
    word: null,
    mark: null,
    ...over,
  };
}

const ticket = (key: string, source: 'ticketReview' | 'ticketBlocked') =>
  view({ key: `ticket:${key}`, source, group: 'today', title: `Ticket ${key}`, ticketId: key, ticketKey: key, decision: null });

let views: InboxViews;
let asks: WindowAsk[];

beforeEach(() => {
  localStorage.clear();
  resetApp();
  setLang('fr');
  inbox.reset();
  inboxViews.reset();
  app.projects = [project({ id: 'p1', name: 'portail' })];
  views = { enabled: true, views: [], blocking: 0, total: 0 };
  asks = [];
  fakeBackend({
    inbox_list: () => ({ enabled: true, asks: [...asks], open: asks.length }),
    inbox_views: () => ({ ...views, views: [...views.views] }),
    inbox_seen: () => null,
  });
});

async function show(vs: InboxItem[], as: WindowAsk[]) {
  views = { enabled: true, views: vs, blocking: 0, total: vs.length };
  asks = as;
  app.settings.mcpEnabled = true;
  app.ui.view = 'inbox';
  await inbox.load();
  await inboxViews.load();
  render(InboxQueue);
  render(InboxScreen);
  await tick();
}

/** A box with several types: two go-aheads, a choice, a plain yes or no, two tickets, a decision. */
const mixed = () =>
  show(
    [ticket('DEM-1', 'ticketReview'), ticket('DEM-2', 'ticketBlocked'), view()],
    [
      windowAsk({ id: 'g-1', kind: 'confirm', gate: true, title: 'Publier la 2.2.6', createdAt: NOW - 3 * MIN }),
      windowAsk({ id: 'g-2', kind: 'confirm', gate: true, title: 'Signer le plugin', createdAt: NOW - 2 * MIN }),
      windowAsk({ id: 'c-1', kind: 'choice', title: 'La direction du tableau', createdAt: NOW - MIN }),
      windowAsk({ id: 'q-1', kind: 'confirm', gate: false, title: 'Garder l’ancien nom ?', createdAt: NOW - 30_000 }),
    ],
  );

const filter = () => screen.getByTestId('inbox-filter');
const chip = (name: RegExp | string) => within(filter()).getByRole('button', { name });
const titles = () =>
  within(screen.getByTestId('inbox-queue'))
    .getAllByRole('button')
    .filter((b) => b.dataset.testid === 'inbox-row' || b.dataset.testid === 'ivw-row')
    .map((b) => b.querySelector('.t')?.textContent);
const headings = () =>
  within(screen.getByTestId('inbox-queue'))
    .getAllByRole('heading', { level: 2 })
    .map((h) => h.textContent?.replace(/\s*\d+\s*$/, '').trim());

describe('the filter of the box by type', () => {
  it('offers « Tout », then each type that waits with its number, gathered, in a fixed order', async () => {
    await mixed();
    const names = within(filter())
      .getAllByRole('button')
      .map((b) => b.textContent?.trim());
    expect(names).toEqual(['Tout', 'Questions 1', 'Choix 1', 'Feux verts 2', 'Tickets 2', 'Décisions 1']);
    expect(chip('Tout')).toHaveAttribute('aria-pressed', 'true');
    expect(filter()).toHaveAttribute('aria-label', 'Filtrer par type');
    // No filter: nothing said hidden, no « Effacer ».
    expect(screen.getByTestId('inbox-filter-hidden')).toBeEmptyDOMElement();
    expect(screen.queryByTestId('inbox-filter-clear')).toBeNull();
  });

  it('keeps the groups and their order, shows only the types chosen, several at once', async () => {
    await mixed();
    expect(headings()).toEqual(['Aujourd’hui', 'Plus tôt']);
    await userEvent.click(chip(/^Feux verts/));
    expect(titles()).toEqual(['Publier la 2.2.6', 'Signer le plugin']);
    expect(headings()).toEqual(['Aujourd’hui']);
    await userEvent.click(chip(/^Décisions/));
    expect(titles()).toEqual(['Publier la 2.2.6', 'Signer le plugin', 'Trois échéances']);
    expect(headings()).toEqual(['Aujourd’hui', 'Plus tôt']);
    expect(chip(/^Feux verts/)).toHaveAttribute('aria-pressed', 'true');
    expect(chip(/^Décisions/)).toHaveAttribute('aria-pressed', 'true');
    expect(chip('Tout')).toHaveAttribute('aria-pressed', 'false');
    // J and K go through the rows shown: « 1 sur 3 ».
    await waitFor(() => expect(screen.getByTestId('inbox-nth')).toHaveTextContent('1 sur 3'));
  });

  it('does not change the rail’s count, and says quietly what it hides', async () => {
    await mixed();
    const before = inbox.count.total;
    expect(before).toBe(7);
    await userEvent.click(chip(/^Tickets/));
    expect(inbox.count.total).toBe(before);
    // The header and « il en reste » say all that waits; the line says what the filter hides.
    expect(screen.getByTestId('inbox-scoped-count')).toHaveTextContent('7');
    expect(screen.getByTestId('inbox-queue')).toHaveTextContent('il en reste 7');
    const hidden = screen.getByTestId('inbox-filter-hidden');
    expect(hidden).toHaveAttribute('role', 'status');
    expect(hidden).toHaveTextContent('5 autres demandes masquées par le filtre');
    // A request of a type hidden arrives: the line says one more, the list does not show it.
    asks = [...asks, windowAsk({ id: 'c-2', kind: 'choice', title: 'Le logo', createdAt: NOW })];
    await inbox.load();
    await tick();
    expect(hidden).toHaveTextContent('6 autres demandes masquées par le filtre');
    expect(titles()).toEqual(['Ticket DEM-1', 'Ticket DEM-2']);
    expect(inbox.count.total).toBe(8);
  });

  it('is cleared in one gesture, by « Effacer », « Tout » or « Tout montrer »', async () => {
    await mixed();
    for (const clear of [
      () => screen.getByTestId('inbox-filter-clear'),
      () => chip('Tout'),
      () => within(screen.getByTestId('inbox-filter-hidden')).getByRole('button', { name: 'Tout montrer' }),
    ]) {
      await userEvent.click(chip(/^Choix/));
      expect(titles()).toHaveLength(1);
      expect(screen.getByTestId('inbox-filter-clear')).toHaveAccessibleName('Effacer le filtre');
      await userEvent.click(clear());
      expect(titles()).toHaveLength(7);
      expect(screen.queryByTestId('inbox-filter-clear')).toBeNull();
      expect(localStorage.getItem(FILTER_KEY)).toBeNull();
    }
  });

  it('is kept on this machine from one opening to the next', async () => {
    await mixed();
    await userEvent.click(chip(/^Feux verts/));
    await userEvent.click(chip(/^Tickets/));
    expect(JSON.parse(localStorage.getItem(FILTER_KEY) ?? '')).toEqual(['gate', 'tickets']);
    // The window opened again: the store reads it back.
    const { readFilter } = await import('../../lib/inbox/filter');
    expect(readFilter(localStorage)).toEqual(['gate', 'tickets']);
  });

  it('keeps a type chosen at 0 once its last request is gone, and says so when it hides everything', async () => {
    await mixed();
    await userEvent.click(chip(/^Choix/));
    asks = asks.filter((a) => a.id !== 'c-1');
    await inbox.load();
    await tick();
    expect(chip(/^Choix/)).toHaveTextContent('Choix 0');
    expect(titles()).toEqual([]);
    // Never « Rien ne t'attend » while the filter hides what waits.
    expect(screen.queryByTestId('inbox-empty')).toBeNull();
    expect(screen.getByTestId('inbox-filter-empty')).toHaveTextContent('Rien de ce type ne t’attend');
    expect(screen.getByTestId('inbox-filter-empty')).toHaveTextContent('6 autres demandes attendent, masquées par le filtre.');
    await userEvent.click(screen.getByTestId('inbox-filter-empty-clear'));
    expect(titles()).toHaveLength(6);
  });

  it('opens a row shown when the one open is hidden by the filter', async () => {
    await mixed();
    await waitFor(() => expect(inbox.selected).toBe('g-1'));
    await userEvent.click(chip(/^Décisions/));
    expect(inbox.selected).toBe('view:decision:p1:a');
  });

  it('goes from one of its buttons to another with the arrows, Home and End, and toggles with Space and Enter', async () => {
    await mixed();
    chip('Tout').focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(chip(/^Questions/)).toHaveFocus();
    await userEvent.keyboard('{End}');
    expect(chip(/^Décisions/)).toHaveFocus();
    await userEvent.keyboard('{ArrowRight}');
    expect(chip('Tout')).toHaveFocus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(chip(/^Décisions/)).toHaveFocus();
    await userEvent.keyboard(' ');
    expect(chip(/^Décisions/)).toHaveAttribute('aria-pressed', 'true');
    await userEvent.keyboard('{Home}');
    expect(chip('Tout')).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    expect(chip(/^Décisions/)).toHaveAttribute('aria-pressed', 'false');
  });

  it('is not offered with a single type and nothing chosen', async () => {
    await show([], [windowAsk({ id: 'c-1', kind: 'choice' })]);
    expect(screen.queryByTestId('inbox-filter')).toBeNull();
  });

  it('speaks English', async () => {
    setLang('en');
    await mixed();
    const names = within(filter())
      .getAllByRole('button')
      .map((b) => b.textContent?.trim());
    expect(names).toEqual(['All', 'Questions 1', 'Choices 1', 'Go-aheads 2', 'Tickets 2', 'Decisions 1']);
    await userEvent.click(chip(/^Tickets/));
    expect(screen.getByTestId('inbox-filter-hidden')).toHaveTextContent('5 more requests hidden by the filter');
  });
});
