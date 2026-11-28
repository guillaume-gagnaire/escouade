// « Mettre à jour le serveur » (ESC-702): on the refusal of the versions, the button asks the server by this computer's
// token after a confirmation, says once what refused it (never asked again by itself), and in the image gives the
// `docker pull` to copy; on a server's space, Réglages › Général shows the server's own updates, its button and its two
// switches (its settings), never this computer's.

import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { connection, type ServerInfo, type ServerUpdate } from '../../lib/connection.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import ServerScreen from './ServerScreen.svelte';

const TRAVAIL: ServerInfo = {
  id: 's1',
  name: 'Travail',
  ssh: 'acme-escouade',
  host: 'escouade.acme.internal',
  remotePort: 46480,
  device: 'DESKTOP-7QK2 · Windows',
  version: '2.3.0',
  machine: { host: 'escouade', user: 'escouade' },
  active: true,
};

function refusedFor(serverUpdate: ServerUpdate) {
  connection.server = TRAVAIL;
  connection.state = 'refused';
  connection.refusal = {
    code: 'version',
    error: '',
    serverVersion: '2.3.0',
    update: 'server',
    clientVersion: '2.3.2',
    serverUpdate,
  };
}

function reset() {
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

describe('« Mettre à jour le serveur » on the refusal of the versions', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    reset();
  });
  afterEach(() => {
    cleanup();
    reset();
  });

  it('says both versions and asks the server by this computer’s token, after a confirmation', async () => {
    const backend = fakeBackend({ server_update_ask: () => '2.3.2' });
    refusedFor({ how: 'command', command: 'escouade-server update --to 2.3.2' });
    render(ServerScreen);
    expect(screen.getByRole('alertdialog')).toHaveTextContent('Ce poste a Escouade 2.3.2, le serveur 2.3.0');
    expect(screen.getByRole('alertdialog')).toHaveTextContent('Le serveur est à mettre à jour vers 2.3.2.');
    // The command by hand, to copy, beside the button.
    expect(screen.getByText('escouade-server update --to 2.3.2')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Mettre à jour le serveur' }));
    // Nothing asked before the confirmation, which says what follows.
    expect(backend.called('server_update_ask')).toHaveLength(0);
    const confirm = screen.getByRole('group', { name: 'Mettre à jour Travail en 2.3.2 ?' });
    expect(confirm).toHaveTextContent('vérifie sa signature');
    expect(confirm).toHaveTextContent('dès qu’il est au repos');
    await userEvent.click(within(confirm).getByRole('button', { name: 'Mettre à jour' }));
    await vi.waitFor(() => expect(screen.getByTestId('update-asked')).toHaveTextContent('Travail se met en 2.3.2'));
    expect(backend.called('server_update_ask').map((c) => c.args)).toEqual([{ id: 's1', version: '2.3.2' }]);
    expect(screen.getByRole('button', { name: 'Se reconnecter' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Aller sur Ce poste' })).toBeVisible();
  });

  it('says once what refused it, never asking again by itself', async () => {
    const backend = fakeBackend({
      server_update_ask: () => {
        throw 'ce serveur ne tourne pas sous systemd : lance « escouade-server update --to 2.3.2 » sur le serveur';
      },
    });
    refusedFor({ how: 'command', command: 'escouade-server update --to 2.3.2' });
    render(ServerScreen);
    await userEvent.click(screen.getByRole('button', { name: 'Mettre à jour le serveur' }));
    await userEvent.click(within(screen.getByRole('group')).getByRole('button', { name: 'Mettre à jour' }));
    await vi.waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('ne tourne pas sous systemd'));
    await new Promise((r) => setTimeout(r, 200));
    expect(backend.called('server_update_ask')).toHaveLength(1);
    // The button is there to try again by hand.
    expect(screen.getByRole('button', { name: 'Mettre à jour le serveur' })).toBeEnabled();
  });

  it('in the image, gives the docker pull to copy and no button that would change the server', async () => {
    const backend = fakeBackend();
    const write = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText: write }, configurable: true });
    const command = 'docker pull ghcr.io/guillaume-gagnaire/escouade/escouade-server:2.3.2 && docker compose up -d';
    refusedFor({ how: 'image', command });
    render(ServerScreen);
    expect(screen.getByTestId('update-server')).toHaveTextContent('Dans l’image Docker, rien n’est remplacé');
    expect(screen.getByText(command)).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Mettre à jour le serveur' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Copier la commande' }));
    expect(write).toHaveBeenCalledWith(command);
    expect(backend.called('server_update_ask')).toHaveLength(0);
  });

  it('speaks English too', () => {
    setLang('en');
    fakeBackend();
    refusedFor({ how: 'command', command: 'escouade-server update --to 2.3.2' });
    render(ServerScreen);
    expect(screen.getByRole('button', { name: 'Update the server' })).toBeVisible();
    expect(screen.getByRole('alertdialog')).toHaveTextContent('The server needs updating to 2.3.2.');
  });
});

