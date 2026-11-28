// The phone's shell, direction B (MO-1): its three tabs, the chips of the spaces, the « + » and its sheet, the band of
// a space that is cut, its empty states, in French and English; and « Escouade sur ce téléphone » with no space.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { setLang } from '$shared/lib/i18n';
import App from './App.svelte';
import { kanban } from './lib/board.svelte';
import { useLock } from './lib/lock';
import { nav } from './lib/nav.svelte';
import { pairing } from './lib/pairing.svelte';
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
    pairing.open = false;
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

  it('shows the Kanban of a project: its space, its columns in segments, its tickets and their gestures as buttons', async () => {
    Object.assign(kanban, { choice: null, lists: {}, board: null, column: null, open: null, rejecting: null, busy: [], error: null });
    localStorage.clear();
    const unlocks: string[] = [];
    useLock({ unlock: async ({ reason }: { reason: string }) => void unlocks.push(reason), lockState: async () => ({}) } as never);
    await withSpaces([{}, {}]);
    const line = (over: Record<string, unknown>) => ({
      agent: null,
      iteration: 0,
      maxLoops: 5,
      criteria: [],
      partial: false,
      blocked: null,
      conflict: false,
      step: null,
      ...over,
    });
    const board = {
      projects: [project, { id: 'p2', name: 'relances', color: 'oklch(0.72 0.12 48)' }],
      projectId: 'p1',
      counts: { todo: 1, doing: 1, review: 2, pr: 0, done: 40 },
      tickets: [
        line({ id: 't0', key: 'FAC-44', title: 'Exporter en CSV', column: 'todo', criteria: [false] }),
        line({
          id: 't4',
          key: 'FAC-41',
          title: 'Relances à J+15',
          column: 'doing',
          agent: 'relances',
          iteration: 2,
          criteria: [true, false],
        }),
        line({
          id: 't1',
          key: 'FAC-38',
          title: 'Avoirs partiels',
          column: 'review',
          agent: 'tests-e2e',
          iteration: 3,
          criteria: [true, true, false],
        }),
        line({
          id: 't2',
          key: 'FAC-40',
          title: 'Export FEC',
          column: 'review',
          agent: 'export-fec',
          iteration: 2,
          blocked: 'Conflit',
          conflict: true,
        }),
        line({ id: 't9', key: 'FAC-12', title: 'Fini', column: 'done', iteration: 4 }),
      ],
    };
    const none = { projects: [], projectId: null, counts: board.counts, tickets: [] };
    render(App);
    await fireEvent.click(screen.getByRole('tab', { name: 'Kanban' }));
    await tick();
    sockets[0].answer('board_glance', none);
    sockets[1].answer('board_glance', board);
    await tick();
    sockets[1].answer('board_glance', board);
    await waitFor(() => expect(screen.getByRole('button', { name: 'facturation, sur Travail : changer de projet' })).toBeInTheDocument());
    // The segments: « À tester » first, as what waits for the user.
    const segments = within(screen.getByRole('radiogroup', { name: 'Les colonnes' })).getAllByRole('radio');
    expect(segments.map((x) => x.getAttribute('aria-label'))).toEqual([
      'À faire, 1 ticket',
      'En cours, 1 ticket',
      'À tester, 2 tickets',
      'Terminé, 40 tickets',
    ]);
    expect(screen.getByRole('radio', { name: 'À tester, 2 tickets' })).toBeChecked();
    const row = screen.getByRole('button', { name: 'FAC-38 Avoirs partiels' });
    expect(row).toHaveTextContent('tests-e2e');
    expect(row).toHaveTextContent('boucle 3/5');
    expect(screen.getByRole('img', { name: '2 critères remplis sur 3' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'FAC-40 Export FEC' })).toHaveTextContent('Conflit de merge');

    // Touched: its gestures as buttons (the swipe's accessible way).
    expect(row).toHaveAttribute('aria-expanded', 'false');
    await fireEvent.click(row);
    expect(row).toHaveAttribute('aria-expanded', 'true');
    const gestures = screen.getByRole('group', { name: 'Les gestes de FAC-38' });
    expect(
      within(gestures)
        .getAllByRole('button')
        .map((b) => b.textContent?.trim()),
    ).toEqual(['Renvoyer', 'Valider']);
    // « Renvoyer » asks what to change.
    await fireEvent.click(within(gestures).getByRole('button', { name: 'Renvoyer' }));
    const why = screen.getByRole('textbox', { name: 'Ce qu’il faut changer' });
    const send = screen.getByRole('button', { name: 'Renvoyer à l’agent' });
    expect(send).toBeDisabled();
    await fireEvent.input(why, { target: { value: 'Ajoute un test' } });
    expect(send).toBeEnabled();
    await fireEvent.click(send);
    await tick();
    expect(sockets[1].calls().find((c) => c.cmd === 'ticket_reject')?.args).toEqual({ id: 't1', comment: 'Ajoute un test' });
    sockets[1].answer('ticket_reject', null);
    await tick();
    sockets[1].answer('board_glance', board);

    // The conflict: « L'agent résout », under the lock.
    await fireEvent.click(screen.getByRole('button', { name: 'FAC-40 Export FEC' }));
    await fireEvent.click(
      within(screen.getByRole('group', { name: 'Les gestes de FAC-40' })).getByRole('button', { name: 'L’agent résout' }),
    );
    await waitFor(() => expect(sockets[1].calls().some((c) => c.cmd === 'ticket_resolve_conflict')).toBe(true));
    expect(unlocks).toEqual(['Laisser l’agent résoudre le conflit de FAC-40 ? · Escouade · Travail']);
    sockets[1].answer('ticket_resolve_conflict', null);
    await tick();
    sockets[1].answer('board_glance', board);

    // « À faire »: « Lancer »; « En cours »: no gesture; « Terminé »: the older ones said to be on the computer.
    await fireEvent.click(screen.getByRole('radio', { name: 'À faire, 1 ticket' }));
    await fireEvent.click(screen.getByRole('button', { name: 'FAC-44 Exporter en CSV' }));
    expect(within(screen.getByRole('group', { name: 'Les gestes de FAC-44' })).getByRole('button', { name: 'Lancer' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('radio', { name: 'En cours, 1 ticket' }));
    expect(screen.queryByRole('button', { name: 'FAC-41 Relances à J+15' })).toBeNull();
    expect(screen.getByText('Relances à J+15')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('radio', { name: 'Terminé, 40 tickets' }));
    expect(screen.getByText('39 tickets terminés plus anciens se voient au poste.')).toBeInTheDocument();

    // The sheet of the projects: another one shown.
    await fireEvent.click(screen.getByRole('button', { name: 'facturation, sur Travail : changer de projet' }));
    const sheet = screen.getByRole('dialog', { name: 'Les projets' });
    expect(within(sheet).getByRole('button', { name: 'facturation' })).toHaveAttribute('aria-current', 'true');
    await fireEvent.click(within(sheet).getByRole('button', { name: 'relances' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(kanban.choice).toEqual({ space: 'travail', project: 'p2' });
    await tick();
    expect(sockets[1].calls().at(-1)).toMatchObject({ cmd: 'board_glance', args: { projectId: 'p2' } });
    useLock(null);
  });

  it('speaks English when the system does', async () => {
    setLang('en');
    await withSpaces([{}, {}]);
    render(App);
    expect(screen.getByRole('tab', { name: 'Your turn' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Everywhere, 0 waiting' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Nothing is waiting for you' })).toBeInTheDocument();
  });

  it('shows a space that revoked this phone on a screen of its own, when and from which device', async () => {
    startSpaces([{ id: 'travail', name: 'Travail', here: false, url: 'ws://127.0.0.1:2/remote', token: 'b' }], {
      open: (url) => {
        const s = new FakeSocket(url);
        sockets.push(s);
        return s;
      },
      delays: [10_000],
      every: 60_000,
    });
    render(App);
    sockets[0].onopen?.();
    sockets[0].json({
      t: 'refused',
      code: 'token',
      error: 'revoked',
      revoked: { device: 'Pixel 8', at: Date.UTC(2026, 10, 27, 9, 0), by: 'MacBook-Air' },
    });
    sockets[0].close(1008, 'token');
    const dialog = await screen.findByRole('alertdialog', { name: 'Ce téléphone n’est plus appairé à Travail' });
    expect(within(dialog).getByText(/Révoqué le .*, depuis MacBook-Air./)).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Appairer de nouveau' })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Retirer Travail de ce téléphone' })).toBeInTheDocument();
    expect(screen.queryByRole('tablist')).toBeNull();
  });

  it('lists the spaces of this phone in the « + » sheet, with « Appairer un espace »', async () => {
    await withSpaces([{}, {}]);
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Créer' }));
    const sheet = screen.getByRole('dialog', { name: 'Créer' });
    expect(within(sheet).getByText('Tes espaces')).toBeInTheDocument();
    expect(within(sheet).getByText('Travail')).toBeInTheDocument();
    await fireEvent.click(within(sheet).getByRole('button', { name: 'Appairer un espace' }));
    expect(screen.getByRole('heading', { name: 'Escouade sur ce téléphone' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Scanner le QR code' })).toBeInTheDocument();
  });
});
