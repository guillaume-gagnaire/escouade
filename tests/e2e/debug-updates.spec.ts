// A test's app never goes for a real version of Escouade (ESC-543): started then quit cleanly (« Quitter », which installs the
// update ready), it reads no real latest.json, downloads no real installer and runs none. A real installer would install in
// the folder of the app of the machine's user and close it first. A proxy of the test (HTTP_PROXY, HTTPS_PROXY, which the
// updater follows when the settings name none) sees where the app goes and lets nothing through; the app's temporary
// folder (TEMP, where the updater would write an installer to run it) is one of the test's. ESCOUADE_PORT_BASE gives the
// ports when set (+2 for the proxy, +3 for the update server), else the system picks them.
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import type { AddressInfo } from 'node:net';
import { expect, test, type App } from './fixture';
import { onThisMachine } from './update-guard';

/** What the proxy was asked for (`host:port` of a tunnel, an address of a plain request), and the app's temporary folder. */
const seen = { proxied: [] as string[], temp: '', served: [] as string[] };

const BASE = Number(process.env.ESCOUADE_PORT_BASE ?? 0);

async function listen(server: http.Server, offset: number): Promise<string> {
  await new Promise<void>((resolve) => server.listen(BASE ? BASE + offset : 0, '127.0.0.1', resolve));
  return `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
}

/** The proxy of the test: notes what is asked of it and refuses it all, so that nothing goes through it to the Internet. */
async function witness(): Promise<{ address: string; close: () => Promise<void> }> {
  const proxy = http.createServer((req, res) => {
    seen.proxied.push(req.url ?? '');
    res.writeHead(502).end();
  });
  proxy.on('connect', (req, socket) => {
    seen.proxied.push(req.url ?? '');
    socket.end('HTTP/1.1 403 Forbidden\r\n\r\n');
  });
  const address = await listen(proxy, 2);
  return { address, close: () => new Promise((resolve) => proxy.close(() => resolve())) };
}

/** The app's variables: the proxy, and a temporary folder of the test's. */
function witnessed(proxy: string): Record<string, string> {
  seen.temp = fs.mkdtempSync(path.join(os.tmpdir(), 'debug-updates-temp-'));
  return { HTTP_PROXY: proxy, HTTPS_PROXY: proxy, ALL_PROXY: proxy, NO_PROXY: '', TEMP: seen.temp, TMP: seen.temp };
}

function appLog(app: App): string {
  const file = path.join(app.data, 'app.log');
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '';
}

/** What the proxy was asked of that is not on this machine: the Internet. */
function abroad(): string[] {
  return seen.proxied.filter((asked) => !onThisMachine(/^https?:\/\//.test(asked) ? asked : `http://${asked}`));
}

/** « Quitter » (the tray's, the menu's): with an update ready, it installs silently and the app stays closed. */
async function quit(app: App) {
  await app.page
    .evaluate(() => {
      const t = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string): Promise<unknown> } }).__TAURI_INTERNALS__;
      return t.invoke('quit_app');
    })
    // The window goes with the app: the call may not come back.
    .catch(() => {});
  await expect.poll(() => app.stopped(), { timeout: 30_000 }).toBe('exit code 0 (0x0)');
}

/** What the updater would have written to run an installer (`<app>-<version>-updater-…`). */
function installers(): string[] {
  return fs.readdirSync(seen.temp).filter((name) => /updater/i.test(name));
}

test.describe('with no test server', () => {
  test.use({
    appEnv: async ({}, use) => {
      seen.proxied = [];
      const proxy = await witness();
      await use(witnessed(proxy.address));
      await proxy.close();
    },
  });

  test('an app that starts then quits cleanly reads no real latest.json and runs no installer', async ({ app }) => {
    // The window's first check, 8 s after the start: off in a debug build.
    await expect.poll(() => appLog(app), { timeout: 45_000 }).toContain('update check off in a debug build');
    await quit(app);
    const log = appLog(app);
    expect(log).not.toContain('update check reads');
    expect(log).not.toContain('installing Escouade');
    expect(abroad()).toEqual([]);
    expect(installers()).toEqual([]);
  });
});

test.describe('with a test server that offers a real installer', () => {
  test.use({
    appEnv: async ({}, use) => {
      seen.proxied = [];
      seen.served = [];
      const proxy = await witness();
      // A version above every one, whose installer is a real version's address (GitHub's).
      const server = http.createServer((req, res) => {
        seen.served.push(req.url ?? '');
        if ((req.url ?? '').split('?')[0] !== '/latest.json') {
          res.writeHead(404).end();
          return;
        }
        const entry = { signature: 'test', url: 'https://github.com/guillaume-gagnaire/escouade/releases/download/v99.0.0/setup.exe' };
        const targets = ['windows-x86_64', 'windows-x86_64-nsis', 'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64'];
        const manifest = { version: '99.0.0', notes: '- Une vraie version', platforms: Object.fromEntries(targets.map((t) => [t, entry])) };
        res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(manifest));
      });
      const origin = await listen(server, 3);
      await use({ ...witnessed(proxy.address), ESCOUADE_UPDATE_ENDPOINT: `${origin}/latest.json`, NO_PROXY: '127.0.0.1' });
      await new Promise((resolve) => server.close(resolve));
      await proxy.close();
    },
  });

  test('the version found is never downloaded, and quitting installs nothing', async ({ app }) => {
    await expect.poll(() => appLog(app), { timeout: 45_000 }).toContain('setup.exe not downloaded');
    expect(seen.served).toContain('/latest.json');
    await quit(app);
    expect(appLog(app)).not.toContain('installing Escouade');
    expect(abroad()).toEqual([]);
    expect(installers()).toEqual([]);
  });
});
