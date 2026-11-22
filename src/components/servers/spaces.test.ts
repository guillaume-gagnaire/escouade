// The spaces (ESC-539, direction B « les piles », docs/SERVER-UX.md § 5 and 6): the piles of the rail and their packing
// into plates, « Partout » and its filter, the machine on each request and on the gesture that answers it, a request
// answered on its own space's connection, a space that is cut greyed with no gesture, `servers.aggregateView` to false
// leaving the numbers alone, the shortcuts, the named notifications, Réglages de l'espace › Appareils.

import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { connection, connectionOf, type Connection, type ServerInfo } from '../../lib/connection.svelte';
import { setLang } from '../../lib/i18n';
import { locks, NO_LOCKS } from '../../lib/locks.svelte';
import { servers } from '../../lib/servers.svelte';
import { spaceKey } from '../../lib/shortcuts';
import {
  compactPiles,
  contentless,
  countsOnly,
  notification,
  setSpaceLink,
  spaceList,
  spacesShown,
  spacesState,
  workSpace,
  type Glance,
} from '../../lib/spaces.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import type { Locks } from '../../lib/types';
import DevicesPage from '../settings/DevicesPage.svelte';
import ServersPage from '../settings/ServersPage.svelte';
import Everywhere from './Everywhere.svelte';
import RailSpace from './RailSpace.svelte';

const server = (id: string, name: string, host: string, ssh: string): ServerInfo => ({
  id,
  name,
  ssh,
  host,
  remotePort: 46480,
  device: 'DESKTOP-7QK2 · Windows',
  version: '2.3.2',
  active: false,
});
const TRAVAIL = server('s1', 'Travail', 'escouade.acme.internal', 'acme-escouade');
const PERSO = server('s2', 'Perso', 'vps.example.net', 'vps-perso');
const LABO = server('s3', 'Labo', 'labo.acme.internal', 'labo-gpu');

function view(list: ServerInfo[]) {
  servers.view = { servers: list, active: null, tokensInFile: false, thisDevice: 'DESKTOP-7QK2 · Windows' };
}

function resetConnection() {
  Object.assign(connection, {
    state: 'local',
    reason: '',
    secretsInFile: false,
    server: null,
    machine: null,
    latency: null,
    lostAt: null,
    tries: 0,
    retryAt: null,
    refusal: null,
    problem: null,
  });
}

const glance = (over: Partial<Glance> = {}): Glance => ({
  machine: { host: 'escouade', user: 'escouade' },
  version: '2.3.2',
  waiting: 2,
  running: 1,
  countsOnly: false,
  contentless: false,
  colors: ['#5fb37c', '#a78bfa'],
  projects: [
    { id: 'pf', name: 'facturation', color: '#5fb37c' },
    { id: 'pk', name: 'infra-k8s', color: '#22b8c8' },
  ],
  agents: [
    {
      id: 'a1',
      projectId: 'pf',
      name: 'relances-impayes',
      status: 'waiting',
      activity: null,
      lastActivity: 1,
      request: {
        id: 'r1',
        kind: 'permission',
        tool: 'Bash',
        arg: 'psql $DATABASE_URL -f migrations/0042_relances.sql',
        description: null,
        reason: null,
        questions: [],
        cut: false,
        defaultNo: false,
      },
    },
    {
      id: 'a2',
      projectId: 'pk',
      name: 'quotas-pods',
      status: 'waiting',
      activity: null,
      lastActivity: 2,
      request: {
        id: 'r2',
        kind: 'question',
        tool: 'AskUserQuestion',
        arg: '',
        description: null,
        reason: null,
        questions: [{ question: 'Limiter la mémoire des workers à 512 Mo ou 1 Go ?', options: ['512 Mo', '1 Go'] }],
        cut: false,
        defaultNo: false,
      },
    },
    {
      id: 'a3',
      projectId: 'pf',
      name: 'export-sepa',
      status: 'running',
      activity: 'Lance npx vitest run src/sepa',
      lastActivity: 3,
      request: null,
    },
  ],
  asks: [],
  ...over,
});

