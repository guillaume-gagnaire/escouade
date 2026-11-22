import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { FEATURES } from '../../lib/features';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import { app } from '../../lib/state.svelte';
import type { Features, Locks } from '../../lib/types';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

const ALL_ON = Object.fromEntries(FEATURES.map((f) => [f, true])) as Features;
const NO_LOCKS: Locks = { locks: {}, rows: {}, warnings: [] };

const panel = () => within(document.getElementById('settings-panel')!);
const rowOf = (f: string) => document.querySelector(`[data-feature="${f}"]`) as HTMLElement;

describe('Réglages › Fonctionnalités (F-1, ESC-550)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    setLocks(NO_LOCKS);
    app.settings = { ...SETTINGS, features: { ...ALL_ON } };
  });

  it('lists the essential, never off, then each feature with what it does and what it takes with it', () => {
    fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'features' });
    expect(screen.getByRole('heading', { name: 'Fonctionnalités' })).toBeInTheDocument();
    const essential = panel().getByRole('switch', { name: 'Agents et conversation' });
    expect(essential).toBeDisabled();
    expect(essential).toHaveAttribute('aria-checked', 'true');
    const names = panel()
      .getAllByRole('switch')
      .map((s) => s.getAttribute('aria-label'));
    expect(names).toEqual([
      'Agents et conversation',
      'Kanban',
      'Éditeur',
      'Espace Docs',
      'Coût',
      'Statistiques',
      'Workflows',
      'Tâches programmées',
      'Overseer',
      'Voix',
    ]);
    expect(rowOf('kanban')).toHaveTextContent('Les tickets et leurs critères, le pilote auto, les sources de tickets.');
    expect(rowOf('docs')).toHaveTextContent(
      'Coupé : son onglet et ses outils MCP disparaissent. Les pages, décisions et documents restent.',
    );
    // What it takes is what the switch describes, for a screen reader.
    const docs = panel().getByRole('switch', { name: 'Espace Docs' });
    expect(document.getElementById(docs.getAttribute('aria-describedby')!)).toHaveTextContent('ses outils MCP disparaissent');
    // « Revoir la présentation » (F-2, ESC-559): the assistant again.
    expect(panel().getByRole('button', { name: 'Revoir la présentation' })).toBeInTheDocument();
  });

  it('switches a feature off and saves it at once', async () => {
    const b = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'features' });
    await userEvent.click(panel().getByRole('switch', { name: 'Espace Docs' }));
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(1));
    expect(b.called('save_settings')[0].args.settings.features).toEqual({ ...ALL_ON, docs: false });
  });

  it('saves a second switch of the same group after the first one', async () => {
    const b = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'features' });
    await userEvent.click(panel().getByRole('switch', { name: 'Kanban' }));
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(1));
    await userEvent.click(panel().getByRole('switch', { name: 'Espace Docs' }));
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(2));
    expect(b.called('save_settings')[1].args.settings.features).toEqual({ ...ALL_ON, kanban: false, docs: false });
  });

  it('greys « Tâches programmées » while the Kanban is off: it goes with it', () => {
    fakeBackend({ save_settings: () => [] });
    app.settings = { ...SETTINGS, features: { ...ALL_ON, kanban: false } };
    render(SettingsModal, { page: 'features' });
    const sched = panel().getByRole('switch', { name: 'Tâches programmées' });
    expect(sched).toBeDisabled();
    expect(rowOf('scheduled')).toHaveTextContent('Part avec « Kanban » : rallume-le d’abord.');
  });

  it('greys a feature a policy imposes, with who imposes it', () => {
    fakeBackend({ save_settings: () => [] });
    app.settings = { ...SETTINGS, features: { ...ALL_ON, voice: false } };
    setLocks({
      locks: {
        'features.voice': { key: 'features.voice', value: false, by: { kind: 'team', team: 'Acme' }, bounds: [] },
      },
      rows: { featureVoice: ['features.voice'] },
      warnings: [],
    });
    render(SettingsModal, { page: 'features' });
    const voice = rowOf('voice');
    expect(voice).toHaveTextContent('Imposé par l’équipe Acme');
    expect(voice.querySelector('.ctl')).toHaveAttribute('aria-disabled', 'true');
    expect(within(voice).getByRole('switch', { name: 'Voix' })).toBeDisabled();
  });

  it('says what the installation already used when it was updated', () => {
    fakeBackend({ save_settings: () => [] });
    app.settings = {
      ...SETTINGS,
      features: { ...ALL_ON },
      featuresUsed: [
        { feature: 'kanban', projects: 2 },
        { feature: 'voice', projects: 0 },
      ],
    };
    render(SettingsModal, { page: 'features' });
    expect(rowOf('kanban')).toHaveTextContent('Utilisée sur 2 projets');
    expect(rowOf('voice')).toHaveTextContent('Utilisée');
    expect(rowOf('docs')).not.toHaveTextContent('Utilisée');
  });

  it('takes the pages of a feature off out of the bar', () => {
    fakeBackend({ save_settings: () => [] });
    app.settings = { ...SETTINGS, features: { ...ALL_ON, overseer: false, voice: false } };
    render(SettingsModal, { page: 'features' });
    const nav = screen.getByRole('navigation');
    expect(within(nav).queryByRole('button', { name: /Overseer/ })).toBeNull();
    expect(within(nav).queryByRole('button', { name: /^Voix/ })).toBeNull();
    expect(within(nav).getByRole('button', { name: /Fonctionnalités/ })).toBeInTheDocument();
  });
});
