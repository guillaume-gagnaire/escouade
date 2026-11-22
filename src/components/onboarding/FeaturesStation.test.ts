import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { FEATURES } from '../../lib/features';
import { tour } from '../../lib/features-tour.svelte';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { onboarding, takeView } from '../../lib/onboarding.svelte';
import { app } from '../../lib/state.svelte';
import type { Features, OnboardingStep, OnboardingView, StepStatus } from '../../lib/types';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import Onboarding from './Onboarding.svelte';

// The station of the features in the first launch's line (F-2, ESC-559): after the first agent, before the statistics;
// « Suivant » screen by screen, « Retour » a screen back first, « Passer : garder l'essentiel » passes it with no switch
// changed; the line says how many are on.

const STEPS: OnboardingStep[] = ['harness', 'account', 'defaults', 'hookups', 'project', 'agent', 'features', 'telemetry'];
const ALL_OFF = Object.fromEntries(FEATURES.map((f) => [f, false])) as Features;

function view(features: StepStatus = 'todo'): OnboardingView {
  return {
    open: true,
    trigger: 'first',
    steps: STEPS.map((id) => ({ id, status: id === 'features' ? features : id === 'telemetry' ? 'todo' : 'done' })),
    resumeAt: 'features',
    started: true,
    finished: false,
    askTelemetryAgain: false,
    hookups: [],
    demo: '',
  };
}

function core(start: OnboardingView) {
  let v = start;
  return fakeBackend({
    onboarding_view: () => v,
    onboarding_act: ({ act }) => {
      if (act.kind === 'step') v = { ...v, steps: v.steps.map((s) => (s.id === act.step ? { ...s, status: act.outcome } : s)) };
      return v;
    },
    save_settings: () => [],
    features_tour_view: () => ({ open: false, used: [{ feature: 'voice', projects: 0 }], new: [] }),
    features_tour_act: () => ({ open: false, used: [], new: [] }),
  });
}

const line = () => within(screen.getByRole('navigation', { name: /stations/ }));

describe('the station of the features (F-2, ESC-559)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    setLocks({ locks: {}, rows: {}, warnings: [] });
    app.settings = { ...SETTINGS, features: { ...ALL_OFF, voice: true }, telemetry: null };
    tour.view = null;
  });

  it('is a station of the line between the first agent and the statistics, one feature per screen', async () => {
    const b = core(view());
    takeView(view());
    render(Onboarding);
    const names = line()
      .getAllByRole('listitem')
      .map((li) => li.querySelector('.nm')?.textContent);
    expect(names.slice(5, 8)).toEqual(['Premier agent', 'Fonctionnalités', 'Statistiques']);
    expect(screen.getByText('8 stations · environ 3 minutes')).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1, name: 'Que veux-tu voir autour de tes agents ?' })).toHaveFocus();
    expect(screen.getByRole('heading', { level: 2, name: 'Le Kanban' })).toBeInTheDocument();
    // The voice plugged in at « Ce que tu veux brancher »: used, it stays on.
    await userEvent.click(screen.getByRole('button', { name: 'Voix' }));
    expect(await screen.findByText('Tu l’utilises déjà : elle reste allumée.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Kanban' }));
    // « Suivant » goes a screen on, « Retour » a screen back before it goes back a station.
    await userEvent.click(screen.getByRole('button', { name: /^Suivant/ }));
    expect(screen.getByRole('heading', { level: 2, name: 'L’éditeur' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Retour/ }));
    expect(screen.getByRole('heading', { level: 2, name: 'Le Kanban' })).toBeInTheDocument();
    expect(onboarding.at).toBe('features');
    // Switched on, then the line says how many are on once it is done.
    await userEvent.click(screen.getByRole('switch', { name: 'Activer le Kanban' }));
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(1));
    for (let i = 1; i < FEATURES.length; i++) await userEvent.click(screen.getByRole('button', { name: /^Suivant/ }));
    await userEvent.click(screen.getByRole('button', { name: /^Continuer/ }));
    await waitFor(() => expect(onboarding.at).toBe('telemetry'));
    expect(b.called('features_tour_act')[0].args.act).toEqual({ kind: 'end' });
    expect(b.called('onboarding_act').at(-1)?.args.act).toMatchObject({ kind: 'step', step: 'features', outcome: 'done' });
    expect(line().getByRole('button', { name: /Fonctionnalités/ })).toHaveTextContent('2 allumées');
  });

  it('« Passer : garder l’essentiel » passes it with no switch changed', async () => {
    const b = core(view());
    takeView(view());
    render(Onboarding);
    await userEvent.click(screen.getByRole('button', { name: 'Passer : garder l’essentiel' }));
    await waitFor(() => expect(onboarding.at).toBe('telemetry'));
    expect(b.called('onboarding_act').at(-1)?.args.act).toMatchObject({ step: 'features', outcome: 'skipped' });
    expect(b.called('save_settings')).toHaveLength(0);
  });

  it('goes by the keyboard: ↓ to the switch, Espace switches, Entrée is « Suivant », Échap passes', async () => {
    const b = core(view());
    takeView(view());
    render(Onboarding);
    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getByRole('switch', { name: 'Activer le Kanban' })).toHaveFocus();
    await userEvent.keyboard(' ');
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(1));
    screen.getByRole('heading', { level: 1 }).focus();
    await userEvent.keyboard('{Enter}');
    expect(screen.getByRole('heading', { level: 2, name: 'L’éditeur' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(onboarding.at).toBe('telemetry'));
  });

  it('every feature imposed: shown locked, crossed in one « Continuer »', async () => {
    core(view('policy'));
    takeView(view('policy'));
    onboarding.at = 'features';
    render(Onboarding);
    expect(screen.getByText(/Toutes les fonctionnalités sont imposées sur ce poste/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Continuer/ }));
    await waitFor(() => expect(onboarding.at).toBe('telemetry'));
  });
});