/** A connection to a space as the window holds it, open (or cut). */
function conn(state: Connection['state'] = 'open'): Connection {
  const c = connectionOf();
  c.state = state;
  c.latency = 38;
  return c;
}

function lock(pairs: Record<string, unknown>): Locks {
  const out: Locks = { locks: {}, rows: {}, warnings: [] };
  for (const [key, value] of Object.entries(pairs))
    out.locks[key] = { key, value, by: { kind: 'device' }, bounds: [], layers: [] } as unknown as Locks['locks'][string];
  return out;
}

let backend: ReturnType<typeof fakeBackend>;

beforeEach(() => {
  setLang('fr');
  resetApp();
  resetConnection();
  locks.current = NO_LOCKS;
  spacesState.watched = {};
  spacesState.filter = 'all';
  backend = fakeBackend({ remote_devices: () => [] });
});
afterEach(() => {
  cleanup();
  servers.view = null;
});

describe('the spaces', () => {
  it('are this computer first, then the servers; shown from two on', () => {
    view([]);
    expect(spacesShown()).toBe(false);
    view([TRAVAIL, PERSO]);
    expect(spaceList().map((s) => [s.name, s.index, s.current])).toEqual([
      ['Ce poste', 1, true],
      ['Travail', 2, false],
      ['Perso', 3, false],
    ]);
    expect(spacesShown()).toBe(true);
  });

  it('pack their piles into plates beyond three spaces, or at 1024', () => {
    expect(compactPiles(2, 1440)).toBe(false);
    expect(compactPiles(3, 1440)).toBe(false);
    expect(compactPiles(4, 1440)).toBe(true);
    expect(compactPiles(3, 1024)).toBe(true);
  });

  it('take Alt+1…9 (⌃ on macOS) and Ctrl+² by the key’s place', () => {
    const key = (o: Partial<KeyboardEvent>) => new KeyboardEvent('keydown', o as KeyboardEventInit);
    expect(spaceKey(key({ code: 'Digit2', key: 'é', altKey: true }), false)).toBe(2);
    expect(spaceKey(key({ code: 'Digit2', key: '2', ctrlKey: true }), true)).toBe(2);
    // Ctrl+digit stays the projects' on Windows; Alt+digit types a character on macOS; AltGr is never a shortcut.
    expect(spaceKey(key({ code: 'Digit2', ctrlKey: true }), false)).toBeNull();
    expect(spaceKey(key({ code: 'Digit2', altKey: true }), true)).toBeNull();
    expect(spaceKey(key({ code: 'Digit2', altKey: true, ctrlKey: true }), false)).toBeNull();
    expect(spaceKey(key({ code: 'Numpad2', altKey: true }), false)).toBeNull();
    expect(spaceKey(key({ code: 'Backquote', key: '²', ctrlKey: true }), false)).toBe('next');
  });

  it('say which server is a work one, and what the policies leave of it', () => {
    view([TRAVAIL, PERSO]);
    const [here, travail, perso] = spaceList();
    // Without `servers.allowed`, every server is a work one; never this computer.
    expect([here, travail, perso].map((s) => workSpace(s))).toEqual([false, true, true]);
    const l = lock({ 'servers.allowed': ['*.acme.internal'], 'servers.aggregateView': false, 'servers.contentlessNotifications': true });
    expect([here, travail, perso].map((s) => workSpace(s, l))).toEqual([false, true, false]);
    expect(countsOnly(travail, null, l)).toBe(true);
    expect(countsOnly(perso, null, l)).toBe(false);
    expect(contentless(travail, null, l)).toBe(true);
    expect(contentless(perso, null, l)).toBe(false);
    // The server's own policy holds for it, whatever this computer's.
    expect(countsOnly(perso, glance({ countsOnly: true }), NO_LOCKS)).toBe(true);
  });

  it('name their notifications, without content when the policy says so', () => {
    const g = glance();
    expect(notification({ name: 'Travail' }, g, g.agents[0], false)).toEqual({
      title: 'Travail · facturation · relances-impayes',
      body: 'Autorisation · Bash : psql $DATABASE_URL -f migrations/0042_relances.sql',
    });
    expect(notification({ name: 'Travail' }, g, g.agents[1], false).body).toBe(
      'Question : Limiter la mémoire des workers à 512 Mo ou 1 Go ?',
    );
    expect(notification({ name: 'Travail' }, g, g.agents[0], true)).toEqual({
      title: 'Travail',
      body: 'Un agent attend ta réponse (le détail ne s’affiche que dans Escouade)',
    });
  });
});

