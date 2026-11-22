// The proposals of a feature switched off at the right moment (F-3, ESC-558, docs/FEATURES.md § 2, spec
// « Fonctionnalités »): when the core noticed that a feature off would serve, a small window at the bottom right says
// what it noticed and what the feature would do, with « Activer », « Pas maintenant » and « Ne plus proposer ». The core
// keeps what was noticed and answered (`offers.rs`), and holds the rules it can see: one a day, a week after « Pas
// maintenant », never a feature imposed, never during a turn on screen nor while an agent waits for a permission. The
// window adds its own: never while the user types, nor over a window, nor over the first launch; one at a time; and one
// thing at the right moment per session with the bubbles of the discovery (ESC-506): the bubble goes first.
//
// A client of the remote API shows its core's proposals the same way. Nothing here leaves the machine.

import { momentTaken, permissionWaiting, takeMoment, typing, discover } from './discover.svelte';
import { tour } from './features-tour.svelte';
import { api } from './ipc';
import { onboarding } from './onboarding.svelte';
import { app } from './state.svelte';
import type { Feature, Offer, OfferView, UiEvent } from './types';

/** How often the window asks whether a proposal is due, besides when the core says it noticed something. */
export const OFFER_POLL_MS = 60_000;
/** How long after the window is ready it asks first. */
export const OFFER_FIRST_MS = 3_000;

export const offers = $state<{
  /** The proposal on screen: one at a time, until it is answered. */
  shown: Offer | null;
  /** « Ne plus proposer »: Réglages › Fonctionnalités › « Propositions écartées ». */
  never: Feature[];
}>({ shown: null, never: [] });

/** A turn of an agent on screen is under way. */
function turnOnScreen(): boolean {
  return app.shownAgents.some((id) => app.agents[id]?.status === 'running');
}

/**
 * Not the moment: the user types, an agent waits for a permission, a turn on screen goes on, the first launch or a
 * window is over the screen (the assistant of the features too, ESC-559), or a bubble of the discovery is there (it goes
 * first).
 */
export function busy(): boolean {
  return typing() || permissionWaiting() || turnOnScreen() || onboarding.shown || tour.shown || app.modal !== null || !!discover.tip;
}

let asking = false;

/** Asks the core whether a proposal is due, and shows it when the moment allows. True when one shows. */
export async function check(): Promise<boolean> {
  if (asking || offers.shown || momentTaken() || !app.ready || busy()) return false;
  asking = true;
  let v: OfferView | null;
  try {
    v = await api.featureOffer([...app.shownAgents]).catch(() => null);
  } finally {
    asking = false;
  }
  if (!v) return false;
  offers.never = v.never;
  const offer = v.offer;
  // The moment may have passed while the core answered.
  if (!offer || offers.shown || momentTaken() || busy()) return false;
  takeMoment();
  offers.shown = offer;
  void api.featureOfferAct({ kind: 'shown', feature: offer.feature }).catch(() => {});
  return true;
}

/** « Activer », « Pas maintenant » or « Ne plus proposer » of the proposal on screen. */
export async function answer(kind: 'enable' | 'later' | 'never') {
  const o = offers.shown;
  if (!o) return;
  offers.shown = null;
  try {
    const v = await api.featureOfferAct({ kind, feature: o.feature });
    offers.never = v.never;
    if (v.settings) app.takeSettings(v.settings);
  } catch (e) {
    app.toast(String(e), 'error');
  }
}

/** « Reproposer » of « Propositions écartées »: the feature may be proposed again. */
export async function restore(feature: Feature) {
  const v = await api.featureOfferAct({ kind: 'restore', feature }).catch(() => null);
  if (v) offers.never = v.never;
}

/** The proposals set apart for good, read again (Réglages › Fonctionnalités). */
export async function loadNever() {
  const v = await api.featureOffers().catch(() => null);
  if (v) offers.never = v.never;
}

/** An event of the core heard: it noticed or answered something, a proposal may be due. */
export function hear(type: UiEvent['type']) {
  if (type === 'featureOffer') void check();
}

let timer: ReturnType<typeof setInterval> | undefined;
let first: ReturnType<typeof setTimeout> | undefined;

/** Asks now and then while the window is open. */
export function start() {
  stop();
  first = setTimeout(() => void check(), OFFER_FIRST_MS);
  timer = setInterval(() => void check(), OFFER_POLL_MS);
}

export function stop() {
  clearTimeout(first);
  clearInterval(timer);
}

/** For the tests: a new session. */
export function resetOffers() {
  stop();
  offers.shown = null;
  offers.never = [];
  asking = false;
}
