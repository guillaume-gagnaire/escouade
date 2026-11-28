// « Se connecter à un serveur » (ESC-537, docs/SERVER-UX.md § 2, 3, 4): the dialog's steps against a fake core (the
// hosts of ~/.ssh/config, each state of a connection, the code typed, pasted or got over SSH, the pairing), the screens
// of the states in French and in English, and the machine said: title bar, status bar, band of a cut, composer paused,
// Réglages › Serveurs.

import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { connection, type ServerInfo, type ServerProblem as Problem } from '../../lib/connection.svelte';
import { setLang } from '../../lib/i18n';
import { nth, plateOf, servers } from '../../lib/servers.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import ConnectionBand from '../ConnectionBand.svelte';
import ConnectServerModal from './ConnectServerModal.svelte';
import ServerProblem from './ServerProblem.svelte';
import ServerScreen from './ServerScreen.svelte';
import TunnelItem from './TunnelItem.svelte';
import ServersPage from '../settings/ServersPage.svelte';

const TRAVAIL: ServerInfo = {
  id: 's1',
  name: 'Travail',
  ssh: 'acme-escouade',
  host: 'escouade.acme.internal',
  remotePort: 46480,
  device: 'DESKTOP-7QK2 · Windows',
  version: '2.3.2',
  machine: { host: 'escouade', user: 'escouade' },
  active: true,
};

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

const HOSTS = [
  { alias: 'acme-escouade', hostName: 'escouade.acme.internal', user: 'escouade', port: 22 },
  { alias: 'labo-gpu', hostName: '10.4.0.12', user: 'gpu', proxyJump: 'bastion-acme' },
];

