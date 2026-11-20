import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { onboarding } from '../../lib/onboarding.svelte';
import { closeTip, discover, go, keyPressed, offer, offerWhenQuiet, openMap, resetDiscover, TYPING_MS } from '../../lib/discover.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { DiscoverAct, DiscoverItem, DiscoverView, Discovery } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import DiscoverMap from './DiscoverMap.svelte';
import DiscoverTip from './DiscoverTip.svelte';
import NewBand from './NewBand.svelte';

const LANES: [DiscoverItem['lane'], Discovery[]][] = [
  ['plan', ['ticket', 'autopilot', 'docs']],
  ['code', ['dictation', 'voice', 'terminal', 'editor', 'secondAgent']],
  ['review', ['launch', 'forge', 'prView', 'prReview']],
  ['ship', ['approve', 'openPr']],
  ['drive', ['plugin', 'mcp', 'account', 'team']],
];
const NEW: Discovery[] = ['editor', 'prView', 'prReview', 'team'];

/** The core's view: every discovery to try but those said. */
function view(over: Partial<Record<Discovery, Partial<DiscoverItem>>> = {}, extra: Partial<DiscoverView> = {}): DiscoverView {
  const items: DiscoverItem[] = LANES.flatMap(([lane, ids]) =>
    ids.map((id) => ({ id, lane, status: 'todo' as const, absent: null, after: null, new: NEW.includes(id), ...over[id] })),
  );
  const here = items.filter((i) => i.status !== 'absent');
  return {
    items,
    hidden: false,
    done: here.filter((i) => i.status === 'done').length,
    total: here.length,
    dot: true,
    tips: items.filter((i) => i.status === 'todo').map((i) => i.id),
    band: null,
    ...extra,
  };
}

/** A core that keeps the discovery as `discover.rs` does, enough for the window. */
function core(start: DiscoverView) {
  let v = start;
  const set = (id: Discovery, status: DiscoverItem['status']) => {
    v = { ...v, items: v.items.map((i) => (i.id === id ? { ...i, status } : i)) };
  };
  return fakeBackend({
    voice_info: () => ({ devices: [{ id: 'm', name: 'Micro', default: true }], models: [] }),
    discover_view: () => v,
    discover_act: ({ act }: { act: DiscoverAct }) => {
      if (act.kind === 'dismiss') set(act.id, 'dismissed');
      if (act.kind === 'restore') set(act.id, 'todo');
      if (act.kind === 'seen') set(act.id, 'done');
      if (act.kind === 'hide') v = { ...v, hidden: act.hidden, dot: !act.hidden, tips: act.hidden ? [] : v.tips };
      if (act.kind === 'tipped') v = { ...v, tips: v.tips.filter((t) => t !== act.id) };
      if (act.kind === 'bandSeen') v = { ...v, band: null };
      return v;
    },
  });
}

const detail = () => screen.getByTestId('discover-detail');
const stop = (id: Discovery) => document.querySelector<HTMLButtonElement>(`[data-stop="${id}"]`)!;
/** A station, once the view is read. */
async function found(id: Discovery) {
  await waitFor(() => expect(stop(id)).not.toBeNull());
  return stop(id);
}

