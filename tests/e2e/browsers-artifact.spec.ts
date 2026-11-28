// The artifact of a Claude Code agent beside its conversation, reloaded at each version it publishes (ESC-681, AB-10;
// docs/AGENT-BROWSERS.md § 2 bis): the fake agent publishes version 1 of an artifact (its `Artifact` tool, in its journal),
// opens it by `artifact: "last"` (an address it did not publish is refused); version 2 reloads the tab by itself, the band
// saying « Recharge : version 2 »; while the user has the hand, version 3 leaves the tab as they have it, and once they give
// it back version 4 reloads it. Outside the launch (the C4 tiers), reading and capturing it wait for the user's leave.
// claude.ai is never reached: a debug build takes the test's local server as the artifacts' origin
// (ESCOUADE_E2E_ARTIFACT_ORIGIN), at a port of the worktree's range when it has one. At 1440 × 900 and at 1024 × 728.
// On Windows.
import fs from 'node:fs';
import http from 'node:http';
import type { Page } from '@playwright/test';
import { addProject, expect, test, unfoldSections, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

const BASE = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
const PORT = BASE > 0 ? BASE + 7 : 47_000 + (process.pid % 1000);
const ORIGIN = `http://127.0.0.1:${PORT}`;
const ARTIFACT = `${ORIGIN}/code/artifact/plan-24`;

/** The artifact's server: the version last published, and the one each request of the tab got. */
function serveArtifact() {
  const state = { version: 1, served: [] as number[], cookies: [] as (string | undefined)[] };
  const server = http.createServer((req, res) => {
    if (req.url !== '/code/artifact/plan-24') {
      res.writeHead(404).end();
      return;
    }
    state.served.push(state.version);
    state.cookies.push(req.headers.cookie);
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' });
    res.end(`<!doctype html><html><head><title>Plan de la 2.4</title></head>
<body style="font:16px system-ui;margin:24px"><h1 id="v">Version ${state.version}</h1></body></html>`);
  });
  return new Promise<{ server: http.Server; state: typeof state }>((ok, ko) => {
    server.once('error', ko);
    server.listen(PORT, '127.0.0.1', () => ok({ server, state }));
  });
}

const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

/** The server as the agent of the launch `argv` reaches it: with its own token. */
function agentServer(argv: string[]) {
  const entry = JSON.parse(fs.readFileSync(argv[argv.indexOf('--mcp-config') + 1], 'utf8')).mcpServers.escouade;
  return { url: entry.url as string, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
}

/** A fake agent: `initialize`, then its tools, as Claude Code calls them. */
async function fakeAgent(server: { url: string; token: string }) {
  let id = 0;
  const rpc = async (method: string, params: object, notify = false) => {
    // A reset socket (closed by the 2.3's server after 2 s idle, SEC-17, while a synchronous step held the event loop)
    // was never read: the call goes again, once, on a new one (ESC-701).
    const post = () =>
      fetch(server.url, {
        method: 'POST',
        headers: {
          authorization: `Bearer ${server.token}`,
          'content-type': 'application/json',
          accept: 'application/json, text/event-stream',
        },
        body: JSON.stringify({ jsonrpc: '2.0', ...(notify ? {} : { id: ++id }), method, params }),
      });
    const res = await post().catch((e: { cause?: { code?: string } }) =>
      ['ECONNRESET', 'UND_ERR_SOCKET', 'EPIPE'].includes(e?.cause?.code ?? '') ? post() : Promise.reject(e),
    );
    const text = await res.text();
    return text ? JSON.parse(text) : null;
  };
  await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc('notifications/initialized', {}, true);
  const call = async (name: string, args: object) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    return { text: (r.content as { text?: string }[]).map((c) => c.text ?? '').join('\n'), error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, r.text).toBe(false);
    return JSON.parse(r.text);
  };
  return { call, json };
}

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;
async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}
type Drive = { hand: string; agent: string | null; step: number; what: string };
const state = (page: Page, key: string) => invoke<Drive>(page, 'browser_drive_state', { key });

/** A message to the agent, in its composer (the tab's address bar is a textbox too). */
async function send(page: Page, text: string) {
  const box = page.locator('main.conv').first().getByRole('textbox').last();
  await box.fill(text);
  await box.press('Enter');
}