describe('the dialog « Se connecter à un serveur »', () => {
  let backend: ReturnType<typeof fakeBackend>;
  let attempt: (a: any) => unknown;
  let pair: (a: any) => unknown;
  let overSsh: (a: any) => unknown;

  beforeEach(() => {
    setLang('fr');
    resetApp();
    resetConnection();
    attempt = () => ({
      outcome: 'ok',
      value: { attempt: 'a1', version: '2.3.2', name: 'acme-escouade', device: 'DESKTOP · Windows', host: 'x' },
    });
    pair = () => ({ outcome: 'ok', value: 's1' });
    overSsh = () => ({ outcome: 'ok', value: 'K7Q4-MXR9' });
    backend = fakeBackend({
      servers_ssh_hosts: () => HOSTS,
      servers_view: () => ({ servers: [], active: null, tokensInFile: false, thisDevice: 'DESKTOP · Windows' }),
      server_attempt: (a) => attempt(a),
      server_pair: (a) => pair(a),
      server_code_over_ssh: (a) => overSsh(a),
      server_open: () => ({ outcome: 'problem', problem: { kind: 'refused', code: 'x', message: 'stop here' } }),
      server_cancel: () => null,
    });
  });
  afterEach(cleanup);

  it('lists the hosts of ~/.ssh/config with what they point to, and connects to the one chosen', async () => {
    render(ConnectServerModal);
    const dialog = await screen.findByRole('dialog', { name: 'Se connecter à un serveur' });
    const radios = await within(dialog).findAllByRole('radio');
    expect(radios.map((r) => r.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'acme-escouade ~/.ssh/config escouade@escouade.acme.internal · port 22',
      'labo-gpu ~/.ssh/config gpu@10.4.0.12 · par bastion-acme (ProxyJump)',
    ]);
    expect(radios[0]).toHaveAttribute('aria-checked', 'true');
    expect(dialog).toHaveTextContent('Il ne garde aucune clé');
    await userEvent.click(radios[1]);
    await userEvent.click(within(dialog).getByRole('button', { name: /Se connecter/ }));
    expect(backend.called('server_attempt')[0].args).toMatchObject({ host: 'labo-gpu', remotePort: null });
    // Escouade answers: the pairing step, the code's eight cells.
    expect(await within(dialog).findByText('Appairer ce poste à labo-gpu')).toBeVisible();
    expect(within(dialog).getByRole('group', { name: 'Code d’appairage, 8 caractères' })).toBeVisible();
    expect(dialog).toHaveTextContent('Escouade 2.3.2 répond sur le serveur');
  });

  it('takes a host typed, with the server’s port', async () => {
    render(ConnectServerModal);
    const dialog = await screen.findByRole('dialog');
    await userEvent.type(within(dialog).getByPlaceholderText('utilisateur@adresse:port'), 'escouade@vps.example.net:2222');
    await userEvent.type(within(dialog).getByPlaceholderText('46480'), '4122');
    await userEvent.keyboard('{Enter}');
    await userEvent.click(within(dialog).getByRole('button', { name: /Se connecter/ }));
    expect(backend.called('server_attempt')[0].args).toMatchObject({ host: 'escouade@vps.example.net:2222', remotePort: 4122 });
  });

  const states: [string, Problem, RegExp[]][] = [
    [
      'unreachable',
      {
        kind: 'unreachable',
        why: 'unreachable',
        output: 'ssh: connect to host escouade.acme.internal port 22: Connection timed out',
        command: 'ssh -N -L 47831:127.0.0.1:46480 acme-escouade',
      },
      [
        /acme-escouade ne répond pas/,
        /Rien n’a été changé sur ce poste ni sur le serveur/,
        /Connection timed out/,
        /VPN de l’entreprise est coupé : l’adresse se termine en \.internal/,
        /HostName escouade\.acme\.internal/,
      ],
    ],
    [
      'unknown host key',
      { kind: 'hostKey', changed: false, output: 'Host key verification failed.', command: 'ssh -N …' },
      [/Clé d’hôte inconnue/, /Escouade n’accepte pas de clé à ta place : connecte-toi une fois par ssh acme-escouade/],
    ],
    [
      'no Escouade',
      { kind: 'noEscouade', output: 'channel 2: open failed', command: 'ssh' },
      [/Pas d’Escouade sur le serveur/, /est-il installé et lancé/],
    ],
    [
      'another version',
      { kind: 'version', server: '2.3.0', client: '2.3.2', update: 'server' },
      [/autre version d’Escouade/, /Ce poste a Escouade 2\.3\.2, le serveur 2\.3\.0/, /mettre à jour vers 2\.3\.2/],
    ],
    [
      'the policy',
      { kind: 'policy', message: 'Le serveur x.example est refusé : la stratégie de ce poste ne permet que *.acme.internal.' },
      [/Refusé par la stratégie de ce poste/, /ne permet que \*\.acme\.internal/],
    ],
    [
      'an account refused',
      { kind: 'unreachable', why: 'auth', output: 'Permission denied (publickey).', command: 'ssh' },
      [/refuse la connexion SSH/, /ssh-add/],
    ],
  ];

  for (const [name, problem, says] of states) {
    it(`says what happened and what to do: ${name}`, async () => {
      attempt = () => ({ outcome: 'problem', problem });
      render(ConnectServerModal);
      const dialog = await screen.findByRole('dialog');
      await within(dialog).findAllByRole('radio');
      await userEvent.click(within(dialog).getByRole('button', { name: /Se connecter/ }));
      const shown = await within(dialog).findByRole('alertdialog');
      for (const s of says) expect(shown).toHaveTextContent(s);
      // The command to type is in the code font, as the block of the command (asked of ESC-537, done with ESC-539).
      if (problem.kind === 'hostKey' && !problem.changed) expect(shown.querySelector('p code')).toHaveTextContent(/^ssh acme-escouade$/);
      // The step « Connexion » in red; « Réessayer » and « Modifier l'hôte » there.
      expect(within(dialog).getAllByRole('listitem')[1]).toHaveClass('bad');
      expect(within(dialog).getByRole('button', { name: 'Réessayer' })).toBeVisible();
      await userEvent.click(within(dialog).getByRole('button', { name: 'Modifier l’hôte' }));
      expect(within(dialog).getByText('Quelle machine ?')).toBeVisible();
    });
  }

  it('reads a code typed cell by cell, says a wrong one and an expired one, empties it and goes back to it', async () => {
    let tries = 0;
    pair = () => {
      tries++;
      return tries === 1
        ? { outcome: 'problem', problem: { kind: 'code', reason: 'wrong', message: '' } }
        : tries === 2
          ? { outcome: 'problem', problem: { kind: 'code', reason: 'expired', message: '' } }
          : tries === 3
            ? { outcome: 'problem', problem: { kind: 'code', reason: 'wait', message: '' } }
            : { outcome: 'ok', value: 's1' };
    };
    render(ConnectServerModal);
    const dialog = await screen.findByRole('dialog');
    await within(dialog).findAllByRole('radio');
    await userEvent.click(within(dialog).getByRole('button', { name: /Se connecter/ }));
    const group = await within(dialog).findByRole('group', { name: 'Code d’appairage, 8 caractères' });
    const submit = within(dialog).getByRole('button', { name: /Appairer/ });
    expect(submit).toBeDisabled();
    // Each letter moves to the next cell; a letter of no code (O, 0, 1, I) is not taken.
    await userEvent.keyboard('k7q4O');
    const cells = within(group).getAllByRole('textbox') as HTMLInputElement[];
    expect(cells.map((c) => c.value).join('')).toBe('K7Q4');
    await userEvent.keyboard('mxr9');
    expect(submit).toBeEnabled();
    await userEvent.click(submit);
    expect(backend.called('server_pair')[0].args).toMatchObject({ attempt: 'a1', code: 'K7Q4-MXR9', name: 'acme-escouade' });
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Ce code n’est pas le bon.');
    expect(cells.every((c) => c.value === '')).toBe(true);
    expect(document.activeElement).toBe(cells[0]);
    // A code pasted whole, with its dash.
    await userEvent.paste('ABCD-EFGH');
    expect(cells.map((c) => c.value).join('')).toBe('ABCDEFGH');
    // Backspace goes back a cell.
    await userEvent.keyboard('{Enter}');
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Ce code a expiré : relance escouade-server pair.');
    // Too many wrong tries in the minute (ESC-654): the code still holds, the user is told to wait, not to start over.
    await userEvent.paste('ABCD-EFGH');
    await userEvent.keyboard('{Enter}');
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Trop d’essais faux en une minute : le code vaut toujours, réessaie dans une minute.',
    );
  });

  it('fills the code « le lancer d’ici par SSH », then pairs and opens the space', async () => {
    render(ConnectServerModal);
    const dialog = await screen.findByRole('dialog');
    await within(dialog).findAllByRole('radio');
    await userEvent.click(within(dialog).getByRole('button', { name: /Se connecter/ }));
    await userEvent.click(await within(dialog).findByTestId('pair-over-ssh'));
    const cells = within(within(dialog).getByRole('group')).getAllByRole('textbox') as HTMLInputElement[];
    await vi.waitFor(() => expect(cells.map((c) => c.value).join('')).toBe('K7Q4MXR9'));
    expect(backend.called('server_code_over_ssh')[0].args).toEqual({ attempt: 'a1' });
    const name = within(dialog).getByDisplayValue('acme-escouade');
    await userEvent.clear(name);
    await userEvent.type(name, 'Travail');
    await userEvent.click(within(dialog).getByRole('button', { name: /Appairer/ }));
    expect(backend.called('server_pair')[0].args).toMatchObject({ code: 'K7Q4-MXR9', name: 'Travail', device: 'DESKTOP · Windows' });
    // Then the window goes on it (here refused, said in the dialog).
    expect(await within(dialog).findByText('stop here')).toBeVisible();
    expect(backend.called('server_open')[0].args).toEqual({ id: 's1' });
  });
});

