// The assistant of the features (F-2, ESC-559, docs/FEATURES.md § 1, spec « Fonctionnalités »): one feature per screen,
// a capture of the real app, what it does in three points and a big switch « Activer ». It is a station of the first
// launch (FeaturesStation), it opens by itself at the first window of the version that brings the switches on an
// installation updated to it (the core says so: `features::TourView.open`), and « Revoir la présentation » of Réglages ›
// Fonctionnalités opens it again. Its switches are the settings' own, saved as Réglages saves them: « Passer » changes
// none, so it keeps the essential and what is used (a new installation has the rest off, `NEW_INSTALL`).

import { FEATURES, featureSwitch } from './features';
import { api } from './ipc';
import { lockedBy } from './locks.svelte';
import { featureRow } from './features';
import { app } from './state.svelte';
import type { Feature, Features, FeatureUse, FeaturesTourView, LockedBy } from './types';

/** Why the assistant is on screen: the first launch's station, the update that brings the choice, « Revoir la présentation »,
 * or the window of the novelties (a feature a later update brought). */
export type TourMode = 'first' | 'update' | 'review' | 'new';

export const tour = $state<{
  /** What the core said last. */
  view: FeaturesTourView | null;
  /** Shown on its own, over the app (the update, « Revoir la présentation »); the station shows it in the journey. */
  shown: boolean;
  mode: TourMode;
}>({ view: null, shown: false, mode: 'update' });

/** The core's view taken; shown by itself when it says so (the first window after the update), never over the journey. */
export function takeTour(view: FeaturesTourView | null, journey = false): void {
  tour.view = view;
  if (view?.open && !journey && !tour.shown) {
    tour.mode = 'update';
    tour.shown = true;
  }
}

/** The core's view read again. */
export async function loadTour(journey = false): Promise<void> {
  takeTour(await api.featuresTourView().catch(() => null), journey);
}

/** « Revoir la présentation » (Réglages › Fonctionnalités): every screen again, with the switches as they are. The settings'
 * window is closed first: its draft would save the switches of before. */
export async function reviewTour(): Promise<void> {
  app.modal = null;
  await loadTour(true);
  tour.mode = 'review';
  tour.shown = true;
}

/** Ended (« Terminer ») or passed (« Passer : garder l'essentiel »): no switch changes, it never opens again by itself. */
export async function endTour(): Promise<void> {
  const v = await api.featuresTourAct({ kind: 'end' }).catch(() => null);
  if (v) tour.view = v;
  tour.shown = false;
}

/** The features of the window of the novelties met: their screens are not shown again. */
export async function metFeatures(features: Feature[]): Promise<void> {
  if (!features.length) return;
  const v = await api.featuresTourAct({ kind: 'met', features }).catch(() => null);
  if (v) tour.view = v;
}

/** What the installation uses of `f` (noted at the update, or now); undefined when it does not. */
export function usedOf(f: Feature, used: FeatureUse[] | undefined = tour.view?.used): FeatureUse | undefined {
  return used?.find((u) => u.feature === f);
}

/** Who imposes the switch of `f` (the device's policy, a team); null when nobody does. */
export const lockOf = (f: Feature): LockedBy | null => lockedBy(featureRow(f));

/** The switch of `f` saved as Réglages › Fonctionnalités saves it (the core stops or starts what it does). */
export async function setFeature(f: Feature, on: boolean): Promise<boolean> {
  const settings = $state.snapshot(app.settings);
  const features = Object.fromEntries(FEATURES.map((x) => [x, featureSwitch(settings, x)])) as Features;
  features[f] = on;
  const next = { ...settings, features };
  const ok = await app.run(api.saveSettings(next).then(() => true));
  if (ok) app.settings = next;
  return !!ok;
}

/** The features switched on now: what the line says under the station. */
export const onCount = (): number => FEATURES.filter((f) => featureSwitch(app.settings, f)).length;
