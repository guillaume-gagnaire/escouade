// The first launch's journey (ESC-504, docs/ONBOARDING.md, direction C « La ligne »): what the core says of it
// (`onboarding::OnboardingView`), the station on screen, and the gestures on it. The core keeps where it stands (in
// `state.json`) and reads the real state of the app: the window shows it and asks it again after each change.

import { api } from './ipc';
import type { EngineId, Hookup, OnboardingAct, OnboardingChoice, OnboardingStep, OnboardingView, StepStatus } from './types';

export const onboarding = $state<{
  /** What the core said last; null until the start. */
  view: OnboardingView | null;
  /** On screen, in place of the rail and the work. */
  shown: boolean;
  /** The station on screen; null on the screen of the end. */
  at: OnboardingStep | null;
  /** The harness chosen at its station, for the next ones (the account, the defaults). */
  engine: EngineId | null;
  /** The project opened at its station (ESC-505): where the first agent works. */
  project: string | null;
}>({ view: null, shown: false, at: null, engine: null, project: null });

/** The stations, in their order, as the core gives them. */
export function stations(): OnboardingStep[] {
  return onboarding.view?.steps.map((s) => s.id) ?? [];
}

/** Where `step` stands. */
export function statusOf(step: OnboardingStep): StepStatus {
  return onboarding.view?.steps.find((s) => s.id === step)?.status ?? 'todo';
}

/** Where a card of « Ce que tu veux brancher » stands (ESC-512). */
export function cardStatus(h: Hookup): StepStatus {
  return onboarding.view?.hookups?.find((c) => c.id === h)?.status ?? 'todo';
}

/**
 * The cards passed (their « Plus tard », left unticked, or the station passed): what the discovery (ON-3, Aide › À
 * découvrir) offers again, and the pages of Réglages that plug them in. A card a policy locks is not one.
 */
export function passedHookups(): Hookup[] {
  return (onboarding.view?.hookups ?? []).filter((c) => c.status === 'skipped').map((c) => c.id);
}

/** The view of the start (`InitialState.onboarding`): shown when the core says so, at its station. */
export function takeView(view: OnboardingView | undefined | null): void {
  onboarding.view = view ?? null;
  if (view?.open) {
    onboarding.shown = true;
    onboarding.at = view.resumeAt;
  }
}

/** The core's view read again (a setting, an account, a project changed): the stations the real state made done. */
export async function refresh(): Promise<void> {
  const v = await api.onboardingView().catch(() => null);
  if (v) onboarding.view = v;
}

/** `act` told to the core; its view taken. */
export async function act(a: OnboardingAct): Promise<OnboardingView | null> {
  const v = await api.onboardingAct(a).catch(() => null);
  if (v) onboarding.view = v;
  return v;
}

/** The station after `step` the journey goes to: the next one, or the next one neither done nor passed (a first start). */
export function after(step: OnboardingStep): OnboardingStep | null {
  const list = stations();
  const rest = list.slice(list.indexOf(step) + 1);
  if (onboarding.view?.trigger === 'redo') return rest[0] ?? null;
  return rest.find((s) => statusOf(s) === 'todo') ?? null;
}

/** The station before `step`. */
export function before(step: OnboardingStep): OnboardingStep | null {
  const list = stations();
  return list[list.indexOf(step) - 1] ?? null;
}

/** `step` done or passed, then the next station, or the end. */
export async function leave(step: OnboardingStep, outcome: 'done' | 'skipped', choice?: OnboardingChoice): Promise<void> {
  await act(choice ? { kind: 'step', step, outcome, choice } : { kind: 'step', step, outcome });
  const next = after(step);
  if (next) onboarding.at = next;
  else await end();
}

/** The last station passed: the screen of the end. */
export async function end(): Promise<void> {
  await act({ kind: 'finish' });
  onboarding.at = null;
}

/** « Tout passer »: every station not done passed, the journey ended and closed. */
export async function skipAll(): Promise<void> {
  await act({ kind: 'skipAll' });
  close();
}

/** Closed: the app on screen. */
export function close(): void {
  onboarding.shown = false;
  onboarding.at = null;
}

/** « Refaire le premier lancement » (Aide, Réglages › Général): every station shown from the first, nothing undone. */
export async function redo(): Promise<void> {
  const v = await act({ kind: 'redo' });
  if (!v) return;
  onboarding.engine = null;
  onboarding.project = null;
  onboarding.at = v.resumeAt ?? stations()[0] ?? null;
  onboarding.shown = true;
}

/** What the foot's main button does at a station: its words, and the gesture (true: the station is done, go on). */
export interface Primary {
  label: string;
  disabled?: boolean;
  /** At a station that recommends a choice: whether the one taken is it (the funnel of the statistics, ESC-507). */
  choice?: OnboardingChoice;
  run: () => Promise<boolean>;
  /** The words of « Plus tard » at this station (the features': « Passer : garder l'essentiel »). */
  later?: string;
  /** « Retour » within the station first (a screen of the features before): true when it went back there. */
  back?: () => boolean;
}
