// The band « veut lire » of a browser (ESC-669, choice C4): an agent asked to read a page outside its project's launch.
// It shows in the browser's view, above the page, the oldest request first; « Permettre » and « Refuser » go to the
// backend, which tells the agent; « Retirer les accords » in the row's menu takes the leaves back; the switch
// « Agents et navigateurs » of Réglages › Agents, under « Les agents peuvent piloter Escouade ».
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

import { devpages, onPageEvent, placementOf, type PageState } from '../lib/devpage.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { settingsForm } from '../lib/settings.svelte';
import { app } from '../lib/state.svelte';
import type { PageConsents } from '../lib/types';
import { agent, fakeBackend, project, resetApp, SETTINGS } from '../test/ipc';
import BrowsersSection from './BrowsersSection.svelte';
import ConsentBand from './ConsentBand.svelte';
import DevPage from './DevPage.svelte';
import McpSection from './settings/McpSection.svelte';

const ASK = { agent: 'a7', origin: 'https://claude.ai' };
const page = (over: Partial<PageState> = {}): PageState => ({
  url: 'https://claude.ai/chat/xyz',
  title: 'claude.ai',
  docTitle: 'Claude',
  phase: 'ready',
  projectId: 'p1',
  by: { kind: 'user' },
  opened: 1,
  ...over,
});

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [agent({ id: 'a7', name: 'esc-412' }), agent({ id: 'a8', name: 'esc-413' })] });
  app.ui.panes = {};
  devpages.pages = { 'browser:c': page() };
});

describe('the band of a browser', () => {
  it('says which agent wants to read which host, with the user’s session, and how many more wait', () => {
    const answered: [string, boolean][] = [];
    render(ConsentBand, {
      asks: [ASK, { agent: 'a8', origin: 'https://docs.stripe.com' }],
      onanswer: (a, allow) => void answered.push([a.origin, allow]),
    });
    const band = screen.getByTestId('consent-band');
    expect(band).toHaveTextContent('esc-412 veut lire claude.ai (avec ta session, dans ce navigateur) et 1 autre demande');
    expect(within(band).getByRole('button', { name: 'Permettre' })).toBeInTheDocument();
    expect(within(band).getByRole('button', { name: 'Refuser' })).toBeInTheDocument();
  });

  it('shows nothing without a request', () => {
    render(ConsentBand, { asks: [], onanswer: () => {} });
    expect(screen.queryByTestId('consent-band')).not.toBeInTheDocument();
  });

  it('reads in English', () => {
    setLang('en');
    render(ConsentBand, { asks: [ASK], onanswer: () => {} });
    expect(screen.getByTestId('consent-band')).toHaveTextContent('esc-412 wants to read claude.ai (with your session, in this browser)');
    expect(screen.getByRole('button', { name: 'Allow' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Refuse' })).toBeInTheDocument();
    setLang('fr');
  });

  it('comes in the browser’s view when the backend says an agent asked, and « Permettre » goes to the backend', async () => {
    let held: PageConsents = { asks: [], grants: [] };
    const b = fakeBackend({
      devpage_consents: () => held,
      devpage_consent: (args: { allow: boolean }) => {
        held = { asks: [], grants: args.allow ? [ASK] : [] };
        return held;
      },
    });
    render(DevPage, { pageKey: 'browser:c' });
    expect(screen.queryByTestId('consent-band')).not.toBeInTheDocument();
    // The tool asked: the backend tells the window, which reads the band again.
    held = { asks: [ASK], grants: [] };
    onPageEvent({ agentId: 'browser:c', phase: 'consents', url: '' });
    const band = await screen.findByTestId('consent-band');
    expect(band).toHaveTextContent('esc-412 veut lire claude.ai');
    await userEvent.click(within(band).getByRole('button', { name: 'Permettre' }));
    expect(b.called('devpage_consent').map((c) => c.args)).toEqual([
      { agentId: 'browser:c', agent: 'a7', origin: 'https://claude.ai', allow: true },
    ]);
    await waitFor(() => expect(screen.queryByTestId('consent-band')).not.toBeInTheDocument());
    expect(devpages.pages['browser:c'].consents?.grants).toEqual([ASK]);
  });

  it('« Refuser » tells the backend no', async () => {
    const b = fakeBackend({
      devpage_consents: () => ({ asks: [ASK], grants: [] }),
      devpage_consent: () => ({ asks: [], grants: [] }),
    });
    render(DevPage, { pageKey: 'browser:c' });
    const band = await screen.findByTestId('consent-band');
    await userEvent.click(within(band).getByRole('button', { name: 'Refuser' }));
    expect(b.called('devpage_consent').at(-1)?.args.allow).toBe(false);
  });
});

describe('« Retirer les accords »', () => {
  it('is in the row’s menu while the browser has leaves or requests, and takes them back', async () => {
    const b = fakeBackend({ devpage_revoke: () => ({ asks: [], grants: [] }) });
    render(BrowsersSection, { project: project() });
    const row = await screen.findByText('Claude');
    /** The labels of the menu open now. */
    const items = () => menu.open?.items ?? [];
    await fireEvent.contextMenu(row);
    expect(items().map((i) => i.label)).not.toContain('Retirer les accords');
    menu.open = null;
    devpages.pages['browser:c'].consents = { asks: [], grants: [ASK] };
    await tick();
    await fireEvent.contextMenu(row);
    const item = items().find((i) => i.label === 'Retirer les accords');
    expect(item).toBeTruthy();
    item!.onClick?.();
    await waitFor(() => expect(b.called('devpage_revoke').map((c) => c.args)).toEqual([{ agentId: 'browser:c' }]));
    await waitFor(() => expect(devpages.pages['browser:c'].consents?.grants).toEqual([]));
  });
});

describe('where a browser is shown', () => {
  const rect = (width: number) => ({ width }) as DOMRect;
  it('is full alone, beside when narrower than the work area, below when as wide', () => {
    expect(placementOf(false, rect(800), rect(1200))).toBe('full');
    expect(placementOf(true, rect(600), rect(1200))).toBe('beside');
    expect(placementOf(true, rect(1200), rect(1200))).toBe('below');
  });
});

describe('the switch « Agents et navigateurs »', () => {
  it('is under « Les agents peuvent piloter Escouade », on by default, saved with the draft, greyed while the server is off', async () => {
    fakeBackend({ mcp_status: () => ({ running: false, port: 0, error: null }), mcp_declare_status: () => [] });
    app.settings = { ...SETTINGS, mcpEnabled: true };
    settingsForm.settings = { ...SETTINGS, mcpEnabled: true };
    const { unmount } = render(McpSection);
    const sw = screen.getByRole('switch', { name: 'Agents et navigateurs' });
    expect(sw).toHaveAttribute('aria-checked', 'true');
    expect(sw).toBeEnabled();
    expect(screen.getByText(/le lancement du projet sans demander/)).toBeInTheDocument();
    await userEvent.click(sw);
    expect(settingsForm.settings.agentBrowsers).toBe(false);
    unmount();
    app.settings = { ...SETTINGS, mcpEnabled: false };
    render(McpSection);
    expect(screen.getByRole('switch', { name: 'Agents et navigateurs' })).toBeDisabled();
  });
});
