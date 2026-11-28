// `escouade-server` for the e2e (ESC-514, docs/REMOTE.md « Le mode serveur »): the core without a window started on a
// data folder of its own, a device paired by `escouade-server pair`, and a local forward to its API (what `ssh -L` is
// for a client of a VPS), which the desktop app's window then speaks to (`ESCOUADE_REMOTE_URL`).

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { endTree } from './proctree';

/**
 * Ends an `escouade-server` of the test and all it started (its agents' fake CLIs, their terminals, orphans included), by
 * their pids and never by a program's name, and waits until they are gone (ESC-522): what `taskkill /T` alone missed or
 * left dying could hold a port or a folder of the next test.
 */
async function endServer(child: ChildProcess, at: number, exitedAt: () => number | null) {
  const pid = child.pid;
  if (!pid) return;
  const { left, said } = await endTree(
    () => ({ pid, at, exitedAt: exitedAt() }),
    () => false,
    120_000,
  );
  if (left.length) {
    console.warn(
      `processes of escouade-server still there 120 s after it was ended: ${left.map((p) => p.pid).join(', ')}; taskkill: ${said}`,
    );
  }
}

export interface Server {
  /** Its data folder. */
  data: string;
  /** Its port, on 127.0.0.1. */
  port: number;
  /** The port of the local forward to it. */
  forward: number;
  /** The address of its API, through the forward. */
  url: string;
  /** The token of the device paired for the test. */
  token: string;
  /** What it wrote on its standard output, its log. */
  log: () => string;
  /** Stops it (and the forward). */
  stop: () => Promise<void>;
}

/** The program, beside the app's. */
export function serverProgram(appExe: string): string {
  return path.join(path.dirname(appExe), `escouade-server${path.extname(appExe)}`);
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** `escouade-server <args>` run to its end: its standard output. */
export function serverCommand(program: string, args: string[], env: NodeJS.ProcessEnv): string {
  return execFileSync(program, args, { env, encoding: 'utf8', timeout: 60_000 });
}

/** A local forward of `listen` (0: a free port) to 127.0.0.1:`to`, as `ssh -L` gives: its port and its end. */
export function forward(listen: number, to: number): Promise<{ port: number; close: () => void }> {
  const sockets = new Set<net.Socket>();
  const server = net.createServer((client) => {
    const upstream = net.connect({ host: '127.0.0.1', port: to });
    for (const s of [client, upstream]) {
      sockets.add(s);
      s.on('close', () => sockets.delete(s));
      s.on('error', () => {
        client.destroy();
        upstream.destroy();
      });
    }
    client.pipe(upstream);
    upstream.pipe(client);
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(listen, '127.0.0.1', () => {
      const port = (server.address() as net.AddressInfo).port;
      resolve({
        port,
        close: () => {
          for (const s of sockets) s.destroy();
          server.close();
        },
      });
    });
  });
}

/** The first message of the server to `message`, sent first on a new WebSocket to `url`. */
export function firstAnswer(url: string, message: unknown): Promise<Record<string, any>> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      ws.close();
      reject(new Error(`no answer from ${url} in 20 s`));
    }, 20_000);
    ws.onopen = () => ws.send(JSON.stringify(message));
    ws.onmessage = (e) => {
      clearTimeout(timer);
      ws.close();
      resolve(JSON.parse(String(e.data)));
    };
    ws.onerror = () => {
      clearTimeout(timer);
      reject(new Error(`the WebSocket to ${url} failed`));
    };
  });
}

/** The log `out` from its byte `from` on: what the process started then wrote. */
function ownLog(out: string, from: number): () => string {
  return () => (fs.existsSync(out) ? fs.readFileSync(out).subarray(from).toString('utf8') : '');
}

/** An `escouade-server` started, not paired yet (ESC-537: the client pairs it itself, through a fake `ssh`). */
export interface BareServer {
  data: string;
  port: number;
  log: () => string;
  stop: () => Promise<void>;
}