describe('the states of a server, said', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    resetConnection();
    fakeBackend();
  });
  afterEach(cleanup);

  it('says a revoked token: when and from which device, what goes on, and how to pair again', () => {
    render(ServerProblem, {
      props: {
        problem: { kind: 'revoked', revoked: { device: 'Poste', at: Date.UTC(2026, 10, 16, 17, 4), by: 'MacBook-Air' } },
        host: 'acme-escouade',
        name: 'Travail',
      },
    });
    const shown = screen.getByRole('alertdialog');
    expect(shown).toHaveTextContent('Ce poste n’est plus appairé à Travail');
    expect(shown).toHaveTextContent('depuis MacBook-Air');
    expect(shown).toHaveTextContent('Les agents de Travail continuent sur le serveur');
    expect(shown).toHaveTextContent('escouade-server pair');
  });

  it('says the states in English too', () => {
    setLang('en');
    flushSync();
    render(ServerProblem, {
      props: {
        problem: { kind: 'unreachable', why: 'unreachable', output: 'timed out', command: 'ssh' },
        host: 'acme-escouade',
        target: 'escouade.acme.internal',
      },
    });
    expect(screen.getByRole('alertdialog')).toHaveTextContent('acme-escouade isn’t answering');
    expect(screen.getByRole('alertdialog')).toHaveTextContent('Nothing was changed on this computer or on the server.');
    cleanup();
    render(ServerProblem, { props: { problem: { kind: 'hostKey', changed: false, output: '', command: '' }, host: 'bastion-test' } });
    expect(screen.getByRole('alertdialog')).toHaveTextContent(
      'Escouade won’t accept a host key for you: connect once with ssh bastion-test.',
    );
  });

  it('shows the whole space on a revoked token, with the way back to this computer', async () => {
    connection.server = TRAVAIL;
    connection.state = 'refused';
    connection.refusal = { code: 'token', error: '', revoked: { device: 'Poste', at: 1_700_000_000_000 } };
    render(ServerScreen);
    expect(screen.getByRole('alertdialog')).toHaveTextContent('Ce poste n’est plus appairé à Travail');
    expect(screen.getByRole('alertdialog')).toHaveTextContent('depuis le serveur');
    expect(screen.getByRole('button', { name: 'Appairer de nouveau' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Aller sur Ce poste' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Retirer Travail de ce poste' })).toBeVisible();
  });

  it('says another version with the side to update', () => {
    connection.server = TRAVAIL;
    connection.state = 'refused';
    connection.refusal = { code: 'version', error: '', serverVersion: '2.3.0', update: 'server', clientVersion: '2.3.2' };
    render(ServerScreen);
    expect(screen.getByRole('alertdialog')).toHaveTextContent('Travail a une autre version d’Escouade');
    expect(screen.getByRole('alertdialog')).toHaveTextContent('mettre à jour vers 2.3.2');
  });
});

