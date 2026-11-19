#!/usr/bin/env node
// Test double of OpenSSH's `ssh` for « Se connecter à un serveur » (ESC-537): the Rust tests (`servers_tests`) and the
// e2e (`tests/e2e/server-connect.spec.ts`) run it in place of the system's, never a real SSH.
//
// - `ssh -N -L 127.0.0.1:<local>:127.0.0.1:<remote> … <host>`: a local forward, as `ssh -L` makes it, to 127.0.0.1 of
//   this machine (the « server »), its remote port mapped by FAKE_SSH_PORTS (`46480=4122,…`; unmapped: the same port).
//   FAKE_SSH_PORTS_FILE: more of them, a line `<remote>=<real>` each, read at each connection (a port a program of
//   the « server » took after the forward started: the fake `codex login`'s, ESC-538).
//   It says what `ssh -v` says once ready (« Authenticated to … », « Local forwarding listening on … »); a connection
//   nothing answers says « channel 2: open failed: connect failed: Connection refused », as `ssh` does.
// - `ssh … <host> <command>`: `escouade-server pair` runs FAKE_SSH_SERVER `pair --data-dir FAKE_SSH_DATA` and prints
//   its output; without FAKE_SSH_SERVER, « escouade-server: not found » (127).
// - The host says the failure it plays: `…inconnu…` an unknown host key, `…change…` a changed one, `…injoignable…` a
//   timed out connection, `…refuse…` an account refused.
// - FAKE_SSH_DIR: each run appends its arguments to `runs.log` there; while `down` exists there, every run fails as an
//   unreachable host; when `cut` appears there, the running forwards end (« closed by remote host ») and `cut` goes.
//   `down-<host>` and `cut-<host>` do the same for that host alone (ESC-539: a cut of one space never touches another).
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';

const argv = process.argv.slice(2);
const dir = process.env.FAKE_SSH_DIR || '';
if (dir) {
  fs.mkdirSync(dir, { recursive: true });
  fs.appendFileSync(path.join(dir, 'runs.log'), JSON.stringify(argv) + '\n');
}

// The options, then the host, then the command.
let forward = null;
let noCommand = false;
const options = [];
let i = 0;
for (; i < argv.length; i++) {
  const a = argv[i];
  if (a === '-N') noCommand = true;
  else if (a === '-v') continue;
  else if (a === '-o') options.push(argv[++i]);
  else if (a === '-p') i++;
  else if (a === '-L') forward = argv[++i];
  else if (a.startsWith('-')) continue;
  else break;
}
const host = argv[i] || '';
const command = argv.slice(i + 1).join(' ');
const say = (line) => process.stderr.write(line + '\n');
const fail = (lines, code = 255) => {
  for (const l of lines) say(l);
  process.exit(code);
};

// Never asked to accept a key, never asked a question.
if (!options.includes('BatchMode=yes') || !options.includes('StrictHostKeyChecking=yes')) {
  fail(['fake-ssh: BatchMode=yes and StrictHostKeyChecking=yes are required'], 2);
}

