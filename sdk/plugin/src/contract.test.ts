// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { createPlugin, ERRORS, ESCOUADE_MIN, RpcError, type Manifest } from './index';
import { connect, FAKE_FOLDERS } from './testing';

// The contract of 2.2.5 (ESC-531): profiles, the settings of each project, the jokers of permissions.commands and
// limits.processes, as the SDK asks for them and as its fake Escouade serves them, by the app's rules
// (crates/escouade-plugins: runtime.rs, profiles.rs, procs.rs).

const ACME = '0123456789abcdef';
const PERSO = 'fedcba9876543210';

/** The VPN plugin of issues #38 to #42, as its author would write it. */
const MANIFEST = {
  id: 'dev.example.vpn',
  name: 'VPN',
  version: '1.0.0',
  escouade: { api: '1', min: ESCOUADE_MIN },
  run: { js: 'dist/plugin.mjs' },
  permissions: {
    events: ['agent.started'],
    secrets: ['profile:vpn', 'token'],
    files: [{ root: 'temp', access: 'write' }],
    commands: [
      'unshare -Urn openvpn --config {path:temp} --script-security 1',
      'slirp4netns --configure --mtu=65520 {pid} tap0',
      'tunnel --port {port} --name {name} --retries {int}',
      'tunnel --rules {path:plugin}',
    ],
  },
  limits: { processes: 2, reason: 'A tunnel and its network.' },
  contributes: {
    secrets: [{ key: 'token', title: 'Token' }],
    profiles: [
      {
        id: 'vpn',
        title: 'VPN profile',
        nameField: 'name',
        fields: [
          { key: 'name', type: 'string', title: 'Name' },
          { key: 'server', type: 'string', title: 'Server' },
          { key: 'proto', type: 'select', title: 'Protocol', options: ['udp', 'tcp'], default: 'udp' },
          { key: 'password', secret: true, title: 'Password' },
          { key: 'ovpn', secret: true, multiline: true, import: ['.ovpn'], title: 'Configuration' },
        ],
      },
    ],
    settings: [
      { key: 'profile', type: 'profile', of: 'vpn', scope: 'project', title: 'VPN profile' },
      { key: 'routes', type: 'string', scope: 'project', title: 'Routes', default: '10.0.0.0/8' },
      { key: 'mtu', type: 'number', scope: 'project', title: 'MTU', repo: false },
      { key: 'verbose', type: 'boolean', title: 'Verbose' },
    ],
  },
} as unknown as Manifest;

const OVPN = 'client\nremote vpn.acme.example 1194\n<ca>\n-----BEGIN CERTIFICATE-----\n</ca>\n';

function vpn(options: Parameters<typeof connect>[1] = {}) {
  const plugin = createPlugin();
  const fake = connect(plugin, {
    manifest: MANIFEST,
    version: ESCOUADE_MIN,
    profiles: {
      vpn: [
        {
          id: ACME,
          fields: { name: ' Acme ', server: 'vpn.acme.example', proto: 'tcp', password: 'leak?' },
          secrets: { password: 'hunter2', ovpn: OVPN },
        },
        { id: PERSO, fields: { server: 'vpn.perso.example' } },
      ],
    },
    settings: { verbose: true, routes: 'kept before it was a project’s' },
    projectSettings: { atlas: { routes: '10.1.0.0/16', mtu: 1200, profile: 'Acme', verbose: false } },
    projectValues: { atlas: { profile: ACME }, borealis: { profile: PERSO, mtu: 1400 } },
    files: { 'rules.json': '{}' },
    ...options,
  });
  return { plugin, fake };
}

const refused = async (p: Promise<unknown>) => {
  const e = await p.then(
    () => null,
    (e: unknown) => e,
  );
  expect(e).toBeInstanceOf(RpcError);
  return e as RpcError;
};

