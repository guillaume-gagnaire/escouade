// The phone's e2e (MO-1): an `escouade-server` (its debug build) started on a data folder of the test's, with the fake
// `claude`, a device paired as the phone will be (`escouade-server pair`, then its code); and a local forward to its API
// that stands for the phone's tunnel (`127.0.0.1:<port>`, octet for octet) while the WebView is a browser's page. It
// takes the page's Origin off (`http://127.0.0.1:<port>`): on the loopback the core refuses every page's Origin but
// its own window's, the phone's WebView's included (`remoteapi::proto::PHONE_ORIGINS`, taken through the tunnel alone).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { startBareServer, type BareServer } from '../../../tests/e2e/server';
import { apiClient, pairPhone, phoneForward, prepareServer, ROOT, stoppedAgent as stopAgent } from '../scripts/local-space.mjs';

export { ROOT };
const EXE = process.platform === 'win32' ? '.exe' : '';
/** The core without a window: built before the e2e (`cargo build --bin escouade-server`). */
export const SERVER = process.env.ESCOUADE_E2E_SERVER ?? path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-server${EXE}`);

/** The ports of this worktree (ESCOUADE_PORT_BASE): the page on base + 5. */
export const BASE = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
export const PAGE_PORT = BASE ? BASE + 5 : 4174;

/** A space of the test: its server, the address the phone speaks to and its token. */
export interface TestSpace {
  server: BareServer;
  /** The phone's address of its API: through the forward that stands for the tunnel. */
  url: string;
  token: string;
  /** A folder of the test's: its repository, its fake `claude`'s files. */
  root: string;
  /** The forward's address, as the phone types it with a code (`127.0.0.1:<port>`, MO-4). */
  address: string;
  /** A new pairing code of the server (`escouade-server pair`), as the user reads it on the server. */
  pairCode(): string;
  /** A command of the core, as a client of the API of its own calls it. */
  call<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  /** Cut: the forward closed (the server goes on), as a tunnel that falls. */
  cut(): void;
  stop(): Promise<void>;
}

/**
 * The test's folder removed: under Windows, a process just stopped (the server, its fake `claude`) holds its files a
 * moment longer. Tried for 15 s; left for the system's cleaning after that, never a failure of the test.
 */
async function removeLater(dir: string): Promise<void> {
  for (const deadline = Date.now() + 15_000; ;) {
    try {
      fs.rmSync(dir, { recursive: true, force: true });
      return;
    } catch (e) {
      if (Date.now() > deadline) {
        console.warn(`left behind: ${dir} (${String(e)})`);
        return;
      }
      await new Promise((r) => setTimeout(r, 500));
    }
  }
}

/** A space of the test: its server started, a phone paired, its forward. */
export async function startSpace(): Promise<TestSpace> {
  if (!fs.existsSync(SERVER)) throw new Error(`build the server first: cargo build --bin escouade-server (missing ${SERVER})`);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-mobile-e2e-'));
  const { data, env } = prepareServer(root);
  const server = await startBareServer(SERVER, data, env, 0);
  let forward: Awaited<ReturnType<typeof phoneForward>> | null = null;
  let api: Awaited<ReturnType<typeof apiClient>> | null = null;
  try {
    forward = await phoneForward(0, server.port);
    const token = await pairPhone(SERVER, data, env, server.port);
    api = await apiClient(`ws://127.0.0.1:${server.port}/remote`, token);
    const fwd = forward;
    const own = api;
    return {
      server,
      url: `ws://127.0.0.1:${fwd.port}/remote`,
      token,
      address: `127.0.0.1:${fwd.port}`,
      pairCode: () => {
        const printed = execFileSync(SERVER, ['pair', '--data-dir', data], { env, encoding: 'utf8', timeout: 60_000 });
        const code = /([A-Z2-9]{4}-[A-Z2-9]{4})/.exec(printed)?.[1];
        if (!code)
          throw new Error(`escouade-server pair printed no code:
${printed}`);
        return code;
      },
      root,
      call: own.call,
      cut: () => fwd.close(),
      stop: async () => {
        own.close();
        fwd.close();
        await server.stop();
        await removeLater(root);
      },
    };
  } catch (e) {
    api?.close();
    forward?.close();
    await server.stop();
    throw e;
  }
}

/** An agent of `space` stopped on a permission. */
export const stoppedAgent = (space: TestSpace): Promise<void> => stopAgent(space.call, space.root);
