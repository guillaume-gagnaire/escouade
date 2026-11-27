// Réglages › Agents › « Pages des agents » (ESC-673, docs/AGENT-PAGES.md § 9): under « Les agents peuvent piloter
// Escouade », beside « Agents et navigateurs », on by default, saved with the draft, greyed while the server is off.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp, SETTINGS } from '../../test/ipc';
import McpSection from './McpSection.svelte';

beforeEach(() => {
  setLang('fr');
  resetApp();
  fakeBackend({ mcp_status: () => ({ running: false, port: 0, error: null }), mcp_declare_status: () => [] });
});

describe('the switch « Pages des agents »', () => {
  it('is on by default, after « Agents et navigateurs », saved with the draft, greyed while the server is off', async () => {
    app.settings = { ...SETTINGS, mcpEnabled: true };
    settingsForm.settings = { ...SETTINGS, mcpEnabled: true };
    const { unmount } = render(McpSection);
    const switches = screen.getAllByRole('switch').map((s) => s.getAttribute('aria-label'));
    expect(switches.indexOf('Pages des agents')).toBe(switches.indexOf('Agents et navigateurs') + 1);
    const sw = screen.getByRole('switch', { name: 'Pages des agents' });
    expect(sw).toHaveAttribute('aria-checked', 'true');
    expect(sw).toBeEnabled();
    expect(screen.getByText(/sans réseau, sans accès à Escouade ni à tes fichiers/)).toBeInTheDocument();
    await userEvent.click(sw);
    expect(settingsForm.settings.agentPages).toBe(false);
    // « Agents et navigateurs » is its own: untouched.
    expect(settingsForm.settings.agentBrowsers).not.toBe(false);
    unmount();
    app.settings = { ...SETTINGS, mcpEnabled: false };
    settingsForm.settings = { ...SETTINGS, mcpEnabled: false };
    render(McpSection);
    expect(screen.getByRole('switch', { name: 'Pages des agents' })).toBeDisabled();
  });

  it('says it in English', () => {
    setLang('en');
    app.settings = { ...SETTINGS, mcpEnabled: true };
    settingsForm.settings = { ...SETTINGS, mcpEnabled: true, agentPages: false };
    render(McpSection);
    expect(screen.getByRole('switch', { name: 'Agents’ pages' })).toHaveAttribute('aria-checked', 'false');
  });
});
