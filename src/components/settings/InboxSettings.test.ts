import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The settings' other tabs have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import { afterGesture } from '../../test/settings';
import SettingsModal from '../modals/SettingsModal.svelte';

// The box « À toi » in the settings (ESC-510, docs/INBOX.md § 6.3, § 10.5): under « Les agents peuvent piloter Escouade »,
// greyed with the server off; « Garder les fichiers des demandes closes » (7, 30, 90 days); what its files weigh, in
// « Avancé › Données locales », and « Vider les fichiers des demandes closes ».

const backend = (over: Record<string, (a: any) => unknown> = {}) =>
  fakeBackend({
    mcp_status: () => ({ running: false, port: 0, error: null }),
    mcp_declare_status: () => [],
    mcp_activity: () => [],
    save_settings: () => [],
    inbox_weight: () => 212 * 1024 * 1024,
    inbox_erase_closed_files: () => 12 * 1024 * 1024,
    ...over,
  });

/** The question before turning the server off, as the window opens it. */
interface Asked {
  title: string;
  body: string;
  alt?: { label: string };
  onCancel?: () => void;
  onConfirm: (option: boolean) => void | Promise<void>;
}

describe('the settings of the box « À toi »', () => {
  beforeEach(() => resetApp());

  it('says the box is off with the server, its files kept greyed', async () => {
    backend();
    app.settings = { ...SETTINGS, mcpEnabled: false };
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    const box = await screen.findByTestId('inbox-settings');
    expect(box).toHaveTextContent('La boîte « À toi »');
    expect(box).toHaveTextContent('Éteinte avec le serveur');
    expect(within(box).queryByRole('button', { name: /Ouvrir la boîte/ })).toBeNull();
    expect(within(box).getByRole('button', { name: '7 jours' })).toBeDisabled();
  });

  it('keeps the files of closed requests 7, 30 or 90 days, saved as chosen, and opens the box', async () => {
    const be = backend();
    app.settings = { ...SETTINGS, mcpEnabled: true };
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    const box = await screen.findByTestId('inbox-settings');
    expect(within(box).getByRole('button', { name: '30 jours' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(box).getByRole('button', { name: '7 jours' }));
    await afterGesture();
    await waitFor(() => expect(be.called('save_settings').at(-1)?.args.settings.inboxKeepDays).toBe(7));
    await userEvent.click(within(box).getByRole('button', { name: /Ouvrir la boîte/ }));
    expect(app.modal).toBeNull();
    expect(app.ui.view).toBe('inbox');
  });

  it('says what the box’s files weigh, and empties those of closed requests', async () => {
    const be = backend();
    app.settings = { ...SETTINGS, mcpEnabled: true };
    app.modal = { kind: 'settings', page: 'advanced' };
    render(SettingsModal, { page: 'advanced' });
    expect(await screen.findByTestId('inbox-weight')).toHaveTextContent('212 Mo');
    await userEvent.click(screen.getByRole('button', { name: 'Vider les fichiers des demandes closes' }));
    await waitFor(() => expect(be.called('inbox_erase_closed_files')).toHaveLength(1));
    await waitFor(() => expect(app.toasts.at(-1)?.text).toBe('12 Mo libérés'));
  });

  it('asks before turning the server off with requests waiting, which are kept until it is on again', async () => {
    const be = backend({
      inbox_closing: () => ({ open: 4, titles: ['Choisis la direction', 'Publier 1.4.0 ?', 'Essaie le lien'] }),
      mcp_set_enabled: () => null,
    });
    app.settings = { ...SETTINGS, mcpEnabled: true };
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    await userEvent.click(screen.getByRole('switch', { name: 'Les agents peuvent piloter Escouade' }));
    await waitFor(() => expect(app.modal?.kind).toBe('confirm'));
    const ask = app.modal as unknown as Asked;
    expect(ask.title).toBe('Éteindre le serveur MCP ?');
    expect(ask.body).toContain('4 demandes de tes agents t’attendent');
    expect(ask.body).toContain('• Publier 1.4.0 ?');
    expect(ask.body).toContain('et 1 autre');
    expect(ask.alt?.label).toBe('Ouvrir la boîte');
    expect(be.called('mcp_set_enabled')).toHaveLength(0);
    // « Annuler » goes back to the settings, the server on.
    ask.onCancel!();
    expect(app.modal?.kind).toBe('settings');
    await userEvent.click(screen.getByRole('switch', { name: 'Les agents peuvent piloter Escouade' }));
    await waitFor(() => expect(app.modal?.kind).toBe('confirm'));
    await (app.modal as unknown as Asked).onConfirm(false);
    await waitFor(() => expect(be.called('mcp_set_enabled').map((c) => c.args)).toEqual([{ enabled: false }]));
    await waitFor(() => expect(app.settings.mcpEnabled).toBe(false));
  });

  it('turns the server off without asking when nothing waits', async () => {
    const be = backend({ inbox_closing: () => ({ open: 0, titles: [] }), mcp_set_enabled: () => null });
    app.settings = { ...SETTINGS, mcpEnabled: true };
    app.modal = { kind: 'settings', page: 'agents' };
    render(SettingsModal, { page: 'agents' });
    await userEvent.click(screen.getByRole('switch', { name: 'Les agents peuvent piloter Escouade' }));
    await waitFor(() => expect(be.called('mcp_set_enabled')).toHaveLength(1));
    expect(app.modal?.kind).toBe('settings');
  });
});
