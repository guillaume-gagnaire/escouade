import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { discover, keyPressed, momentTaken, offer as offerTip, resetDiscover, TYPING_MS } from '../lib/discover.svelte';
import { FEATURES } from '../lib/features';
import { setLang } from '../lib/i18n';
import { setLocks } from '../lib/locks.svelte';
import { check, offers, resetOffers } from '../lib/offers.svelte';
import { onboarding } from '../lib/onboarding.svelte';
import { app } from '../lib/state.svelte';
import type { DiscoverView, Feature, Features, Offer, OfferAct, OfferView, Settings } from '../lib/types';
import { agent, fakeBackend, resetApp, SETTINGS } from '../test/ipc';
import FeatureOffer from './FeatureOffer.svelte';
import SettingsModal from './modals/SettingsModal.svelte';

// The proposals of a feature switched off at the right moment (F-3, ESC-558): the small window at the bottom right,
// its three gestures, « Ne plus proposer » given back in Réglages › Fonctionnalités, and the moments it never comes:
// while the user types, an agent waits for a permission, a turn on screen goes on, a window or a bubble is there, or
// the session's moment was taken.

const KANBAN_OFF = { ...Object.fromEntries(FEATURES.map((f) => [f, true])), kanban: false } as Features;
const OFFER: Offer = { feature: 'kanban', signal: 'mcpTickets', count: 2 };

/** A core that keeps the proposals as `offers.rs` does, enough for the window. */
function core(start: Offer | null = OFFER) {
  let offer = start;
  let never: Feature[] = [];
  const acts: OfferAct[] = [];
  let settings: Settings = { ...SETTINGS, features: { ...KANBAN_OFF } };
  const view = (extra: Partial<OfferView> = {}): OfferView => ({ offer, never: [...never], ...extra });
  const backend = fakeBackend({
    feature_offer: () => view(),
    feature_offers: () => ({ ...view(), offer: null }),
    feature_offer_act: ({ act }: { act: OfferAct }) => {
      acts.push(act);
      if (act.kind === 'shown') offer = null;
      if (act.kind === 'never') never = [...never, act.feature];
      if (act.kind === 'restore') never = never.filter((f) => f !== act.feature);
      if (act.kind === 'enable') {
        settings = { ...settings, features: { ...settings.features!, [act.feature]: true } };
        return view({ offer: null, settings });
      }
      return view({ offer: null });
    },
    save_settings: () => [],
  });
  return { backend, acts, asked: () => backend.calls.filter((c) => c.cmd === 'feature_offer').length };
}

const card = () => screen.queryByTestId('feature-offer');

