import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { FEATURES } from '../../lib/features';
import { reviewTour, takeTour, tour } from '../../lib/features-tour.svelte';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { app } from '../../lib/state.svelte';
import type { Features, FeaturesTourView, Locks } from '../../lib/types';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import UpdateModal from '../modals/UpdateModal.svelte';
import FeatureTour from './FeatureTour.svelte';
import FeaturesTourModal from './FeaturesTourModal.svelte';

// The assistant of the features (F-2, ESC-559): one feature per screen, its capture, three points and « Activer »; what is
// used said; the lock of a device or a team; « Passer » that changes nothing; « Revoir la présentation »; the window of the
// novelties.

const ALL_OFF = Object.fromEntries(FEATURES.map((f) => [f, false])) as Features;
const NO_LOCKS: Locks = { locks: {}, rows: {}, warnings: [] };
const VIEW: FeaturesTourView = { open: true, used: [{ feature: 'kanban', projects: 2 }], new: [] };

/** A core that saves the settings and keeps the assistant's view. */
function core(view: FeaturesTourView = VIEW) {
  let v = view;
  return fakeBackend({
    save_settings: () => [],
    features_tour_view: () => v,
    features_tour_act: ({ act }) => {
      if (act.kind === 'end') v = { ...v, open: false };
      if (act.kind === 'met') v = { ...v, new: v.new.filter((f) => !act.features.includes(f)) };
      return v;
    },
  });
}

const theSwitch = () => screen.getByRole('switch', { name: /^(Activer|Turn on)/ });