describe('the machine said, and a cut', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    resetConnection();
    fakeBackend();
  });
  afterEach(cleanup);

  it('says the tunnel and its latency in the status bar, and the next try in amber while cut', async () => {
    connection.server = TRAVAIL;
    connection.state = 'open';
    connection.latency = 38;
    render(TunnelItem);
    expect(screen.getByTestId('status-tunnel')).toHaveTextContent('Travail · ssh acme-escouade · 38 ms');
    connection.state = 'lost';
    connection.retryAt = Date.now() + 4000;
    flushSync();
    expect(screen.getByTestId('status-tunnel')).toHaveTextContent(/Travail · reconnexion dans [34] s/);
    expect(screen.getByTestId('status-tunnel')).toHaveClass('cut');
  });

  it('says nothing of a tunnel on this computer', () => {
    render(TunnelItem);
    expect(screen.queryByTestId('status-tunnel')).toBeNull();
  });

  it('names the space in the band of a cut, when, the next try, that the agents go on; then « Reconnecté »', async () => {
    connection.server = TRAVAIL;
    connection.state = 'lost';
    connection.lostAt = new Date(2026, 10, 18, 14, 32).getTime();
    connection.tries = 6;
    connection.retryAt = Date.now() + 4000;
    render(ConnectionBand);
    const band = screen.getByRole('alert');
    expect(band).toHaveTextContent('Connexion à Travail perdue à 14:32');
    expect(band).toHaveTextContent(/Nouvel essai dans [34] s \(6ᵉ\)/);
    expect(band).toHaveTextContent('Les agents continuent sur le serveur ; ce que tu vois date de 14:32.');
    expect(within(band).getByRole('button', { name: 'Aller sur Ce poste' })).toBeVisible();
    connection.state = 'open';
    flushSync();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(app.toasts.map((t) => t.text)).toContain('Reconnecté à Travail');
  });

  it('writes the ordinal of a try and the letter of a space', () => {
    expect(nth(6)).toBe('6ᵉ');
    expect(nth(1)).toBe('1ʳᵉ');
    setLang('en');
    expect(nth(2)).toBe('2nd');
    expect(nth(11)).toBe('11th');
    expect(plateOf(' travail')).toBe('T');
    expect(plateOf('')).toBe('?');
  });
});

