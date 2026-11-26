// An agent's page in its shelter, in the real app (ESC-667, docs/AGENT-PAGES.md § 8.4): the trap of
// `src-tauri/src/agentpage_trap.html`, the same as `agentpage_probe`'s, opened by a debug build's command (the MCP tool
// comes after, AP-2). Whatever it tries, nothing reaches the network (two listeners of the loopback say so, not the page),
// no command of Escouade nor of a plugin answers it, code made from text is refused, no permission and no download is
// given, another page is not read, and its navigation without a gesture is only kept for the window to offer.
import dgram from 'node:dgram';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, test } from './fixture';

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;

/** A command of the app, called from the window (the agent's page cannot: that is the point). */
async function call<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

interface View {
  page: string;
  url: string;
  loaded: boolean;
  title?: string;
  wants?: string;
  downloadsRefused: number;
  permissionsRefused: number;
  navigationsRefused: number;
}

/** Listens on the loopback, by TCP and by UDP (WebRTC's STUN): every request and datagram that reaches them (`hits`), and
 * the connections that sent nothing (`bare`). */
async function listeners() {
  const hits: string[] = [];
  const opened: net.Socket[] = [];
  const spoke = new Set<net.Socket>();
  const tcp = net.createServer((socket) => {
    opened.push(socket);
    socket.once('data', (d) => {
      spoke.add(socket);
      hits.push('tcp ' + d.toString('latin1').split('\r\n')[0]);
    });
    socket.on('error', () => {});
    setTimeout(() => {
      if (!socket.destroyed) socket.destroy();
    }, 1500);
  });
  await new Promise<void>((ok) => tcp.listen(0, '127.0.0.1', ok));
  const udp = dgram.createSocket('udp4');
  udp.on('message', (m) => hits.push('udp ' + m.length));
  await new Promise<void>((ok) => udp.bind(0, '127.0.0.1', ok));
  const port = (tcp.address() as net.AddressInfo).port;
  return {
    hits,
    bare: () => opened.filter((s) => !spoke.has(s)).length,
    tcp: port,
    udp: udp.address().port,
    close: () => {
      tcp.close();
      udp.close();
    },
  };
}

test("an agent's page in its shelter reaches nothing and nothing of Escouade answers it", async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2; WKWebView and WebKitGTK are tried by agentpage_probe in the CI');
  const { page } = app;
  const out = await listeners();
  try {
    const opened = await call<View>(page, 'agentpage_open_trap', {
      tcp: out.tcp,
      udp: out.udp,
      bounds: { x: 120, y: 120, width: 800, height: 500 },
    });
    expect(opened.page).toBe('p-1');
    // What the trap got, in its title: the one thing it may give.
    let view: View | null = null;
    await expect
      .poll(
        async () => {
          view = await call<View | null>(page, 'agentpage_view', { page: 'p-1' });
          return view?.title?.startsWith('escouade-probe:') ?? false;
        },
        { timeout: 60_000, intervals: [500] },
      )
      .toBe(true);
    // Its navigation without a gesture, and what would come late (a prefetch, STUN).
    await page.waitForTimeout(4000);
    view = await call<View | null>(page, 'agentpage_view', { page: 'p-1' });
    const r = JSON.parse(view!.title!.slice('escouade-probe:'.length)) as Record<string, unknown>;
    const said = (k: string) => String(r[k]);

    // No request and no datagram left the page. The end-to-end tests force WebView2's folder (`WEBVIEW2_USER_DATA_FOLDER`),
    // which leaves the pages no browser of their own, and so no dead end: their filter alone answers every request, but
    // WebView2 may still connect, without a word, to the address of the navigation it refused (a preconnection). The app
    // as shipped has the dead end too: `agentpage_probe` sees no connection at all.
    expect(out.hits, 'nothing reached the network').toEqual([]);
    test.info().annotations.push({ type: 'bare connections', description: String(out.bare()) });
    expect(fs.readFileSync(path.join(app.data, 'app.log'), 'utf8')).toContain('WEBVIEW2_USER_DATA_FOLDER is set, no dead end');
    expect(said('own'), 'its own file is served').toBe('reached: own file');
    expect(said('origin')).toBe('http://escouade-page.p-1.localhost');
    for (const k of ['app', 'plugin', 'touch', 'list']) expect(said(k), k).toMatch(/^refused/);
    for (const k of [
      'fetch',
      'img',
      'ws',
      'open',
      'rtcCall',
      'eval',
      'evalIndirect',
      'evalComplex',
      'timer',
      'fn',
      'camera',
      'microphone',
      'clipboard',
      'notification',
      'geolocation',
    ])
      expect(said(k), k).toMatch(/^refused/);
    expect(said('rtc')).toBe('undefined/undefined');
    expect(said('rtcInFrame')).toBe('undefined/undefined');
    expect(said('otherPage')).not.toContain('secret');
    expect(said('otherPath')).not.toMatch(/^reached: 200/);
    expect(said('choose'), 'no channel is open yet').toBe('reached: 404');
    expect(said('storageBefore')).toBe('null');
    // The CSP is enforced by its header and its `<meta>` both: each violation is told twice.
    expect(said('header')).toContain("default-src 'none'");
    expect(said('header')).not.toContain('unsafe-eval');
    expect(r.metas).toBe(2);
    const violations = r.violations as string[];
    expect(violations.filter((v) => v.startsWith('connect-src http://127.0.0.1')).length).toBeGreaterThanOrEqual(2);

    // Refused by the shelter, each one: downloads, permissions, its navigation (kept for the window to offer, never followed).
    expect(view!.downloadsRefused).toBeGreaterThanOrEqual(1);
    expect(view!.permissionsRefused).toBeGreaterThanOrEqual(1);
    expect(view!.wants).toBe(`http://127.0.0.1:${out.tcp}/navigation?secret=1`);
    expect(view!.url).toBe('http://escouade-page.p-1.localhost/');
    // The window's gesture takes the address the core kept, once (here let go: no browser opens).
    expect(await call<string | null>(page, 'agentpage_outside', { page: 'p-1', follow: false })).toBe(view!.wants);
    expect(await call<string | null>(page, 'agentpage_outside', { page: 'p-1', follow: false })).toBeNull();

    await call(page, 'agentpage_close', { page: 'p-1' });
    expect(await call<View | null>(page, 'agentpage_view', { page: 'p-1' })).toBeNull();
    expect(out.hits).toEqual([]);
  } finally {
    out.close();
  }
});