describe('the piles of the rail', () => {
  beforeEach(() => Object.defineProperty(window, 'innerWidth', { value: 1440, configurable: true }));
  afterEach(() => Object.defineProperty(window, 'innerWidth', { value: 1024, configurable: true }));

  it('head the projects with the open space, and stack each other space with what waits there', () => {
    view([TRAVAIL, PERSO]);
    setSpaceLink('s1', null, conn());
    spacesState.watched.s1.glance = glance();
    setSpaceLink('s2', null, conn('lost'));
    render(RailSpace, { part: 'head' });
    expect(screen.getByTestId('rail-space')).toHaveAccessibleName('Espace ouvert : Ce poste');
    cleanup();
    render(RailSpace, { part: 'others' });
    const nav = screen.getByRole('navigation', { name: 'Espaces' });
    const piles = within(nav).getAllByTestId('rail-space-other');
    expect(piles.map((p) => p.getAttribute('aria-label'))).toEqual(['Travail, 2 en attente (Alt 2)', 'Perso, coupé (Alt 3)']);
    expect(within(piles[0]).getByTestId('pile-count')).toHaveTextContent('2');
    // Two of its projects as cards behind its plate.
    expect(piles[0].querySelectorAll('.stack i')).toHaveLength(2);
    expect(nav).toHaveAttribute('data-compact', 'false');
  });

  it('turn into plates at 1024', () => {
    Object.defineProperty(window, 'innerWidth', { value: 1024, configurable: true });
    view([TRAVAIL, PERSO]);
    render(RailSpace, { part: 'others' });
    expect(screen.getByTestId('rail-piles')).toHaveAttribute('data-compact', 'true');
  });

  it('turn into plates beyond three spaces', () => {
    view([TRAVAIL, PERSO, LABO]);
    render(RailSpace, { part: 'others' });
    const nav = screen.getByTestId('rail-piles');
    expect(nav).toHaveAttribute('data-compact', 'true');
    expect(nav.querySelectorAll('.stack i')).toHaveLength(0);
    expect(within(nav).getAllByTestId('rail-space-other')).toHaveLength(3);
  });

  it('open a space’s menu: in its own window, retry while cut', async () => {
    view([TRAVAIL, PERSO]);
    setSpaceLink('s2', null, conn('lost'));
    const { menu } = await import('../../lib/menu.svelte');
    render(RailSpace, { part: 'others' });
    const perso = screen.getAllByTestId('rail-space-other')[1];
    await userEvent.pointer({ keys: '[MouseRight]', target: perso });
    const labels = (menu.open?.items ?? []).map((i) => i.label);
    expect(labels).toEqual(
      expect.arrayContaining(['Aller sur Perso', 'Ouvrir dans une fenêtre', 'Réessayer', 'Retirer Perso de ce poste…']),
    );
    menu.open?.items.find((i) => i.label === 'Ouvrir dans une fenêtre')?.onClick?.();
    await vi.waitFor(() => expect(backend.called('space_window')[0]?.args).toMatchObject({ space: 's2' }));
    menu.open = null;
  });
});