const section = (page: Page) => page.getByRole('region', { name: 'Navigateurs' });
const row = (page: Page, key: string) => section(page).locator(`.row-item.browser[data-browser="${key}"]`);

test.describe('the artifact of an agent, reloaded at each version', () => {
  test.use({
    preStart: { run: () => ({ mcpEnabled: true }) },
    appEnv: { ESCOUADE_E2E_ARTIFACT_ORIGIN: ORIGIN },
  });

  for (const size of SIZES) {
    test(`it opens its last artifact beside its conversation, each version reloads it, never while the user has the hand (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the browsers are exercised on WebView2 here');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      const art = await serveArtifact();
      try {
        await addProject(page, app.repo);
        await unfoldSections(page);
        await expect.poll(() => streams(app).length, { timeout: 30_000 }).toBeGreaterThanOrEqual(1);
        const agent = await fakeAgent(agentServer(streams(app)[0].argv));
        await page.locator('aside.side .agent').first().click();
        await expect(page.locator('main.conv').first()).toBeVisible();
        await page.waitForTimeout(500);

        // Nothing published yet: nothing to open.
        const none = await agent.call('open_browser', { artifact: 'last' });
        expect(none.error).toBe(true);
        expect(none.text).toContain('Aucun artefact publié');

        // Version 1, published by its Artifact tool: in its journal.
        await send(page, 'Publie le plan [artefact v1]');
        await expect(page.getByText('Version 1 publiée.')).toBeVisible({ timeout: 30_000 });
        // An address it did not publish is not its artifact, even of the same origin.
        const foreign = await agent.call('open_browser', { artifact: `${ORIGIN}/code/artifact/someone-elses` });
        expect(foreign.error).toBe(true);
        expect(foreign.text).toContain('pas un artefact que tu as publié');

        // Opened without asking, beside its conversation.
        const opened = await agent.json('open_browser', { artifact: 'last' });
        expect(opened).toMatchObject({ place: 'beside', url: ARTIFACT });
        const key = opened.browser as string;
        await expect(row(page, key)).toBeVisible({ timeout: 30_000 });
        await expect.poll(() => art.state.served, { timeout: 30_000 }).toEqual([1]);
        await expect(page.getByTestId('open-ask-band')).toBeHidden();
        // Its project's storage, never the window's: no cookie sent.
        expect(art.state.cookies[0]).toBeUndefined();

        await expect
          .poll(
            async () => {
              const b = (await agent.json('list_browsers', {})).browsers.find((r: { browser: string }) => r.browser === key);
              return `${b?.placement} ${b?.loaded}`;
            },
            { timeout: 30_000 },
          )
          .toBe('beside true');
        // Outside the launch (C4): to show goes without asking; reading and capturing wait for the user's leave.
        for (const tool of ['read_browser', 'capture_browser']) {
          const r = await agent.json(tool, { browser: key });
          expect(r, tool).toMatchObject({ asked: true, origin: ORIGIN });
          expect(JSON.stringify(r), tool).not.toContain('Version 1');
        }

        // Version 2: the tab reloads by itself, the band says which.
        art.state.version = 2;
        await send(page, 'Le tableau des risques en haut [artefact v2]');
        await expect.poll(() => art.state.served, { timeout: 30_000 }).toEqual([1, 2]);
        await expect.poll(async () => (await state(page, key)).what).toBe('Recharge : version 2');
        expect(await state(page, key)).toMatchObject({ hand: 'agent', step: 1 });

        // The user takes the hand: version 3 leaves the tab as they have it.
        await invoke(page, 'browser_drive_hand', { key, hand: 'take' });
        art.state.version = 3;
        await send(page, 'Encore une retouche [artefact v3]');
        await expect(page.getByText('Version 3 publiée.')).toBeVisible({ timeout: 30_000 });
        await page.waitForTimeout(2_000);
        expect(art.state.served).toEqual([1, 2]);
        // Given back: version 4 reloads it.
        await invoke(page, 'browser_drive_hand', { key, hand: 'give' });
        art.state.version = 4;
        await send(page, 'La dernière [artefact v4]');
        await expect.poll(() => art.state.served, { timeout: 30_000 }).toEqual([1, 2, 4]);
        await expect.poll(async () => (await state(page, key)).what).toBe('Recharge : version 4');
        await page.screenshot({ path: test.info().outputPath(`artifact-${size.width}.png`) });
      } finally {
        art.server.close();
      }
    });
  }
});
