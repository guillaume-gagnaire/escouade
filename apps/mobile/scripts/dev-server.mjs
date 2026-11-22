// `npm run dev:server` (MO-1): the phone's page in Vite, on a local `escouade-server` of its own (its debug build:
// `cargo build --bin escouade-server` first), a phone paired, through the forward that stands for the phone's tunnel.
// The pairing of MO-4 does not exist yet: the space is given to the page of development (`VITE_ESCOUADE_SPACES`).
//
//   ESCOUADE_PORT_BASE=4170 npm run dev:server -- [--demo]
//
// The page on base + 1, the server on base + 2, the forward on base + 3 (without a base: 5174, then free ports).
// `--demo`: a project and its agent stopped on a permission (the fake `claude`), for « À toi » and « Agents ». The
// server's data folder is a temporary one, removed at the end.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { apiClient, pairPhone, phoneForward, prepareServer, ROOT, stoppedAgent } from './local-space.mjs';

const args = process.argv.slice(2);
const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
const ports = { page: base ? base + 1 : 5174, server: base ? base + 2 : 0, forward: base ? base + 3 : 0 };
const exe = process.platform === 'win32' ? '.exe' : '';
const program = process.env.ESCOUADE_E2E_SERVER ?? path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-server${exe}`);
if (!fs.existsSync(program)) {
  console.error(`build the server first: cargo build --bin escouade-server (missing ${program})`);
  process.exit(1);
}

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-mobile-dev-'));
const { data, env } = prepareServer(root);
const children = [];
const stop = () => {
  for (const c of children) c.kill();
  setTimeout(() => {
    fs.rmSync(root, { recursive: true, force: true });
    process.exit(0);
  }, 1000);
};
process.on('SIGINT', stop);
process.on('SIGTERM', stop);

// The server, ready when its log says where its API is.
const server = spawn(program, ['run', '--data-dir', data, '--port', String(ports.server)], { env, stdio: ['ignore', 'pipe', 'pipe'] });
children.push(server);
const port = await new Promise((resolve, reject) => {
  let log = '';
  const read = (chunk) => {
    log += chunk;
    const ready = /ready: remote API on ws:\/\/127\.0\.0\.1:(\d+)\//.exec(log);
    if (ready) resolve(Number(ready[1]));
  };
  server.stdout.on('data', read);
  server.stderr.on('data', read);
  server.on('exit', (code) => reject(new Error(`escouade-server stopped as it started (exit code ${code}):\n${log}`)));
});
const token = await pairPhone(program, data, env, port);
const forward = await phoneForward(ports.forward, port);
if (args.includes('--demo')) {
  const api = await apiClient(`ws://127.0.0.1:${port}/remote`, token);
  await stoppedAgent(api.call, root);
  api.close();
}
const spaces = JSON.stringify([{ id: 'travail', name: 'Travail', url: `ws://127.0.0.1:${forward.port}/remote`, token }]);
console.log(`escouade-server on 127.0.0.1:${port}, the phone's forward on 127.0.0.1:${forward.port}`);

const vite = spawn(process.execPath, [path.join(ROOT, 'node_modules', 'vite', 'bin', 'vite.js')], {
  cwd: path.join(ROOT, 'apps', 'mobile'),
  env: { ...process.env, PORT: String(ports.page), VITE_ESCOUADE_SPACES: spaces },
  stdio: 'inherit',
});
children.push(vite);
vite.on('exit', stop);