if (dir && (fs.existsSync(path.join(dir, 'down')) || fs.existsSync(path.join(dir, `down-${host}`)))) {
  setTimeout(() => fail([`ssh: connect to host ${host} port 22: Connection timed out`]), 200);
} else if (host.includes('inconnu')) {
  fail([`No ED25519 host key is known for ${host} and you have requested strict checking.`, 'Host key verification failed.']);
} else if (host.includes('change')) {
  fail([
    '@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@',
    '@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @',
    '@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@',
    'Host key verification failed.',
  ]);
} else if (host.includes('injoignable')) {
  setTimeout(() => fail([`ssh: connect to host 10.255.0.1 port 22: Connection timed out`]), 300);
} else if (host.includes('refuse')) {
  fail([`escouade@${host}: Permission denied (publickey).`]);
} else if (!noCommand) {
  // A command on the host.
  if (!/escouade-server/.test(command)) fail([`fake-ssh: unknown command ${command}`], 1);
  const server = process.env.FAKE_SSH_SERVER;
  if (!server) fail(['sh: 1: escouade-server: not found'], 127);
  const args = ['pair'];
  if (process.env.FAKE_SSH_DATA) args.push('--data-dir', process.env.FAKE_SSH_DATA);
  const run = spawnSync(server, args, { encoding: 'utf8', env: process.env, shell: /\.(cmd|bat)$/i.test(server) });
  process.stdout.write(run.stdout || '');
  process.stderr.write(run.stderr || '');
  process.exit(run.status ?? 1);
} else {
  // The forward: 127.0.0.1:<local> to 127.0.0.1:<remote, mapped>.
  const m = /^(?:127\.0\.0\.1:)?(\d+):127\.0\.0\.1:(\d+)$/.exec(forward || '');
  if (!m) fail([`fake-ssh: bad forward ${forward}`], 2);
  const local = Number(m[1]);
  const ports = Object.fromEntries(
    (process.env.FAKE_SSH_PORTS || '')
      .split(',')
      .filter(Boolean)
      .map((p) => p.split('=').map(Number)),
  );
  /** The real port of the remote one, read again at each connection (FAKE_SSH_PORTS_FILE may have grown). */
  const remoteOf = () => {
    const file = process.env.FAKE_SSH_PORTS_FILE;
    let more = {};
    try {
      if (file)
        more = Object.fromEntries(
          fs
            .readFileSync(file, 'utf8')
            .split(/\r?\n/)
            .filter(Boolean)
            .map((l) => l.split('=').map(Number)),
        );
    } catch {
      // not there yet
    }
    return ports[Number(m[2])] ?? more[Number(m[2])] ?? Number(m[2]);
  };
  const sockets = new Set();
  let channel = 1;
  const server = net.createServer((client) => {
    channel++;
    const upstream = net.connect({ host: '127.0.0.1', port: remoteOf() });
    sockets.add(client);
    sockets.add(upstream);
    let opened = false;
    upstream.on('connect', () => {
      opened = true;
      client.pipe(upstream);
      upstream.pipe(client);
    });
    upstream.on('error', () => {
      if (!opened) say(`channel ${channel}: open failed: connect failed: Connection refused`);
      client.destroy();
    });
    client.on('error', () => upstream.destroy());
    for (const s of [client, upstream]) s.on('close', () => sockets.delete(s));
    client.on('close', () => upstream.destroy());
    upstream.on('close', () => client.destroy());
  });
  server.on('error', (e) => {
    if (e.code === 'EADDRINUSE')
      fail([
        `bind [127.0.0.1]:${local}: Address already in use`,
        `channel_setup_fwd_listener_tcpip: cannot listen to port: ${local}`,
        'Could not request local forwarding.',
      ]);
    fail([`fake-ssh: ${e.message}`]);
  });
  server.listen(local, '127.0.0.1', () => {
    say(`debug1: Authenticated to ${host} ([127.0.0.1]:22) using "publickey".`);
    say(`debug1: Local connections to 127.0.0.1:${local} forwarded to remote address 127.0.0.1:${m[2]}`);
    say(`debug1: Local forwarding listening on 127.0.0.1 port ${local}.`);
  });
  const end = () => {
    for (const s of sockets) s.destroy();
    server.close();
    say(`Connection to ${host} closed by remote host.`);
    process.exit(255);
  };
  if (dir) {
    setInterval(() => {
      const cut = [path.join(dir, 'cut'), path.join(dir, `cut-${host}`)].find((f) => fs.existsSync(f));
      if (cut) {
        try {
          fs.unlinkSync(cut);
        } catch {
          // another forward took it
        }
        end();
      }
    }, 100);
  }
  process.on('SIGTERM', () => process.exit(0));
}
