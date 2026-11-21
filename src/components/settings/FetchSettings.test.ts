import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import { afterGesture } from '../../test/settings';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// « Récupérer automatiquement (git fetch) » (ESC-561): the general setting in Tous les projets › Git, a project's own in
// its Git page; free values and offered ones, within their bounds, the window saying it brought one back.
describe('the background fetch settings', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.modal = { kind: 'settings' };
  });
  afterEach(() => {
    cleanup();
    setLang('fr');
  });

  const saved = (backend: ReturnType<typeof fakeBackend>) => backend.called('save_settings').at(-1)!.args.settings.gitFetch;

  it('is on, every 5 min within 5 min, by default; switched off, it is saved off', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'allProjects' });
    const toggle = screen.getByRole('switch', { name: 'Récupérer automatiquement (git fetch)' });
    expect(toggle).toBeChecked();
    expect(screen.getByRole('textbox', { name: 'Intervalle' })).toHaveValue('5');
    expect(screen.getByRole('textbox', { name: 'Délai d’attente' })).toHaveValue('300');
    await userEvent.click(toggle);
    await afterGesture();
    expect(saved(backend)).toEqual({ auto: false, everyMin: 5, timeoutSecs: 300 });
  });

  it('takes an offered interval or a free one, and brings one out of its bounds back to them, saying so', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'allProjects' });
    const offered = screen.getByRole('group', { name: 'Intervalle' });
    expect(
      within(offered)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['2 min', '5 min', '15 min', '30 min', '1 h']);
    expect(within(offered).getByRole('button', { name: '5 min' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(offered).getByRole('button', { name: '15 min' }));
    await afterGesture();
    expect(saved(backend).everyMin).toBe(15);
    const field = screen.getByRole('textbox', { name: 'Intervalle' });
    expect(field).toHaveValue('15');
    // A free value.
    await userEvent.clear(field);
    await userEvent.type(field, '45{Enter}');
    await afterGesture();
    expect(saved(backend).everyMin).toBe(45);
    expect(within(offered).queryByRole('button', { pressed: true })).toBeNull();
    // Past 24 h: 24 h, said under it.
    await userEvent.clear(field);
    await userEvent.type(field, '5000{Enter}');
    await afterGesture();
    expect(saved(backend).everyMin).toBe(1440);
    expect(field).toHaveValue('1440');
    expect(screen.getByRole('status')).toHaveTextContent('5000 est hors des bornes (1 min à 24 h) : ramené à 24 h.');
    // Under 1 min: 1 min.
    await userEvent.clear(field);
    await userEvent.type(field, '0{Enter}');
    await afterGesture();
    expect(saved(backend).everyMin).toBe(1);
    // Emptied, or not a number: the value it had stays.
    await userEvent.clear(field);
    await userEvent.tab();
    expect(field).toHaveValue('1');
  });

  it('brings a time limit under 30 s or past 30 min back to its bound', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'allProjects' });
    const field = screen.getByRole('textbox', { name: 'Délai d’attente' });
    await userEvent.clear(field);
    await userEvent.type(field, '10{Enter}');
    await afterGesture();
    expect(saved(backend).timeoutSecs).toBe(30);
    expect(screen.getByRole('status')).toHaveTextContent('10 est hors des bornes (30 s à 30 min) : ramené à 30 s.');
    await userEvent.click(within(screen.getByRole('group', { name: 'Délai d’attente' })).getByRole('button', { name: '10 min' }));
    await afterGesture();
    expect(saved(backend).timeoutSecs).toBe(600);
  });

  it('a project follows the general setting, or says always or never, with its own interval and time limit', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    const modes = screen.getByRole('group', { name: 'Récupérer automatiquement (git fetch)' });
    expect(within(modes).getByRole('button', { name: 'Comme le réglage général' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText(/Réglage général : toutes les 5 min, délai de 5 min\./)).toBeInTheDocument();
    // Its own interval: empty is the general one's, shown in its place.
    const every = screen.getByRole('textbox', { name: 'Intervalle propre au projet' });
    expect(every).toHaveValue('');
    expect(every).toHaveAttribute('placeholder', '5');
    await userEvent.type(every, '60{Enter}');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.gitFetch).toEqual({ mode: 'default', everyMin: 60, timeoutSecs: 0 });
    // Emptied again: the general one's.
    await userEvent.clear(every);
    await userEvent.tab();
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.gitFetch.everyMin).toBe(0);
    // Its own time limit, out of bounds: brought back.
    const timeout = screen.getByRole('textbox', { name: 'Délai d’attente propre au projet' });
    await userEvent.type(timeout, '4000{Enter}');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.gitFetch.timeoutSecs).toBe(1800);
    // Never: its interval and its limit are gone.
    await userEvent.click(within(modes).getByRole('button', { name: 'Jamais' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.gitFetch.mode).toBe('never');
    expect(screen.queryByRole('textbox', { name: 'Intervalle propre au projet' })).toBeNull();
    await userEvent.click(within(modes).getByRole('button', { name: 'Toujours' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.gitFetch.mode).toBe('always');
  });

  it('says the general setting off, and opens it', async () => {
    fakeBackend();
    app.settings.gitFetch = { auto: false, everyMin: 5, timeoutSecs: 300 };
    render(SettingsModal, { page: 'git', projectId: 'p1' });
    expect(screen.getByText(/Réglage général : coupé\./)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Réglage général…' }));
    expect(screen.getByRole('switch', { name: 'Récupérer automatiquement (git fetch)' })).not.toBeChecked();
  });

  it('is in English too', () => {
    fakeBackend();
    setLang('en');
    render(SettingsModal, { page: 'allProjects' });
    expect(screen.getByRole('switch', { name: 'Fetch automatically (git fetch)' })).toBeChecked();
    expect(screen.getByRole('textbox', { name: 'Time limit' })).toHaveValue('300');
  });
});