describe('« Partout »', () => {
  function setup(state: Connection['state'] = 'open', g: Glance = glance()) {
    view([TRAVAIL, PERSO]);
    const answered = vi.fn(async () => null);
    setSpaceLink('s1', { invoke: answered as never, close: () => {}, retryNow: () => {} }, conn(state));
    spacesState.watched.s1.glance = g;
    setSpaceLink('s2', null, conn());
    spacesState.watched.s2.glance = glance({ waiting: 0, running: 0, agents: [] });
    return answered;
  }

  it('shows every space under its machine, the open one first, the machine on each request and on its gesture', async () => {
    const answered = setup();
    render(Everywhere, { filter: 'all' });
    const sections = screen.getAllByTestId('everywhere-space');
    expect(sections.map((s) => s.getAttribute('aria-label'))).toEqual(['Ce poste', 'Travail', 'Perso']);
    const travail = sections[1];
    expect(within(travail).getByRole('heading')).toHaveTextContent('Travail');
    expect(within(travail).getByRole('heading')).toHaveTextContent('escouade.acme.internal · 38 ms');
    expect(within(travail).getByRole('heading')).toHaveTextContent('2 attendent');
    const rows = within(travail).getAllByTestId('everywhere-row');
    expect(rows).toHaveLength(3);
    for (const r of rows) expect(within(r).getByTestId('row-machine')).toHaveTextContent('Travail');
    expect(rows[0]).toHaveTextContent('Autorisation · Bash : psql $DATABASE_URL -f migrations/0042_relances.sql');
    expect(rows[1]).toHaveTextContent('Question · Limiter la mémoire des workers à 512 Mo ou 1 Go ?');
    expect(rows[2]).toHaveTextContent('Lance npx vitest run src/sepa');
    // A permission is answered here, on its own space's connection; a question in its conversation.
    const allow = within(rows[0]).getByRole('button', { name: 'Autoriser sur Travail la demande de relances-impayes' });
    expect(allow).toHaveTextContent('Autoriser sur Travail');
    expect(within(rows[1]).queryByRole('button', { name: /Autoriser/ })).toBeNull();
    await userEvent.click(allow);
    expect(answered).toHaveBeenCalledWith('answer_permission', { id: 'a1', requestId: 'r1', decision: 'allow', message: null });
    // Never on this computer's core.
    expect(backend.called('answer_permission')).toHaveLength(0);
  });

  it('filters by space', () => {
    setup();
    render(Everywhere, { filter: 's1' });
    expect(screen.getAllByTestId('everywhere-space').map((s) => s.getAttribute('aria-label'))).toEqual(['Travail']);
  });

  it('greys the requests of a space that is cut, with no gesture', () => {
    setup('lost');
    render(Everywhere, { filter: 's1' });
    const section = screen.getByTestId('everywhere-space');
    expect(section).toHaveClass('cut');
    expect(within(section).getByRole('heading')).toHaveTextContent('reconnexion');
    for (const r of within(section).getAllByTestId('everywhere-row')) {
      expect(r).toHaveAttribute('data-cut', 'true');
      expect(within(r).queryByRole('button', { name: /Autoriser|Ouvrir/ })).toBeNull();
    }
    expect(within(section).getByRole('button', { name: 'Réessayer' })).toBeVisible();
  });

  it('leaves the numbers alone of a work server when the policy denies the aggregate view', () => {
    setup();
    locks.current = lock({ 'servers.aggregateView': false });
    flushSync();
    render(Everywhere, { filter: 'all' });
    const travail = screen.getAllByTestId('everywhere-space')[1];
    expect(within(travail).queryAllByTestId('everywhere-row')).toHaveLength(0);
    expect(within(travail).getByTestId('everywhere-counts')).toHaveTextContent('2 en attente · 1 au travail');
    expect(travail).not.toHaveTextContent('psql');
    expect(within(travail).queryByRole('button', { name: /Autoriser/ })).toBeNull();
  });

  it('asks a work server its numbers alone when the policy denies the aggregate view', async () => {
    view([TRAVAIL]);
    locks.current = lock({ 'servers.aggregateView': false });
    const asked = vi.fn(async () => glance({ countsOnly: true, agents: [] }));
    setSpaceLink('s1', { invoke: asked as never, close: () => {}, retryNow: () => {} }, conn());
    const { pollAll } = await import('../../lib/spaces.svelte');
    await pollAll();
    expect(asked).toHaveBeenCalledWith('space_glance', { counts: true });
  });
});

