// Profiles, the settings of each project and the jokers (Escouade 2.2.5): the tunnel of a project, from its VPN profile.
import type { Plugin } from '@escouade-dev/plugin';

// The profile the project chose (a `profile` setting of `"scope": "project"`): its id, or null while it chose none.
// Only the user chooses it, never a `.escouade.yml`.
export function profileOf(p: Plugin, project: string) {
  return p.profiles.of(project, 'profile');
}

// Its simple fields, then its secrets: a configuration on several lines comes whole.
export async function credentials(p: Plugin, profile: string) {
  const fields = await p.profiles.get('vpn', profile);
  const password = await p.secret('password', { profile });
  const ovpn = await p.secret('ovpn', { profile });
  return { server: fields?.server, password, ovpn };
}

// A setting of each project: this machine's value for it, else its `.escouade.yml`'s, else the default.
export async function routes(p: Plugin, project: string) {
  const settings = await p.settings(project);
  return settings.routes;
}

// The configuration in its temporary folder, then the tunnel: `{path:temp}` is that file, `{pid}` the first program.
export async function tunnel(p: Plugin, ovpn: string) {
  await p.fs.write({ root: 'temp', path: 'tunnel.ovpn', text: ovpn });
  const vpn = await p.runTyped({ command: 'unshare', args: ['-Urn', 'openvpn', '--config', 'tunnel.ovpn', '--script-security', '1'] });
  const net = await p.runTyped({ command: 'slirp4netns', args: ['--configure', '--mtu=65520', String(vpn.pid), 'tap0'] });
  return [vpn, net];
}