/** `escouade-server` started on `data` (its settings.json written already) with `env`, on `port` (0: a free one), ready. */
export async function startBareServer(program: string, data: string, env: NodeJS.ProcessEnv, port: number): Promise<BareServer> {
  if (!fs.existsSync(program)) throw new Error(`build the server first: cargo build --bin escouade-server (missing ${program})`);
  // Its own log (ESC-539: two servers of one test each read their own « ready » line).
  const out = path.join(data, '..', `${path.basename(data)}-stdout.log`);
  // Its own part of the file: a server started again on the same data appends to it, and the « ready » line of the one
  // before is not this one's (ESC-703: read as ready before it was, or when it did not start at all).
  const from = fs.existsSync(out) ? fs.statSync(out).size : 0;
  const fd = fs.openSync(out, 'a');
  const at = Date.now();
  const child: ChildProcess = spawn(program, ['run', '--data-dir', data, '--port', String(port)], { env, stdio: ['ignore', fd, fd] });
  fs.closeSync(fd);
  let exited: string | null = null;
  let exitedAt: number | null = null;
  child.on('exit', (code) => {
    exited = `exit code ${code}`;
    exitedAt = Date.now();
  });
  const log = ownLog(out, from);
  const stop = async () => {
    await endServer(child, at, () => exitedAt);
  };
  for (const deadline = Date.now() + 90_000; Date.now() < deadline;) {
    const ready = /ready: remote API on ws:\/\/127\.0\.0\.1:(\d+)\//.exec(log());
    if (ready) return { data, port: Number(ready[1]), log, stop };
    if (exited) throw new Error(`escouade-server stopped as it started (${exited}):\n${log()}`);
    await wait(200);
  }
  await stop();
  throw new Error(`escouade-server not ready in 90 s:\n${log()}`);
}

/**
 * `escouade-server` started on `data` (its settings.json written already) with `env`, ready; a device paired through a
 * local forward to it. `ports`: its own and the forward's (0: free ones).
 */
export async function startServer(
  program: string,
  data: string,
  env: NodeJS.ProcessEnv,
  ports: { server: number; forward: number },
): Promise<Server> {
  if (!fs.existsSync(program)) throw new Error(`build the server first: cargo build --bin escouade-server (missing ${program})`);
  const out = path.join(data, '..', 'server-stdout.log');
  // Its own part of the file: a server started again on the same data appends to it, and the « ready » line of the one
  // before is not this one's (ESC-703: read as ready before it was, or when it did not start at all).
  const from = fs.existsSync(out) ? fs.statSync(out).size : 0;
  const fd = fs.openSync(out, 'a');
  const at = Date.now();
  const child: ChildProcess = spawn(program, ['run', '--data-dir', data, '--port', String(ports.server)], {
    env,
    stdio: ['ignore', fd, fd],
  });
  fs.closeSync(fd);
  let exited: string | null = null;
  let exitedAt: number | null = null;
  child.on('exit', (code) => {
    exited = `exit code ${code}`;
    exitedAt = Date.now();
  });
  const log = ownLog(out, from);
  const kill = async () => {
    await endServer(child, at, () => exitedAt);
  };
  let tunnel: { port: number; close: () => void } | null = null;
  try {
    // Ready when its log says where its API is.
    let port = 0;
    for (const deadline = Date.now() + 90_000; Date.now() < deadline;) {
      const ready = /ready: remote API on ws:\/\/127\.0\.0\.1:(\d+)\//.exec(log());
      if (ready) {
        port = Number(ready[1]);
        break;
      }
      if (exited) throw new Error(`escouade-server stopped as it started (${exited}):\n${log()}`);
      await wait(200);
    }
    if (!port) throw new Error(`escouade-server not ready in 90 s:\n${log()}`);
    tunnel = await forward(ports.forward, port);
    const url = `ws://127.0.0.1:${tunnel.port}/remote`;
    // A device paired as the user does: the code `pair` prints, given through the forward.
    const printed = serverCommand(program, ['pair', '--data-dir', data], env);
    const code = /([A-Z2-9]{4}-[A-Z2-9]{4})/.exec(printed)?.[1];
    if (!code) throw new Error(`escouade-server pair printed no code:\n${printed}`);
    const paired = await firstAnswer(url, { t: 'pair', code, name: 'Poste de test' });
    if (paired.t !== 'paired' || typeof paired.token !== 'string') {
      throw new Error(`the pairing was refused: ${JSON.stringify(paired)}`);
    }
    const opened = tunnel;
    return {
      data,
      port,
      forward: opened.port,
      url,
      token: paired.token,
      log,
      stop: async () => {
        opened.close();
        await kill();
      },
    };
  } catch (e) {
    tunnel?.close();
    await kill();
    throw e;
  }
}