describe('Réglages', () => {
  it('lists the devices of the space’s server, this computer marked; removes one without saying who asks', async () => {
    connection.server = { ...TRAVAIL, active: true };
    connection.state = 'open';
    backend = fakeBackend({
      remote_devices: () => [
        { id: 'd1', name: 'DESKTOP-7QK2 · Windows', createdAt: 1, lastSeen: 2 },
        { id: 'd2', name: 'Pixel 8', createdAt: 1 },
      ],
      remote_device_revoke: () => true,
    });
    render(DevicesPage);
    const rows = await screen.findAllByTestId('device-row');
    expect(rows[0]).toHaveTextContent('Ce poste');
    expect(within(rows[0]).getByRole('button', { name: 'Se déconnecter' })).toBeVisible();
    await userEvent.click(within(rows[1]).getByRole('button', { name: 'Retirer' }));
    expect(screen.queryByTestId('device-born')).toBeNull();
    await userEvent.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Retirer' }));
    // Who asks is the window's connection, as the server knows it: the window never names it (SR1-02).
    await vi.waitFor(() => expect(backend.called('remote_device_revoke')[0]?.args).toEqual({ id: 'd2' }));
    // A code is created on the server, never from here: no button, the way said.
    expect(screen.queryByRole('button', { name: /Créer un code/ })).toBeNull();
    expect(screen.getByText(/Lance escouade-server pair sur le serveur/)).toBeVisible();
  });

  it('says before removing a device the devices paired from it, which go with it', async () => {
    connection.server = { ...TRAVAIL, active: true };
    connection.state = 'open';
    app.toasts = [];
    backend = fakeBackend({
      remote_devices: () => [
        { id: 'd1', name: 'DESKTOP-7QK2 · Windows', createdAt: 1, lastSeen: 2 },
        { id: 'd2', name: 'Pixel 8', createdAt: 1 },
        { id: 'd3', name: 'Tablette', createdAt: 2, pairedBy: 'd2' },
        { id: 'd4', name: 'Portable', createdAt: 3, pairedBy: 'd3' },
      ],
      remote_device_revoke: () => true,
    });
    render(DevicesPage);
    const rows = await screen.findAllByTestId('device-row');
    expect(rows[2]).toHaveTextContent('appairé depuis Pixel 8');
    await userEvent.click(within(rows[1]).getByRole('button', { name: 'Retirer' }));
    expect(screen.getByTestId('device-born')).toHaveTextContent(
      'Et 2 appareils appairés depuis lui, retirés avec lui : Tablette, Portable.',
    );
    await userEvent.click(within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Retirer' }));
    await vi.waitFor(() =>
      expect(app.toasts.map((x) => x.text)).toEqual(['Pixel 8 et 2 appareils appairés depuis lui retirés de Travail']),
    );
  });

  it('shows the policies on the servers locked, with who imposes them', async () => {
    view([TRAVAIL]);
    backend = fakeBackend({ servers_view: () => servers.view });
    locks.current = lock({ 'servers.personalSpace': false, 'servers.aggregateView': false, 'servers.contentlessNotifications': true });
    render(ServersPage);
    const rows = await screen.findAllByTestId('server-policy');
    expect(rows.map((r) => r.getAttribute('data-key'))).toEqual([
      'servers.personalSpace',
      'servers.aggregateView',
      'servers.contentlessNotifications',
    ]);
    for (const r of rows) expect(r).toHaveTextContent('Imposé par la stratégie de ce poste');
  });
});

void app;