describe('Réglages › Général on a server: the server’s own updates', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    reset();
    app.modal = { kind: 'settings', page: 'general' };
  });
  afterEach(() => {
    cleanup();
    reset();
  });

  const state = (over: Record<string, unknown>) => ({
    version: '2.3.0',
    phase: 'available',
    available: '2.3.1',
    automatic: false,
    channel: 'stable',
    image: false,
    installs: true,
    ...over,
  });

  it('shows the server’s version, the version found and asks it after a confirmation', async () => {
    connection.server = TRAVAIL;
    connection.state = 'open';
    const backend = fakeBackend({ server_update_state: () => state({}), server_update_start: () => '2.3.1' });
    render(SettingsModal, { page: 'general' });
    const group = await screen.findByTestId('server-updates');
    await vi.waitFor(() => expect(group).toHaveTextContent('escouade-server 2.3.0'));
    expect(group).toHaveTextContent('Mises à jour de Travail');
    expect(group).toHaveTextContent('2.3.1 est disponible');
    expect(group).toHaveTextContent('La mise à jour automatique est éteinte');
    // Not this computer's update: no « Rechercher une mise à jour ».
    expect(screen.queryByRole('button', { name: 'Rechercher une mise à jour' })).toBeNull();
    // The server's two switches.
    expect(within(group).getByRole('switch', { name: 'Mettre le serveur à jour automatiquement' })).toBeVisible();
    expect(within(group).getByRole('switch', { name: 'Canal bêta' })).toBeVisible();
    await userEvent.click(within(group).getByRole('button', { name: 'Mettre à jour le serveur' }));
    expect(backend.called('server_update_start')).toHaveLength(0);
    const confirm = within(group).getByRole('group', { name: 'Mettre à jour Travail en 2.3.1 ?' });
    await userEvent.click(within(confirm).getByRole('button', { name: 'Mettre à jour' }));
    await vi.waitFor(() => expect(backend.called('server_update_start').map((c) => c.args)).toEqual([{ version: '2.3.1' }]));
    await vi.waitFor(() => expect(group).toHaveTextContent('Mise à jour vers 2.3.1 demandée'));
  });

  it('says a version in place waiting for rest, and in the image the new image with its command', async () => {
    connection.server = TRAVAIL;
    connection.state = 'open';
    let now = state({ phase: 'ready', automatic: true, busy: 'turn' });
    fakeBackend({ server_update_state: () => now });
    render(SettingsModal, { page: 'general' });
    const group = await screen.findByTestId('server-updates');
    await vi.waitFor(() => expect(group).toHaveTextContent('2.3.1 est posée'));
    expect(group).toHaveTextContent('il attend : un agent est en plein tour');
    expect(within(group).queryByRole('button', { name: 'Mettre à jour le serveur' })).toBeNull();
    cleanup();
    now = state({
      phase: 'image',
      image: true,
      installs: false,
      command: 'docker pull ghcr.io/guillaume-gagnaire/escouade/escouade-server:2.3.1 && docker compose up -d',
    });
    render(SettingsModal, { page: 'general' });
    const again = await screen.findByTestId('server-updates');
    await vi.waitFor(() => expect(again).toHaveTextContent('Une nouvelle image existe : 2.3.1'));
    expect(again).toHaveTextContent('docker pull ghcr.io/guillaume-gagnaire/escouade/escouade-server:2.3.1');
    expect(within(again).queryByRole('button', { name: 'Mettre à jour le serveur' })).toBeNull();
  });
});
