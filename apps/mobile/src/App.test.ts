// The phone's shell, direction B (MO-1): its three tabs, the chips of the spaces, the « + » and its sheet, the band of
// a space that is cut, its empty states, in French and English; and « Escouade sur ce téléphone » with no space.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { setLang } from '$shared/lib/i18n';
import App from './App.svelte';
import { nav } from './lib/nav.svelte';
import { ALL, phone, startSpaces, stopSpaces } from './lib/phone-spaces.svelte';
import { FakeSocket, glance, tick } from './test/fake-socket';

const project = { id: 'p1', name: 'facturation', color: 'oklch(0.72 0.12 250)' };

describe('the phone’s shell', () => {
  let sockets: FakeSocket[];

  beforeEach(() => {
    sockets = [];
    phone.spaces = [];
    phone.filter = ALL;
    nav.tab = 'inbox';
    nav.sheet = null;
  });
  afterEach(() => stopSpaces());

  async function withSpaces(glances: Record<string, unknown>[]) {
    startSpaces(
      [
        { id: 'here', name: 'Ce poste', here: true, url: 'ws://127.0.0.1:1/remote', token: 'a' },
        { id: 'travail', name: 'Travail', here: false, url: 'ws://127.0.0.1:2/remote', token: 'b' },
      ],
      {
        open: (url) => {
          const s = new FakeSocket(url);
          sockets.push(s);
          return s;
        },
        delays: [10_000],
        every: 60_000,
      },
    );
    sockets.forEach((s) => s.welcome());
    await tick();
    sockets.forEach((s, i) => s.answerGlances(glance(glances[i])));
    await tick();
  }

  it('with no space, says what the app is for and where the pairing starts', () => {
    render(App);
    expect(screen.getByRole('heading', { name: 'Escouade sur ce téléphone' })).toBeInTheDocument();
    expect(screen.queryByRole('tablist')).toBeNull();
  });

  it('shows the three tabs, the chips with what waits, and the agents that stopped', async () => {
    await withSpaces([
      {},
      {
        waiting: 2,
        projects: [project],
        agents: [{ id: 'g1', projectId: 'p1', name: 'tests-e2e', status: 'waiting', activity: null, lastActivity: 1, request: null }],
        asks: [{ id: 'k1', projectId: 'p1', title: 'Publier 1.4.0 ?', createdAt: 1 }],
      },
    ]);
    render(App);
    const tabs = within(screen.getByRole('tablist', { name: 'Les onglets de l’app' })).getAllByRole('tab');
    expect(tabs.map((t) => t.textContent?.replace(/\d/g, '').trim())).toEqual(['À toi', 'Agents', 'Kanban']);
    expect(screen.getByRole('tab', { name: 'À toi, 2 en attente' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('radio', { name: 'Partout, 2 en attente' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Travail, 2 en attente' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /Tes agents se sont arrêtés/ })).toBeInTheDocument();
    expect(screen.getByText('tests-e2e')).toBeInTheDocument();
    expect(screen.getByText('Publier 1.4.0 ?')).toBeInTheDocument();
    // A chip shows its space alone.
    await fireEvent.click(screen.getByRole('radio', { name: 'Ce poste, 0 en attente' }));
    expect(screen.getByRole('heading', { name: 'Rien ne t’attend sur Ce poste' })).toBeInTheDocument();
  });

  it('opens each tab on its empty state', async () => {
    await withSpaces([{}, {}]);
    render(App);
    expect(screen.getByRole('heading', { name: 'Rien ne t’attend' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('tab', { name: 'Agents' }));
    expect(screen.getByRole('heading', { level: 1, name: 'Agents' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Aucun agent' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('tab', { name: 'Kanban' }));
    expect(screen.getByRole('heading', { name: 'Choisis un projet' })).toBeInTheDocument();
    // No chips on the Kanban: its project is chosen there (MO-7).
    expect(screen.queryByRole('radiogroup')).toBeNull();
  });

  it('opens the sheet of the « + », which names what the phone will create, and closes it', async () => {
    await withSpaces([{}, {}]);
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Créer' }));
    const sheet = screen.getByRole('dialog', { name: 'Créer' });
    expect(within(sheet).getByText('Nouveau projet')).toBeInTheDocument();
    expect(within(sheet).getByText('Nouvel agent')).toBeInTheDocument();
    expect(within(sheet).getByText('Nouveau ticket')).toBeInTheDocument();
    await fireEvent.keyDown(sheet, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Créer' }));
    await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Fermer' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('marks a space that is cut, with a band that says when it tries again, the other one unchanged', async () => {
    await withSpaces([{}, {}]);
    render(App);
    sockets[1].close(1006, 'gone');
    await waitFor(() => expect(screen.getByRole('radio', { name: 'Travail, injoignable' })).toBeInTheDocument());
    expect(screen.getByRole('status')).toHaveTextContent(/Travail injoignable · nouvel essai dans \d+ s/);
    expect(screen.getByRole('radio', { name: 'Ce poste, 0 en attente' })).toBeInTheDocument();
  });

  it('speaks English when the system does', async () => {
    setLang('en');
    await withSpaces([{}, {}]);
    render(App);
    expect(screen.getByRole('tab', { name: 'Your turn' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Everywhere, 0 waiting' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Nothing is waiting for you' })).toBeInTheDocument();
  });
});