describe('the assistant of the features (F-2, ESC-559)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    setLocks(NO_LOCKS);
    app.settings = { ...SETTINGS, features: { ...ALL_OFF, kanban: true } };
    tour.view = null;
    tour.shown = false;
  });

  it('shows one feature per screen: its capture, what it does in three points and a big switch « Activer »', async () => {
    core();
    render(FeatureTour, { list: FEATURES, mode: 'first', used: VIEW.used });
    expect(screen.getByRole('heading', { level: 2, name: 'Le Kanban' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Capture d’Escouade : Kanban' })).toHaveAttribute('src', '/features/fr/kanban-dark.jpg');
    const screenOf = screen.getByRole('region', { name: 'Le Kanban' });
    expect(within(screenOf).getAllByRole('listitem')).toHaveLength(3);
    expect(theSwitch()).toHaveAccessibleName('Activer le Kanban');
    expect(theSwitch()).toHaveAttribute('aria-checked', 'true');
    // What it takes with it describes the switch, for a screen reader.
    expect(theSwitch()).toHaveAccessibleDescription(/Ses tickets restent/);
    // The features as chips, the essential first and never a button; the one shown is the current step.
    const chips = within(screen.getByRole('navigation', { name: 'Les fonctionnalités' }));
    expect(chips.getAllByRole('button')).toHaveLength(9);
    expect(chips.getByRole('button', { name: 'Kanban' })).toHaveAttribute('aria-current', 'step');
    expect(screen.getByLabelText('Fonctionnalité 1 sur 9')).toHaveTextContent('1 / 9');
    // ← → go from one screen to the next.
    theSwitch().focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByRole('heading', { level: 2, name: 'L’éditeur' })).toBeInTheDocument();
    expect(theSwitch()).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(chips.getByRole('button', { name: 'Voix' }));
    expect(screen.getByRole('heading', { level: 2, name: 'La voix' })).toBeInTheDocument();
  });

  it('switches a feature on as Réglages › Fonctionnalités saves it', async () => {
    const b = core();
    render(FeatureTour, { list: FEATURES, at: 2, mode: 'first' });
    expect(screen.getByRole('heading', { level: 2, name: 'L’espace Docs' })).toBeInTheDocument();
    await userEvent.click(theSwitch());
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(1));
    expect(b.called('save_settings')[0].args.settings.features).toEqual({ ...ALL_OFF, kanban: true, docs: true });
    await waitFor(() => expect(theSwitch()).toHaveAttribute('aria-checked', 'true'));
  });

  it('says what is used: it stays on at the first launch and the update, used when seen again', () => {
    core();
    const { unmount } = render(FeatureTour, { list: FEATURES, mode: 'update', used: VIEW.used });
    expect(screen.getByText('Tu l’utilises déjà sur 2 projets : il reste allumé.')).toBeInTheDocument();
    unmount();
    render(FeatureTour, { list: FEATURES, mode: 'review', used: VIEW.used });
    expect(screen.getByText('Utilisée sur 2 projets')).toBeInTheDocument();
  });

  it('shows a feature a device or a team imposes locked, with who imposes it', () => {
    core();
    setLocks({
      locks: { 'features.kanban': { key: 'features.kanban', value: true, by: { kind: 'team', team: 'Acme' }, bounds: [] } },
      rows: { featureKanban: ['features.kanban'] },
      warnings: [],
    });
    render(FeatureTour, { list: FEATURES, mode: 'first' });
    expect(theSwitch()).toBeDisabled();
    expect(screen.getByText(/Imposé par l’équipe Acme/)).toBeInTheDocument();
  });

  it('says the scheduled tasks go with the Kanban, their switch closed while it is off', () => {
    core();
    app.settings = { ...SETTINGS, features: { ...ALL_OFF } };
    render(FeatureTour, { list: FEATURES, at: 6, mode: 'first' });
    expect(screen.getByRole('heading', { level: 2, name: 'Les tâches programmées' })).toBeInTheDocument();
    expect(theSwitch()).toBeDisabled();
    expect(screen.getByText('Part avec « Kanban » : rallume-le d’abord.')).toBeInTheDocument();
  });

  it('after the update, « Passer : garder l’essentiel » changes no switch and never opens it again', async () => {
    const b = core();
    takeTour(VIEW);
    expect(tour.shown).toBe(true);
    expect(tour.mode).toBe('update');
    render(FeaturesTourModal);
    const dialog = screen.getByRole('dialog', { name: 'Choisis tes fonctionnalités' });
    expect(dialog).toHaveTextContent('Ce que tu utilises reste allumé');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suivant' }));
    expect(screen.getByRole('heading', { level: 2, name: 'L’éditeur' })).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Passer : garder l’essentiel' }));
    await waitFor(() => expect(tour.shown).toBe(false));
    expect(b.called('features_tour_act')[0].args.act).toEqual({ kind: 'end' });
    expect(b.called('save_settings')).toHaveLength(0);
    // Its view now says it is done: taken again, it does not open.
    takeTour(tour.view);
    expect(tour.shown).toBe(false);
  });

  it('never opens by itself over the first launch, whose station it is', () => {
    core();
    takeTour(VIEW, true);
    expect(tour.shown).toBe(false);
  });

  it('ends on « Terminer » at its last screen', async () => {
    const b = core();
    takeTour(VIEW);
    render(FeaturesTourModal);
    const dialog = screen.getByRole('dialog');
    for (let i = 1; i < FEATURES.length; i++) await userEvent.click(within(dialog).getByRole('button', { name: 'Suivant' }));
    expect(screen.getByRole('heading', { level: 2, name: 'La voix' })).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Terminer' }));
    await waitFor(() => expect(tour.shown).toBe(false));
    expect(b.called('features_tour_act')).toHaveLength(1);
  });

  it('« Revoir la présentation » closes the settings and opens it, its « Passer » a « Fermer »', async () => {
    core({ ...VIEW, open: false });
    app.modal = { kind: 'settings', page: 'features' };
    await reviewTour();
    expect(app.modal).toBeNull();
    expect(tour.shown).toBe(true);
    expect(tour.mode).toBe('review');
    render(FeaturesTourModal);
    const dialog = screen.getByRole('dialog', { name: 'Les fonctionnalités d’Escouade' });
    expect(within(dialog).queryByRole('button', { name: /Passer/ })).toBeNull();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Fermer la présentation' }));
    await waitFor(() => expect(tour.shown).toBe(false));
  });

  it('gives a feature an update brought its screen in the window of the novelties, then it is met', async () => {
    const b = core({ open: false, used: [], new: ['voice'] });
    takeTour({ open: false, used: [], new: ['voice'] });
    render(UpdateModal, { version: '2.4.0', notes: '- Une nouveauté', installed: true });
    expect(screen.getByRole('heading', { name: 'Nouvelle fonctionnalité' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: 'La voix' })).toBeInTheDocument();
    expect(theSwitch()).toHaveAccessibleName('Activer la voix');
    await waitFor(() => expect(b.called('features_tour_act')[0]?.args.act).toEqual({ kind: 'met', features: ['voice'] }));
    await waitFor(() => expect(tour.view?.new).toEqual([]));
  });

  it('is in English too', () => {
    setLang('en');
    core();
    render(FeatureTour, { list: FEATURES, mode: 'update', used: VIEW.used });
    expect(screen.getByRole('heading', { level: 2, name: 'The Kanban' })).toBeInTheDocument();
    expect(theSwitch()).toHaveAccessibleName('Turn on the Kanban');
    expect(screen.getByText('You already use it on 2 projects: it stays on.')).toBeInTheDocument();
    expect(screen.getByRole('img')).toHaveAttribute('src', '/features/en/kanban-dark.jpg');
  });
});