describe('the profiles and the settings of each project', () => {
  it('gives the profile a project chose, its fields and its secrets', async () => {
    const { plugin, fake } = vpn();
    await fake.initialize();
    expect(await plugin.profiles.of('atlas', 'profile')).toBe(ACME);
    expect(await plugin.profiles.list('vpn')).toEqual([
      { id: ACME, name: 'Acme' },
      { id: PERSO, name: PERSO },
    ]);
    // Never a secret field among the simple ones.
    expect(await plugin.profiles.get('vpn', ACME)).toEqual({ name: ' Acme ', server: 'vpn.acme.example', proto: 'tcp' });
    expect(await plugin.profiles.get('vpn', '00000000000000aa')).toBeNull();
    expect(await plugin.secret('password', { profile: ACME })).toBe('hunter2');
    // A secret on several lines comes whole.
    expect(await plugin.secret('ovpn', { profile: ACME })).toBe(OVPN);
    expect(await plugin.secret('ovpn', { profile: PERSO })).toBeNull();
    expect(fake.secretsAsked).toEqual([`password@${ACME}`, `ovpn@${ACME}`, `ovpn@${PERSO}`]);
  });

  it('goes by the ranks: this machine for the project, then its .escouade.yml, then the default', async () => {
    const { plugin, fake } = vpn();
    await fake.initialize();
    // The file gives routes, not mtu (repo: false), never a profile; the user's value of the app's scope stays under it.
    expect(await plugin.settings('atlas')).toEqual({ verbose: false, routes: '10.1.0.0/16', profile: ACME });
    expect(await plugin.settings('borealis')).toEqual({ verbose: true, routes: '10.0.0.0/8', mtu: 1400, profile: PERSO });
    // Without a project, or where it is off: the defaults alone, no profile.
    expect(await plugin.settings()).toEqual({ verbose: true, routes: '10.0.0.0/8', profile: null });
    const off = vpn({ off: ['atlas'] });
    await off.fake.initialize();
    expect(await off.plugin.settings('atlas')).toEqual({ verbose: true, routes: '10.0.0.0/8', profile: null });
    expect(await off.plugin.profiles.of('atlas', 'profile')).toBeNull();
  });

  it('a profile gone or a project that chose none is null', async () => {
    const { plugin, fake } = vpn({ projectValues: { atlas: { profile: '0000000000000000' } } });
    await fake.initialize();
    expect(await plugin.profiles.of('atlas', 'profile')).toBeNull();
    expect(await plugin.profiles.of('nowhere', 'profile')).toBeNull();
  });

  it('refuses what the app refuses, and says it in its refusals', async () => {
    const { plugin, fake } = vpn();
    await fake.initialize();
    // A field of a profile without its profile: never the plugin's own secret of that key.
    expect((await refused(plugin.secret('password'))).code).toBe(ERRORS.invalidParams);
    expect((await refused(plugin.secret('password', { profile: 'Acme' }))).message).toContain('is no profile');
    expect((await refused(plugin.secret('password', { profile: '00000000000000aa' }))).message).toContain('no profile');
    expect((await refused(plugin.secret('server', { profile: ACME }))).message).toContain('no secret field');
    expect((await refused(plugin.profiles.list('wifi'))).code).toBe(ERRORS.invalidParams);
    expect(await plugin.secret('token')).toBeNull();
    // Without profile:vpn in permissions.secrets, its profiles' secrets are not its own.
    const without = vpn({ allowed: ['token'] });
    await without.fake.initialize();
    expect((await refused(without.plugin.secret('password', { profile: ACME }))).code).toBe(ERRORS.notYours);
    expect(fake.refusals.length).toBe(5);
  });

  it('never asks an Escouade older than the contract: it would answer another secret', async () => {
    const { plugin, fake } = vpn({ version: '2.2.4', capabilities: { profiles: false, typedCommands: false } });
    await fake.initialize();
    for (const call of [
      () => plugin.secret('password', { profile: ACME }),
      () => plugin.profiles.list('vpn'),
      () => plugin.profiles.get('vpn', ACME),
      () => plugin.profiles.of('atlas', 'profile'),
      () => plugin.runTyped({ command: 'tunnel', args: ['--rules', 'rules.json'] }),
    ]) {
      const e = await refused(call());
      expect(e.code).toBe(ERRORS.methodNotFound);
      expect(e.message).toContain('(2.2.4)');
      expect(e.message).toContain(`Escouade ${ESCOUADE_MIN} does`);
    }
    expect(fake.secretsAsked).toEqual([]);
    expect(fake.received.filter((m) => (m as { method?: string }).method)).toEqual([]);
    // A plain secret still goes.
    expect(await plugin.secret('token')).toBeNull();
  });
});