describe('a proposal of a feature switched off at the right moment (F-3, ESC-558)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    resetDiscover();
    resetOffers();
    setLocks({ locks: {}, rows: {}, warnings: [] });
    app.settings = { ...SETTINGS, features: { ...KANBAN_OFF } };
    app.modal = null;
    onboarding.shown = false;
  });
  afterEach(() => {
    vi.restoreAllMocks();
    resetOffers();
    resetDiscover();
  });

  it('says what Escouade noticed and what the feature would do, in a dialog that takes no focus', async () => {
    const { acts } = core();
    render(FeatureOffer);
    expect(await check()).toBe(true);
    const dialog = await screen.findByRole('dialog', { name: 'Kanban' });
    expect(dialog).toHaveAttribute('aria-modal', 'false');
    expect(dialog).toHaveTextContent('Fonctionnalité coupée');
    expect(dialog).toHaveTextContent('Ce qu’Escouade a remarqué');
    expect(dialog).toHaveTextContent('2 tickets demandés au serveur MCP cette semaine ont été refusés : le Kanban est coupé.');
    expect(dialog).toHaveTextContent('Ce qu’elle ferait');
    expect(dialog).toHaveTextContent('Le Kanban en ferait des tickets');
    expect(dialog).toHaveTextContent('Ce qu’Escouade remarque reste sur ta machine.');
    expect(
      within(dialog)
        .getAllByRole('button')
        .map((b) => b.textContent?.trim()),
    ).toEqual(['Activer', 'Pas maintenant', 'Ne plus proposer']);
    expect(document.activeElement).not.toBe(dialog);
    // Shown: the core counts it as the day's one; it is the session's moment.
    expect(acts).toEqual([{ kind: 'shown', feature: 'kanban' }]);
    expect(momentTaken()).toBe(true);
  });

  it('« Activer » turns the feature on and takes the settings the core gives back', async () => {
    const { acts } = core();
    render(FeatureOffer);
    await check();
    await userEvent.click(await screen.findByRole('button', { name: 'Activer' }));
    await waitFor(() => expect(card()).toBeNull());
    expect(acts.at(-1)).toEqual({ kind: 'enable', feature: 'kanban' });
    await waitFor(() => expect(app.feature('kanban')).toBe(true));
  });

  it('« Pas maintenant » and Échap put it off, « Ne plus proposer » sets it apart, given back in the settings', async () => {
    const { acts } = core();
    render(FeatureOffer);
    await check();
    await userEvent.click(await screen.findByRole('button', { name: 'Pas maintenant' }));
    await waitFor(() => expect(card()).toBeNull());
    expect(acts.at(-1)).toEqual({ kind: 'later', feature: 'kanban' });
    expect(app.feature('kanban')).toBe(false);

    // A new session: Échap in it is « Pas maintenant ».
    resetDiscover();
    offers.shown = { ...OFFER };
    const dialog = await screen.findByRole('dialog', { name: 'Kanban' });
    dialog.focus();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(card()).toBeNull());
    expect(acts.at(-1)).toEqual({ kind: 'later', feature: 'kanban' });

    offers.shown = { ...OFFER };
    await userEvent.click(await screen.findByRole('button', { name: 'Ne plus proposer' }));
    await waitFor(() => expect(offers.never).toEqual(['kanban']));
    expect(acts.at(-1)).toEqual({ kind: 'never', feature: 'kanban' });

    // Réglages › Fonctionnalités › « Propositions écartées »: « Reproposer » gives it back.
    render(SettingsModal, { page: 'features' });
    const declined = await screen.findByTestId('declined-offers');
    expect(declined).toHaveTextContent('Propositions écartées');
    const again = await within(declined).findByRole('button', { name: 'Reproposer « Kanban »' });
    await userEvent.click(again);
    await waitFor(() => expect(offers.never).toEqual([]));
    expect(acts.at(-1)).toEqual({ kind: 'restore', feature: 'kanban' });
    expect(declined).toHaveTextContent('Aucune : Escouade peut te proposer une fonctionnalité coupée');
  });

  it('never comes while the user types; once the typing stops, it does', async () => {
    const { asked } = core();
    render(FeatureOffer);
    const now = Date.now();
    vi.spyOn(Date, 'now').mockReturnValue(now);
    keyPressed();
    expect(await check()).toBe(false);
    expect(card()).toBeNull();
    expect(asked()).toBe(0);
    // A key a moment ago still counts as typing.
    vi.spyOn(Date, 'now').mockReturnValue(now + TYPING_MS - 1);
    expect(await check()).toBe(false);
    vi.spyOn(Date, 'now').mockReturnValue(now + TYPING_MS);
    expect(await check()).toBe(true);
    expect(await screen.findByTestId('feature-offer')).toBeInTheDocument();
  });

  it('never while an agent waits for a permission, a turn on screen goes on, or a window is open', async () => {
    const { asked } = core();
    render(FeatureOffer);
    const waiting = agent({ id: 'a1', requests: [{ kind: 'permission' }] as never });
    app.agents = { a1: waiting };
    expect(await check()).toBe(false);
    // A turn of the agent on screen.
    app.agents = { a1: agent({ id: 'a1', status: 'running' }) };
    app.ui.activeProject = 'p1';
    app.ui.selectedAgent = { p1: 'a1' };
    app.ui.view = 'project';
    expect(app.shownAgents).toEqual(['a1']);
    expect(await check()).toBe(false);
    app.agents = { a1: agent({ id: 'a1', status: 'done' }) };
    app.modal = { kind: 'newProject' };
    expect(await check()).toBe(false);
    onboarding.shown = true;
    app.modal = null;
    expect(await check()).toBe(false);
    expect(asked()).toBe(0);
    onboarding.shown = false;
    expect(await check()).toBe(true);
  });

  it('one thing at the right moment per session with the bubbles of the discovery: the bubble goes first', async () => {
    core();
    render(FeatureOffer);
    // A bubble on screen: the proposal waits.
    const el = document.createElement('button');
    document.body.append(el);
    discover.view = { items: [], hidden: false, done: 0, total: 0, dot: false, tips: ['dictation'], band: null } as DiscoverView;
    expect(offerTip('dictation', el)).toBe(true);
    expect(await check()).toBe(false);
    // Closed, the session's moment is gone all the same.
    discover.tip = null;
    expect(await check()).toBe(false);
    expect(card()).toBeNull();
    // The next session: the proposal comes, and no bubble after it.
    resetDiscover();
    discover.view = { items: [], hidden: false, done: 0, total: 0, dot: false, tips: ['dictation'], band: null } as DiscoverView;
    expect(await check()).toBe(true);
    expect(offerTip('dictation', el)).toBe(false);
    el.remove();
  });

  it('one at a time: nothing more is asked while one is on screen', async () => {
    const { asked } = core();
    render(FeatureOffer);
    await check();
    expect(asked()).toBe(1);
    resetDiscover();
    expect(await check()).toBe(false);
    expect(asked()).toBe(1);
    expect(screen.getAllByTestId('feature-offer')).toHaveLength(1);
  });

  it('tells what each signal noticed, in English too', async () => {
    setLang('en');
    core({ feature: 'workflows', signal: 'agentFiles', count: 4, project: 'demo-api' });
    render(FeatureOffer);
    await check();
    const dialog = await screen.findByRole('dialog', { name: 'Workflows' });
    expect(dialog).toHaveTextContent('“demo-api” has 4 skills, commands or agents (.claude/, .agents/).');
    expect(dialog).toHaveTextContent('Workflows would read these chains');
    expect(within(dialog).getByRole('button', { name: 'Turn on' })).toBeInTheDocument();
    setLang('fr');
  });
});
