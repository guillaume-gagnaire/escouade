import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { overseer } from '../../lib/overseer.svelte';
import { app } from '../../lib/state.svelte';
import type { OverseerDirective, OverseerView } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const DIRECTIVE: OverseerDirective = {
  id: 'c-2',
  summary: 'Fusionner les tickets du portail',
  actions: ['merge'],
  scope: { projectId: 'p1' },
  source: { message: 'm-3', quote: 'tu peux merger sur le portail', at: 1 },
  createdAt: 1,
  project: 'portail',
  ticket: null,
  message: 'Tu peux merger sur le portail.',
};

const VIEW: OverseerView = {
  enabled: true,
  running: true,
  needsMcp: false,
  agentId: 'ov',
  directives: [DIRECTIVE],
  actions: [],
  messages: [],
};

describe('Réglages › Overseer (ESC-462)', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', page: 'overseer' };
  });

  it('turns it on with Escouade’s tools, on Haiku by default, and saves at once', async () => {
    const b = fakeBackend({ save_settings: () => [], mcp_set_enabled: () => null });
    render(SettingsModal, { page: 'overseer' });
    const panel = within(document.getElementById('settings-panel')!);
    expect(panel.getByRole('combobox', { name: 'Modèle' })).toHaveValue('haiku');
    expect(panel.getByRole('combobox', { name: 'Compte' })).toHaveValue('');
    expect(panel.getByText('Aucune consigne : il te demande avant chaque action qui sort.')).toBeInTheDocument();
    await userEvent.click(panel.getByRole('switch', { name: 'Activer l’Overseer' }));
    // Escouade's tools first, by their own call.
    expect(b.called('mcp_set_enabled')[0].args).toEqual({ enabled: true });
    await waitFor(() => expect(b.called('save_settings')).toHaveLength(1));
    expect(b.called('save_settings')[0].args.settings).toMatchObject({
      mcpEnabled: true,
      overseer: { enabled: true, engine: 'claude', model: 'haiku', account: '', effort: '' },
    });
  });

  it('lists the instructions with where they come from, and withdraws one by a click', async () => {
    const b = fakeBackend({ save_settings: () => [] });
    overseer.take(VIEW);
    render(SettingsModal, { page: 'overseer' });
    const item = document.querySelector('[data-directive="c-2"]') as HTMLElement;
    expect(item).toHaveTextContent('Fusionner les tickets du portail');
    expect(item).toHaveTextContent('fusionner · projet portail');
    expect(item).toHaveTextContent('Née de ton message : « tu peux merger sur le portail »');
    await userEvent.click(screen.getByRole('button', { name: 'Retirer la consigne « Fusionner les tickets du portail »' }));
    expect(b.called('overseer_revoke')[0].args).toEqual({ id: 'c-2' });
  });
});

describe('Réglages › Overseer › Raccourci global (ESC-468)', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'settings', page: 'overseer' };
  });

  it('shows the default, records a new one, registers it at once and keeps it', async () => {
    const b = fakeBackend({ save_settings: () => [], overseer_shortcut_set: () => null });
    overseer.take({ ...VIEW, shortcut: { active: 'Alt+Shift+KeyO', error: null, unsupported: false } });
    render(SettingsModal, { page: 'overseer' });
    const panel = within(document.getElementById('settings-panel')!);
    const shown = document.querySelector('[data-overseer-shortcut]') as HTMLElement;
    expect(shown).toHaveTextContent('Alt+Maj+O');
    expect(panel.getByText('Actif')).toBeInTheDocument();
    await userEvent.click(panel.getByRole('button', { name: 'Changer' }));
    expect(shown).toHaveTextContent('Appuie sur le raccourci…');
    await userEvent.keyboard('{Control>}{Alt>}j{/Alt}{/Control}');
    await waitFor(() => expect(b.called('overseer_shortcut_set')[0]?.args).toEqual({ shortcut: 'Mod+Alt+KeyJ' }));
    await waitFor(() => expect(shown).toHaveTextContent('Ctrl+Alt+J'));
    // Back to the default, or none at all.
    await userEvent.click(panel.getByRole('button', { name: 'Par défaut' }));
    expect(b.called('overseer_shortcut_set')[1].args).toEqual({ shortcut: 'Alt+Shift+KeyO' });
    await userEvent.click(panel.getByRole('button', { name: 'Retirer le raccourci' }));
    expect(b.called('overseer_shortcut_set')[2].args).toEqual({ shortcut: '' });
  });

  it('says why a shortcut another app holds is refused, and keeps the one before', async () => {
    fakeBackend({
      save_settings: () => [],
      overseer_shortcut_set: () => {
        throw 'Ce raccourci est déjà pris par une autre app ou par le système (HotKey already registered) : choisis-en un autre.';
      },
    });
    render(SettingsModal, { page: 'overseer' });
    const panel = within(document.getElementById('settings-panel')!);
    await userEvent.click(panel.getByRole('button', { name: 'Changer' }));
    await userEvent.keyboard('{Control>}{Alt>}k{/Alt}{/Control}');
    await waitFor(() => expect(document.querySelector('[data-shortcut-error]')).toHaveTextContent('déjà pris'));
    expect(document.querySelector('[data-overseer-shortcut]')).toHaveTextContent('Alt+Maj+O');
  });

  it('says what to do under Wayland, where an app has no global shortcut', () => {
    fakeBackend({ save_settings: () => [] });
    overseer.take({ ...VIEW, shortcut: { active: null, error: null, unsupported: true } });
    render(SettingsModal, { page: 'overseer' });
    expect(document.querySelector('[data-shortcut-wayland]')).toHaveTextContent('xdg-open escouade://overseer');
  });
});