describe('the map « À découvrir » (ESC-506)', () => {
  beforeEach(() => {
    resetApp();
    resetDiscover();
    app.modal = null;
  });
  afterEach(() => {
    setLang('fr');
    resetDiscover();
  });

  it('draws the five lanes of the cycle, what is done and what is not here, and the station chosen', async () => {
    core(
      view({
        ticket: { status: 'done' },
        terminal: { status: 'done' },
        team: { status: 'absent', absent: { kind: 'noTeamServer' } },
        autopilot: { after: 'ticket' },
      }),
    );
    render(DiscoverMap);
    expect(await screen.findByText('2 sur 17 · chaque station t’amène au vrai geste')).toBeInTheDocument();
    const map = screen.getByRole('list', { name: 'Les découvertes, rangées dans le cycle' });
    expect(
      within(map)
        .getAllByRole('list')
        .map((l) => l.getAttribute('aria-label')),
    ).toEqual(['Planifier', 'Coder', 'Relire', 'Livrer', 'Piloter']);
    expect(stop('ticket').dataset.status).toBe('done');
    expect(within(stop('ticket')).getByText('fait')).toBeInTheDocument();
    expect(within(stop('team')).getByText('pas de serveur d’équipe')).toBeInTheDocument();
    expect(within(stop('autopilot')).getByText('après « Écrire un ticket »')).toBeInTheDocument();
    // The first to try that needs nothing first is chosen.
    expect(detail().dataset.detail).toBe('docs');
    expect(stop('docs')).toHaveAttribute('aria-pressed', 'true');
    // The dictation: its weight said before.
    await userEvent.click(stop('dictation'));
    expect(stop('dictation')).toHaveAttribute('aria-pressed', 'true');
    expect(within(detail()).getByText(/148 Mo, téléchargé une fois/)).toBeInTheDocument();
    expect(within(detail()).getByRole('button', { name: 'Essayer' })).toBeInTheDocument();
    // A station chosen: its detail.
    await userEvent.click(stop('team'));
    expect(detail().dataset.detail).toBe('team');
    expect(within(detail()).getByText(/une invitation de ton équipe/)).toBeInTheDocument();
    expect(within(detail()).queryByRole('button', { name: 'Essayer' })).toBeNull();
  });

  it('a discovery that needs another first leads to that one', async () => {
    core(view({ autopilot: { after: 'ticket' } }));
    render(DiscoverMap);
    await userEvent.click(await found('autopilot'));
    const first = within(detail()).getByRole('button', { name: 'D’abord : Écrire un ticket' });
    await userEvent.click(first);
    // The real screen: the board of the project.
    expect(app.boardOn).toBe(true);
    expect(discover.selected).toBe('ticket');
  });

  it('« Écarter » dismisses it for good, « Remettre » puts it back; a policy says why', async () => {
    const backend = core(view({ mcp: { status: 'absent', absent: { kind: 'policy', why: 'Interdit par la stratégie de ce poste.' } } }));
    render(DiscoverMap);
    await userEvent.click(await found('voice'));
    await userEvent.click(within(detail()).getByRole('button', { name: 'Écarter' }));
    expect(backend.called('discover_act').at(-1)?.args.act).toEqual({ kind: 'dismiss', id: 'voice' });
    await waitFor(() => expect(stop('voice').dataset.status).toBe('dismissed'));
    expect(within(detail()).getByText(/il ne te sera plus proposé/)).toBeInTheDocument();
    await userEvent.click(within(detail()).getByRole('button', { name: 'Remettre' }));
    await waitFor(() => expect(stop('voice').dataset.status).toBe('todo'));
    await userEvent.click(stop('mcp'));
    expect(within(detail()).getByText('Interdit par la stratégie de ce poste.')).toBeInTheDocument();
  });

  it('« Tout masquer » stops the bubbles and the dot, the map stays, « Réafficher » brings them back', async () => {
    const backend = core(view());
    render(DiscoverMap);
    await userEvent.click(await screen.findByRole('button', { name: 'Tout masquer' }));
    expect(backend.called('discover_act').at(-1)?.args.act).toEqual({ kind: 'hide', hidden: true });
    expect(await screen.findByText(/Masqué : plus de bulle ni de point sur Aide/)).toBeInTheDocument();
    expect(discover.view?.dot).toBe(false);
    await userEvent.click(screen.getByRole('button', { name: 'Réafficher' }));
    await waitFor(() => expect(discover.view?.hidden).toBe(false));
  });

  it('opened from the band, it shows what is new in the release alone', async () => {
    core(view());
    openMap(true, '2.3');
    render(DiscoverMap);
    await screen.findByText('Nouveau dans la 2.3');
    await found('editor');
    expect([...document.querySelectorAll<HTMLElement>('[data-stop]')].map((b) => b.dataset.stop)).toEqual(NEW);
    await userEvent.click(screen.getByRole('button', { name: 'Tout voir' }));
    expect(document.querySelectorAll('[data-stop]')).toHaveLength(18);
  });

  it('says it in English too', async () => {
    setLang('en');
    core(view({ ticket: { status: 'done' } }));
    render(DiscoverMap);
    expect(await screen.findByText('1 of 18 · each stop takes you to the real thing')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Code' })).toBeInTheDocument();
  });
});

describe('« Essayer » leads to the real screen, and never does the gesture itself (ESC-506)', () => {
  beforeEach(() => {
    resetApp();
    resetDiscover();
    app.modal = null;
    core(view());
  });

  it('opens the board, the Docs, the PRs, Plugins and the right settings', () => {
    go('ticket');
    expect(app.boardOn).toBe(true);
    go('docs');
    expect(app.docsOn).toBe(true);
    go('plugin');
    expect(app.ui.view).toBe('plugins');
    go('forge');
    expect(app.modal).toEqual({ kind: 'settings', page: 'services' });
    go('mcp');
    expect(app.modal).toEqual({ kind: 'settings', page: 'agents' });
    go('account');
    expect(app.modal).toEqual({ kind: 'settings', page: 'accounts' });
    // The launch commands: the page, never « Proposer des commandes » (a message to an agent) by itself.
    go('launch');
    expect(app.modal).toEqual({ kind: 'settings', page: 'launch', projectId: 'p1' });
    go('team');
    expect(app.modal).toEqual({ kind: 'team', step: 'join' });
  });

  it('with no project, it starts with one', () => {
    resetApp({ projects: [] });
    go('terminal');
    expect(app.modal).toEqual({ kind: 'newProject' });
  });
});

describe('the bubbles at the right moment (ESC-506)', () => {
  let el: HTMLButtonElement;
  beforeEach(async () => {
    resetApp();
    resetDiscover();
    el = document.createElement('button');
    el.textContent = 'micro';
    document.body.append(el);
  });
  afterEach(() => {
    el.remove();
    resetDiscover();
  });

  async function ready(v = view()) {
    const backend = core(v);
    await openMapLoad();
    return backend;
  }
  /** The view read as at the start. */
  async function openMapLoad() {
    const { load } = await import('../../lib/discover.svelte');
    await load();
  }

  it('shows one, on the real control, once, and tells the core never to show it again', async () => {
    const backend = await ready();
    render(DiscoverTip);
    expect(offer('dictation', el)).toBe(true);
    const tip = await screen.findByRole('dialog', { name: 'À découvrir : Dicter un message' });
    expect(tip).toHaveAttribute('aria-modal', 'false');
    expect(within(tip).getByText(/148 Mo/)).toBeInTheDocument();
    expect(backend.called('discover_act').at(-1)?.args.act).toEqual({ kind: 'tipped', id: 'dictation' });
    // One at a time, and one per session.
    expect(offer('autopilot', el)).toBe(false);
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(offer('autopilot', el)).toBe(false);
  });

  it('never while the user types, nor while an agent waits for a permission, nor what the core no longer allows', async () => {
    await ready(view({}, { tips: ['autopilot'] }));
    expect(offer('dictation', el)).toBe(false);
    keyPressed();
    expect(offer('autopilot', el)).toBe(false);
    await new Promise((r) => setTimeout(r, TYPING_MS + 50));
    app.agents = {
      a1: agent({
        requests: [{ id: 'r', kind: 'permission', tool: 'Bash', arg: 'npm test', description: null } as never],
      }),
    };
    expect(offer('autopilot', el)).toBe(false);
    app.agents = {};
    // Nor over the first launch (ESC-505): its first agent's real composer has the microphone.
    onboarding.shown = true;
    expect(offer('autopilot', el)).toBe(false);
    onboarding.shown = false;
    expect(offer('autopilot', el)).toBe(true);
    closeTip();
  });

  it('leaves on a press elsewhere, which goes on to what it was for', async () => {
    await ready();
    render(DiscoverTip);
    offer('dictation', el);
    const tip = await screen.findByRole('dialog');
    // Its own buttons keep it.
    tip.querySelector('.acts')!.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    expect(screen.queryByRole('dialog')).not.toBeNull();
    const other = document.createElement('button');
    document.body.append(other);
    other.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    other.remove();
  });

  it('leaves when the user starts typing', async () => {
    await ready();
    render(DiscoverTip);
    offer('dictation', el);
    await screen.findByRole('dialog');
    await userEvent.keyboard('a');
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('leaves when its control is pressed: the gesture is under way', async () => {
    await ready();
    render(DiscoverTip);
    offer('dictation', el);
    await screen.findByRole('dialog');
    el.dispatchEvent(new Event('pointerdown', { bubbles: true }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('never over a window open, whose control it would point at under it, and waits for the window to close', async () => {
    // ESC-522: the bubble of the first ticket, come late on a loaded machine, showed over the window of the next ticket
    // and took its Échap: the window never asked to give up what was typed.
    await ready(view({}, { tips: ['autopilot'] }));
    app.modal = { kind: 'newProject' };
    expect(offer('autopilot', el)).toBe(false);
    offerWhenQuiet('autopilot', () => el, true);
    expect(discover.tip).toBeNull();
    app.modal = null;
    await waitFor(() => expect(discover.tip?.id).toBe('autopilot'), { timeout: 1500 });
    closeTip();
  });

  it('leaves when a window opens over the screen: Échap is for the window then', async () => {
    await ready();
    render(DiscoverTip);
    offer('dictation', el);
    await screen.findByRole('dialog');
    app.modal = { kind: 'newProject' };
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    app.modal = null;
  });

  it('a moment that comes while the user types waits for the typing to stop', async () => {
    await ready();
    keyPressed();
    offerWhenQuiet('autopilot', () => el, true);
    expect(discover.tip).toBeNull();
    await waitFor(() => expect(discover.tip?.id).toBe('autopilot'), { timeout: TYPING_MS + 1500 });
    closeTip();
  });

  it('« Écarter » in the bubble dismisses the discovery', async () => {
    const backend = await ready();
    render(DiscoverTip);
    offer('dictation', el);
    await userEvent.click(await screen.findByRole('button', { name: 'Écarter' }));
    expect(backend.called('discover_act').at(-1)?.args.act).toEqual({ kind: 'dismiss', id: 'dictation' });
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });
});

describe('the band « Nouveau dans la 2.3 » (ESC-506)', () => {
  beforeEach(() => {
    resetApp();
    resetDiscover();
  });

  it('opens the map on what is new, and is never shown again once seen or closed', async () => {
    const backend = core(view({}, { band: '2.3' }));
    const { load } = await import('../../lib/discover.svelte');
    await load();
    render(NewBand);
    expect(screen.getByText('Nouveau dans la 2.3 : les PR dans Escouade, l’éditeur VS Code, ton équipe.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Voir' }));
    expect(app.ui.view).toBe('discover');
    expect(discover.onlyNew).toBe(true);
    expect(discover.release).toBe('2.3');
    expect(backend.called('discover_act').at(-1)?.args.act).toEqual({ kind: 'bandSeen' });
    await waitFor(() => expect(screen.queryByTestId('discover-band')).toBeNull());
  });

  it('× closes it for the release', async () => {
    const backend = core(view({}, { band: '2.3' }));
    const { load } = await import('../../lib/discover.svelte');
    await load();
    render(NewBand);
    await userEvent.click(screen.getByRole('button', { name: 'Fermer pour cette version' }));
    expect(backend.called('discover_act').at(-1)?.args.act).toEqual({ kind: 'bandSeen' });
    expect(app.ui.view).not.toBe('discover');
  });
});