describe('the jokers and the limit', () => {
  it('runs an entry of jokers, each one word of its form, the paths the host’s, and gives the pid', async () => {
    const { plugin, fake } = vpn();
    await fake.initialize();
    await plugin.fs.write({ root: 'temp', path: 'atlas.ovpn', text: OVPN });
    const vpnProgram = await plugin.runTyped({
      command: 'unshare',
      args: ['-Urn', 'openvpn', '--config', 'atlas.ovpn', '--script-security', '1'],
    });
    expect(fake.spawned[0].words).toEqual([
      'unshare',
      '-Urn',
      'openvpn',
      '--config',
      `${FAKE_FOLDERS.temp}/atlas.ovpn`,
      '--script-security',
      '1',
    ]);
    const net = await plugin.runTyped({ command: 'slirp4netns', args: ['--configure', '--mtu=65520', String(vpnProgram.pid), 'tap0'] });
    expect(net.pid).toBeGreaterThan(vpnProgram.pid);
    expect([...fake.programs.keys()]).toEqual([vpnProgram.proc, net.proc]);
  });

  it('refuses a word out of its form, a pid not its own, and a third program past its limit of 2', async () => {
    const { plugin, fake } = vpn();
    await fake.initialize();
    const no = (args: string[]) => refused(plugin.runProgram({ command: 'tunnel', args }));
    expect((await no(['--port', '0', '--name', 'a', '--retries', '1'])).message).toContain('out of 1 to 65535');
    expect((await no(['--port', '80', '--name', '-x', '--retries', '1'])).message).toContain('an option');
    expect((await no(['--port', '80', '--name', 'a/b', '--retries', '1'])).message).toContain('not 1 to 64');
    expect((await no(['--port', '80', '--name', 'NUL', '--retries', '1'])).message).toContain('a device of Windows');
    expect((await no(['--port', '80', '--name', 'a', '--retries', '01'])).message).toContain('leading zero');
    expect((await no(['--port', '80', '--name', 'a', '--retries', '1', 'more'])).message).toContain('8 words where the entry has 7');
    expect((await no(['--rules', '../escouade-plugin.json'])).message).toContain('a part . or ..');
    expect((await no(['--rules', 'missing.json'])).message).toContain('no such file');
    const slirp = (pid: string) =>
      refused(plugin.runProgram({ command: 'slirp4netns', args: ['--configure', '--mtu=65520', pid, 'tap0'] }));
    expect((await slirp('1')).message).toContain('no program it runs is alive with this pid');
    // Its temporary folder is where it runs when `dir` is left out: a dir is a root, never a path.
    const dir = await refused(plugin.runProgram({ command: 'tunnel', args: ['--rules', 'rules.json'], dir: 'temp' as 'agent' }));
    expect(dir.message).toBe('dir is agent or project, never a path: "temp"');
    const one = await plugin.runTyped({ command: 'tunnel', args: ['--rules', 'rules.json'] });
    await plugin.runTyped({ command: 'tunnel', args: ['--port', '8080', '--name', 'atlas', '--retries', '3'] });
    const third = await refused(plugin.runProgram({ command: 'tunnel', args: ['--rules', 'rules.json'] }));
    expect(third.message).toBe('2 programs at once at most for this plugin (limits.processes)');
    // Its end gives its place back, and its pid no longer passes.
    const ended = one.exited;
    await fake.exit(one.proc, 0);
    expect(await ended).toBe(0);
    expect((await slirp(String(one.pid))).message).toContain('no program it runs is alive');
    await plugin.runTyped({ command: 'tunnel', args: ['--rules', 'rules.json'] });
    expect(fake.spawned).toHaveLength(3);
  });

  it('holds 4 programs without limits.processes, and an Escouade without the jokers runs none of them', async () => {
    const { limits: _, ...rest } = MANIFEST as unknown as Record<string, unknown>;
    const { plugin, fake } = vpn({
      manifest: { ...rest, permissions: { commands: ['git --version', 'tunnel --rules {path:plugin}'] } } as unknown as Manifest,
    });
    await fake.initialize();
    for (let i = 0; i < 4; i++) await plugin.runProgram({ command: 'git', args: ['--version'] });
    expect((await refused(plugin.runProgram({ command: 'git', args: ['--version'] }))).message).toContain('4 programs at once');
    const old = vpn({ capabilities: { typedCommands: false } });
    await old.fake.initialize();
    const p = await old.plugin.runProgram({ command: 'slirp4netns', args: ['--configure', '--mtu=65520', '1', 'tap0'] }).catch((e) => e);
    expect(p).toBeInstanceOf(RpcError);
    expect((p as RpcError).message).toContain('a word no command matches');
  });
});
