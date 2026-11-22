import { defineZone } from '../types';

// Settings › Phone (ESC-567): see the French catalog.
export default defineZone('phone', {
  intro:
    'Your phone reaches {where} through a WireGuard tunnel built into Escouade: no VPN nor administrator rights on the machine, a single UDP port open, and the API is served inside the tunnel alone, behind each device’s token.',
  whereLocal: 'this computer',
  whereServer: 'this server',
  access: {
    title: 'Access',
    label: 'Access from the phone',
    labelDesc: 'On, the tunnel waits for your phones on the UDP port below. Off, nothing listens.',
  },
  port: {
    label: 'UDP port',
    labelDesc: 'The one the box forwards to this machine; 51820 by default, WireGuard’s.',
    apply: 'Apply',
    invalid: 'A port from 1 to 65535.',
  },
  state: {
    title: 'State',
    off: 'Off',
    starting: 'Starting…',
    on: 'Ready on UDP port {port}',
    error: 'Does not start',
    api: 'API in the tunnel: {api}',
    phones: { one: '{count} phone', other: '{count} phones' },
  },
  box: {
    title: 'The box',
    opened: 'UDP port {port} opened in the box by {by}, renewed while the access is on.',
    todo: 'In the box, forward UDP port {port} to {local} (this machine): Escouade could not open it itself.',
    todoNoLocal: 'In the box, forward UDP port {port} to this machine: Escouade could not open it itself.',
    why: 'The box answered: {error}',
    cgnat:
      'Your provider shares your IPv4 address (CGNAT), or your box is behind another one: a forward does nothing from the Internet there.',
    cgnatWays:
      'The ways out: ask the provider for a dedicated IPv4 (often free), go through the box’s IPv6, Tailscale, or an Escouade server on a VPS.',
    ipv6: 'This machine has a public IPv6 ({ip}): also open UDP port {port} in the box’s IPv6 firewall.',
    server: 'On a server, no box: open UDP port {port} in its firewall (for instance: ufw allow {port}/udp).',
    direct: 'This machine has the public address itself: open UDP port {port} in its firewall.',
    unchecked: 'The network has not been looked at yet.',
    checking: 'Looking at the network…',
    probesOff: 'The network is not looked at here (tests).',
    wanLabel: 'The box’s address on the Internet',
  },
  public: {
    label: 'Public address',
    unknown: 'unknown',
    changed: 'Changed on {date} (before: {before}): a phone paired before it must take the new one.',
    checked: 'Seen {when}',
    check: 'Look at the network again',
  },
  tailscale: {
    title: 'Tailscale',
    found: 'Tailscale is on this machine ({address}): a phone of your tailnet goes through it, with no port to open, CGNAT included.',
    absent: 'Tailscale is not on this machine. If it comes, a phone of your tailnet will go through it, with no port to open.',
    label: 'Also serve on the Tailscale address',
    labelDesc: 'For a phone of your tailnet alone; the device token is still asked.',
    served: 'Served on {addr}',
  },
  phones: {
    title: 'Phones',
    none: 'No phone paired. Escouade’s app for Android and iOS will pair them with a QR code.',
    up: 'Tunnel open · last handshake {ago}',
    idle: 'Tunnel at rest',
    address: 'Address in the tunnel: {address}',
    remove: 'Remove',
    removed: '{name} removed and its access revoked',
    removedUnlinked: '{name} removed from the tunnel. It was tied to no device: no token to revoke.',
  },
  advanced: {
    key: 'The tunnel’s public key: {key}',
    rotate: 'Change the tunnel’s key',
    rotateDesc: 'Every phone will have to be paired again: the old key is no longer recognized.',
    rotateButton: 'Change the key',
    rotateTitle: 'Change the tunnel’s key?',
    rotateText: 'The paired phones no longer reach {where} until they have the new key.',
    rotated: 'New tunnel key',
  },
});