describe('Réglages › Serveurs', () => {
  afterEach(cleanup);

  it('lists the servers known with their state and their gesture, and removes one after asking', async () => {
    setLang('fr');
    resetApp();
    resetConnection();
    const list = [
      { ...TRAVAIL, active: false, lastSeen: 1_700_000_000_000 },
      {
        ...TRAVAIL,
        id: 's2',
        name: 'Bastion',
        ssh: 'bastion-test',
        host: 'bastion.example',
        active: false,
        problem: { kind: 'hostKey', changed: false, output: '', command: '' },
      },
      {
        ...TRAVAIL,
        id: 's3',
        name: 'Ancien VPS',
        ssh: 'old-vps',
        host: 'old.example',
        active: false,
        problem: { kind: 'revoked', revoked: { device: 'P', at: 1_700_000_000_000, by: 'MacBook-Air' } },
      },
    ];
    const backend = fakeBackend({
      servers_view: () => ({ servers: list, active: null, tokensInFile: false, thisDevice: 'D' }),
      server_forget: () => true,
    });
    render(ServersPage);
    const rows = await screen.findAllByTestId('server-row');
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent('Travail');
    expect(rows[0]).toHaveTextContent('Pas connecté');
    expect(within(rows[0]).getByRole('button', { name: 'Ouvrir' })).toBeVisible();
    expect(rows[1]).toHaveTextContent('Clé d’hôte inconnue');
    expect(within(rows[1]).getByRole('button', { name: 'Copier la commande' })).toBeVisible();
    expect(rows[2]).toHaveTextContent('Jeton révoqué');
    expect(rows[2]).toHaveTextContent('depuis MacBook-Air');
    await userEvent.click(within(rows[2]).getByRole('button', { name: 'Appairer de nouveau' }));
    expect(app.modal).toEqual({ kind: 'connectServer', existing: 's3' });
    expect(screen.getByText(/gardé dans le trousseau de ce poste, jamais dans les réglages/)).toBeVisible();
    // Remove: asked, then the server revokes this computer.
    await userEvent.click(within(rows[0]).getByRole('button', { name: 'Retirer' }));
    const ask = screen.getByRole('alertdialog', { name: 'Retirer Travail de ce poste ?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Retirer' }));
    expect(backend.called('server_forget')[0].args).toEqual({ id: 's1' });
    await vi.waitFor(() => expect(app.toasts.map((t) => t.text)).toContain('Travail retiré de ce poste'));
    expect(servers.view).not.toBeNull();
  });
});

describe('the spaces in the rail (direction B)', () => {
  afterEach(cleanup);

  it('heads the projects with the open space, and leads to the other one under « + »', async () => {
    setLang('fr');
    resetApp();
    resetConnection();
    servers.view = { servers: [{ ...TRAVAIL, active: false }], active: null, tokensInFile: false, thisDevice: 'D' };
    const backend = fakeBackend({
      server_open: () => ({ outcome: 'problem', problem: { kind: 'revoked' } }),
      servers_view: () => servers.view,
    });
    const RailSpace = (await import('./RailSpace.svelte')).default;
    render(RailSpace, { props: { part: 'head' } });
    // This computer's label under its plate is « Poste » (direction B, ESC-539); its name is said whole.
    expect(screen.getByTestId('rail-space')).toHaveTextContent('Poste');
    expect(screen.getByRole('button', { name: 'Espace ouvert : Ce poste' })).toBeInTheDocument();
    cleanup();
    render(RailSpace, { props: { part: 'others' } });
    const nav = screen.getByRole('navigation', { name: 'Espaces' });
    await userEvent.click(within(nav).getByRole('button', { name: /^Travail,/ }));
    expect(backend.called('server_open')[0].args).toEqual({ id: 's1' });
    // Refused (here revoked): Réglages › Serveurs says why.
    await vi.waitFor(() => expect(app.modal).toEqual({ kind: 'settings', page: 'servers' }));
    cleanup();
    connection.server = TRAVAIL;
    render(RailSpace, { props: { part: 'others' } });
    expect(screen.getByRole('button', { name: /^Ce poste,/ })).toBeVisible();
  });
});
