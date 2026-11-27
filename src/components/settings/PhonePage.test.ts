// Réglages › Téléphone (ESC-567): on and off, the port, the tunnel's state, what to do in the box, a CGNAT and its
// ways out, the public address that changed, a server's firewall, Tailscale, the phones, the key turned, and the policy
// of the device that greys the switch.

import { cleanup, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { locks, NO_LOCKS } from '../../lib/locks.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend } from '../../test/ipc';
import type { Locks } from '../../lib/types';
import PhonePage from './PhonePage.svelte';

type Status = Record<string, any>;

const base = (over: Status = {}): Status => ({
  enabled: false,
  state: 'off',
  error: null,
  denied: null,
  port: 51820,
  defaultPort: 51820,
  bound: null,
  tunnelAddress: '10.77.0.1',
  api: 'ws://10.77.0.1/remote',
  publicKey: 'cHVibGljLWtleS1vZi10aGUtdHVubmVsLWZvci10ZXN0cw==',
  server: false,
  net: null,
  checking: false,
  checkedAt: null,
  public: null,
  tailscale: null,
  tailscaleOn: false,
  tailscaleServed: null,
  stunOn: true,
  stunServers: ['stun.cloudflare.com:3478', 'stun.l.google.com:19302'],
  peers: [],
  handshakes: 0,
  unknown: 0,
  ...over,
});

const net = (over: Status = {}): Status => ({
  local: '192.168.1.20',
  ipv6: null,
  gateway: '192.168.1.1',
  opened: null,
  openError: 'no box answered',
  wan: null,
  public: '203.0.113.7',
  cgnat: false,
  direct: false,
  probed: true,
  ...over,
});

const on = (over: Status = {}): Status =>
  base({ enabled: true, state: 'on', bound: '[::]:51820', net: net(), checkedAt: Date.now(), ...over });

let status: Status;
let backend: ReturnType<typeof fakeBackend>;

function serve(extra: Record<string, (args: any) => unknown> = {}) {
  backend = fakeBackend({
    phone_status: () => status,
    phone_set: (a) => {
      if (a.enabled !== undefined && a.enabled !== null) status = a.enabled ? on() : base();
      if (a.port) status = { ...status, port: a.port };
      if (a.tailscale !== undefined && a.tailscale !== null) status = { ...status, tailscaleOn: a.tailscale };
      if (a.stun !== undefined && a.stun !== null) status = { ...status, stunOn: a.stun };
      return status;
    },
    phone_check: () => status,
    phone_rotate_key: () => ({ ...status, publicKey: 'bmV3LWtleQ==' }),
    phone_remove: (a) => ({ removed: true, revoked: a.id === 'p1', neverLinked: a.id !== 'p1' }),
    ...extra,
  });
}

const callsOf = (cmd: string) => backend.calls.filter((c) => c.cmd === cmd);

beforeEach(() => {
  setLang('fr');
  locks.current = NO_LOCKS;
  status = base();
  app.toasts = [];
  serve();
});
afterEach(() => {
  cleanup();
  locks.current = NO_LOCKS;
});

describe('Réglages › Téléphone', () => {
  it('turns the access on from the switch, and says the tunnel ready on its port', async () => {
    render(PhonePage);
    expect(await screen.findByTestId('phone-state')).toHaveTextContent('Éteint');
    expect(screen.getByText(/joint ce poste par un tunnel WireGuard/)).toBeVisible();
    const sw = screen.getByRole('switch', { name: 'Accès depuis le téléphone' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    await userEvent.click(sw);
    expect(callsOf('phone_set').at(-1)?.args).toMatchObject({ enabled: true });
    expect(await screen.findByText('Prêt sur le port UDP 51820')).toBeVisible();
    expect(screen.getByText('API dans le tunnel : ws://10.77.0.1/remote')).toBeVisible();
    expect(screen.getByRole('switch', { name: 'Accès depuis le téléphone' })).toHaveAttribute('aria-checked', 'true');
  });

  it('says what to do in the box when it could not open the port, and a CGNAT with its ways out', async () => {
    status = on({ net: net({ cgnat: true, wan: '100.72.3.9' }) });
    render(PhonePage);
    expect(await screen.findByTestId('phone-box-todo')).toHaveTextContent(
      'Dans la box, redirige le port UDP 51820 vers 192.168.1.20 (cette machine)',
    );
    expect(screen.getByText('La box a répondu : no box answered')).toBeVisible();
    const cgnat = screen.getByTestId('phone-cgnat');
    expect(cgnat).toHaveTextContent('CGNAT');
    expect(cgnat).toHaveTextContent('demander une IPv4 dédiée');
    expect(screen.getByTestId('phone-public')).toHaveTextContent('203.0.113.7');
    expect(screen.getByText('100.72.3.9')).toBeVisible();
  });

  it('says the port opened by UPnP, and the public address that changed', async () => {
    status = on({
      net: net({ opened: { by: 'upnp', externalPort: 51820, lease: 3600 }, openError: null }),
      public: { address: '198.51.100.23', since: 2, previous: '203.0.113.7', changedAt: Date.UTC(2026, 10, 20, 9) },
    });
    render(PhonePage);
    expect(await screen.findByText(/Port UDP 51820 ouvert dans la box par UPnP/)).toBeVisible();
    expect(screen.queryByTestId('phone-box-todo')).toBeNull();
    expect(screen.getByTestId('phone-public')).toHaveTextContent('198.51.100.23');
    expect(screen.getByTestId('phone-changed')).toHaveTextContent('avant : 203.0.113.7');
  });

  it('on a server, says its firewall rather than a box', async () => {
    status = on({ server: true });
    render(PhonePage);
    expect(await screen.findByText(/ufw allow 51820\/udp/)).toBeVisible();
    expect(screen.getByText(/joint ce serveur par un tunnel WireGuard/)).toBeVisible();
    expect(screen.queryByTestId('phone-box-todo')).toBeNull();
  });

  it('refuses a port out of range, and applies another one', async () => {
    status = on();
    render(PhonePage);
    const field = await screen.findByRole('spinbutton', { name: 'Port UDP' });
    await userEvent.clear(field);
    await userEvent.type(field, '0');
    await userEvent.click(screen.getByRole('button', { name: 'Appliquer' }));
    expect(screen.getByText('Un port de 1 à 65535.')).toBeVisible();
    expect(callsOf('phone_set')).toHaveLength(0);
    await userEvent.clear(field);
    await userEvent.type(field, '51900');
    await userEvent.click(screen.getByRole('button', { name: 'Appliquer' }));
    expect(callsOf('phone_set').at(-1)?.args).toMatchObject({ port: 51900 });
  });

  it('says who STUN asks, when and what they see, and turns it off', async () => {
    render(PhonePage);
    const sw = await screen.findByRole('switch', { name: 'Demander l’adresse publique à STUN' });
    // Said before the access is even on.
    expect(
      screen.getByText(
        /toutes les 5 minutes, ce poste demande son adresse publique à stun\.cloudflare\.com et stun\.l\.google\.com\s: ils voient ton adresse IP/,
      ),
    ).toBeVisible();
    expect(screen.getByText(/la box \(UPnP, NAT-PMP\) et l’adresse donnée à la main servent encore/)).toBeVisible();
    expect(sw).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(sw);
    expect(callsOf('phone_set').at(-1)?.args).toMatchObject({ stun: false });
    await waitFor(() => expect(sw).toHaveAttribute('aria-checked', 'false'));
  });

  it('shows Tailscale found but off until it is turned on (SR1-05)', async () => {
    status = on({ tailscale: { address: '100.101.102.103', name: null } });
    render(PhonePage);
    const sw = await screen.findByRole('switch', { name: 'Servir aussi sur l’adresse Tailscale' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText(/Éteint tant que tu ne l’allumes pas/)).toBeVisible();
    expect(screen.queryByText(/Servie sur/)).toBeNull();
    await userEvent.click(sw);
    expect(callsOf('phone_set').at(-1)?.args).toMatchObject({ tailscale: true });
  });

  it('offers Tailscale when it is on the machine, and turns it off', async () => {
    status = on({
      tailscaleOn: true,
      tailscale: { address: '100.101.102.103', name: 'poste.tail1234.ts.net' },
      tailscaleServed: '100.101.102.103:51820',
    });
    render(PhonePage);
    expect(await screen.findByText(/Tailscale est sur cette machine \(100\.101\.102\.103 · poste\.tail1234\.ts\.net\)/)).toBeVisible();
    expect(screen.getByText('Servie sur 100.101.102.103:51820')).toBeVisible();
    await userEvent.click(screen.getByRole('switch', { name: 'Servir aussi sur l’adresse Tailscale' }));
    expect(callsOf('phone_set').at(-1)?.args).toMatchObject({ tailscale: false });
  });

  it('lists the phones with their tunnel, and removes one', async () => {
    status = on({
      peers: [
        { id: 'p1', name: 'Pixel 8', address: '10.77.0.2', device: 'd1', addedAt: 1, sinceHandshake: 4000 },
        { id: 'p2', name: 'iPhone', address: '10.77.0.3', device: null, addedAt: 1, sinceHandshake: null },
      ],
    });
    render(PhonePage);
    const rows = await screen.findAllByTestId('phone-row');
    expect(rows[0]).toHaveTextContent('Tunnel ouvert · dernière poignée de main à l’instant');
    expect(rows[1]).toHaveTextContent('Tunnel au repos · Adresse dans le tunnel : 10.77.0.3');
    // A phone paired as a device: its token revoked with its peer, said so (SR1-03).
    await userEvent.click(within(rows[0]).getByRole('button', { name: 'Retirer' }));
    expect(callsOf('phone_remove').at(-1)?.args).toEqual({ id: 'p1' });
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Pixel 8 retiré et son accès révoqué']));
    // A peer never paired as a device: no token to revoke, said too.
    app.toasts = [];
    await userEvent.click(within(rows[1]).getByRole('button', { name: 'Retirer' }));
    expect(callsOf('phone_remove').at(-1)?.args).toEqual({ id: 'p2' });
    await waitFor(() =>
      expect(app.toasts.map((t) => t.text)).toEqual(['iPhone retiré du tunnel. Il n’était lié à aucun appareil : aucun jeton à révoquer.']),
    );
  });

  it('says there is no phone yet, the pairing coming with the app', async () => {
    status = on();
    render(PhonePage);
    expect(await screen.findByText(/Aucun téléphone appairé/)).toBeVisible();
  });

  it('turns the tunnel’s key after a confirmation', async () => {
    status = on();
    render(PhonePage);
    await userEvent.click(await screen.findByRole('button', { name: /Avancé/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Changer la clé' }));
    const dialog = screen.getByRole('alertdialog', { name: 'Changer la clé du tunnel ?' });
    expect(dialog).toHaveTextContent('ne joignent plus ce poste');
    expect(callsOf('phone_rotate_key')).toHaveLength(0);
    await userEvent.click(within(dialog).getByRole('button', { name: 'Changer la clé' }));
    expect(callsOf('phone_rotate_key')).toHaveLength(1);
  });

  it('greys the switch with who imposes it when the policy of the device forbids the phone', async () => {
    const policy: Locks = {
      locks: {
        'servers.phone': { key: 'servers.phone', value: false, by: { kind: 'device' }, bounds: [], layers: [] },
      },
      rows: { phoneAccess: ['servers.phone'] },
      warnings: [],
    } as unknown as Locks;
    locks.current = policy;
    status = base({ denied: 'L’accès depuis le téléphone est interdit par la stratégie de ce poste (servers.phone).' });
    render(PhonePage);
    expect(await screen.findByText('Imposé par la stratégie de ce poste')).toBeVisible();
    const sw = screen.getByRole('switch', { name: 'Accès depuis le téléphone' });
    expect(sw).toBeDisabled();
  });

  it('says why the access does not start', async () => {
    status = base({
      enabled: true,
      state: 'error',
      error: 'Le port UDP 51820 est déjà pris sur cette machine (un autre WireGuard ?) : choisis-en un autre.',
    });
    render(PhonePage);
    expect(await screen.findByText('Ne démarre pas')).toBeVisible();
    expect(screen.getByText(/déjà pris sur cette machine/)).toBeVisible();
  });

  it('speaks English', async () => {
    setLang('en');
    status = on({ net: net({ cgnat: true }) });
    render(PhonePage);
    expect(await screen.findByTestId('phone-box-todo')).toHaveTextContent('In the box, forward UDP port 51820 to 192.168.1.20');
    expect(screen.getByTestId('phone-cgnat')).toHaveTextContent('ask the provider for a dedicated IPv4');
    expect(screen.getByText(/reaches this computer through a WireGuard tunnel/)).toBeVisible();
  });

  it('shows a phone’s QR code ten minutes with its code, then hides it once the phone paired through it', async () => {
    status = on();
    const expiresAt = Date.now() + 600_000;
    serve({
      phone_pair_qr: () => {
        status = on({
          peers: [{ id: 'p9', name: 'Téléphone', address: '10.77.0.2', device: null, addedAt: 1, sinceHandshake: null, pending: true }],
        });
        return {
          svg: '<svg data-testid="qr-svg"></svg>',
          code: 'K7Q2-9MXA',
          expiresAt,
          peer: 'p9',
          endpoint: '203.0.113.7:51820',
          endpointUnknown: false,
        };
      },
    });
    render(PhonePage);
    await userEvent.click(await screen.findByRole('button', { name: /Appairer un téléphone/ }));
    const shown = await screen.findByRole('region', { name: 'QR code d’appairage' });
    expect(within(shown).getByTestId('pair-qr-code')).toHaveTextContent('K7Q2-9MXA');
    expect(within(shown).getByTestId('pair-qr-left')).toHaveTextContent(/Valable encore (9 min|10 min)/);
    expect(shown).toHaveTextContent('Le téléphone joindra 203.0.113.7:51820.');
    expect(shown).toHaveTextContent('ne le photographie pas');
    // Its peer waits, said so in the list.
    await waitFor(() => expect(screen.getByTestId('phone-row')).toHaveTextContent('En attente de son QR code'));
    // The phone paired through it: the QR leaves the screen.
    status = on({
      peers: [{ id: 'p9', name: 'Pixel 8', address: '10.77.0.2', device: 'd9', addedAt: 1, sinceHandshake: 1000, pending: false }],
    });
    await waitFor(() => expect(screen.queryByRole('region', { name: 'QR code d’appairage' })).toBeNull(), { timeout: 6000 });
    expect(screen.getByRole('status')).toHaveTextContent('Téléphone appairé');
    expect(callsOf('phone_pair_qr')).toHaveLength(1);
  });

  it('says why no QR code when the core refuses it', async () => {
    status = on();
    serve({
      phone_pair_qr: () => {
        throw new Error('L’accès du téléphone est éteint');
      },
    });
    render(PhonePage);
    await userEvent.click(await screen.findByRole('button', { name: /Appairer un téléphone/ }));
    expect(await screen.findByText(/L’accès du téléphone est éteint/)).toBeVisible();
  });
});
