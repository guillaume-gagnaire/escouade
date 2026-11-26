// The agents read the browsers' console (ESC-671, AB-4; docs/AGENT-BROWSERS.md § 3 bis): a fake agent, with its own
// token of the MCP server, reads the console and the failed requests of a browser on the project's launch, without
// asking: a console.error and a 404 the page told while it loaded, their addresses without their query string, `from`
// the browser, filtered by level, bounded by `limit`; then, from the cursor its last read gave, only what came after.
// Once on no screen, its console is still read. On another origin the console waits for the user's leave in the
// browser's band. At 1440 × 900 and at 1024 × 728. On Windows: the console comes from WebView2's protocol.
import fs from 'node:fs';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, test, unfoldSections, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

/** The developed app's page: it logs, fails a request with a token in its query, then waits for `/later`. */
const LAUNCH_PAGE = `<!doctype html><html><head><title>Console — demo</title></head>
<body style="font:16px system-ui;margin:24px"><h1>Console</h1>
<script>
console.log('demo prête');
console.info('info du démarrage');
console.error('Échec du chargement des prix');
fetch('/api/prix?token=tok-query-123').catch(() => {});
fetch('/later').then(() => { console.warn('plus tard'); return fetch('/api/apres?session=tok-after-456'); }).catch(() => {});
</script></body></html>`;

/** A third party's site, where the user would be signed in. */
const OTHER_PAGE = `<!doctype html><title>Mon compte</title><body><h1>Compte</h1><script>console.error('erreur du tiers');</script></body>`;

/** A server of `html` at `/`, 404 elsewhere; `/later` answers once `release` is called. */
function serve(html: string, host: string) {
  let release = () => {};
  const held = new Promise<void>((ok) => (release = ok));
  const server = http.createServer((req, res) => {
    if (req.url === '/' || req.url?.startsWith('/console')) {
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      res.end(html);
    } else if (req.url === '/later') {
      void held.then(() => {
        res.writeHead(200, { 'Content-Type': 'text/plain' });
        res.end('ok');
      });
    } else {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end('absent');
    }
  });
  return new Promise<{ server: http.Server; release: () => void }>((ok) =>
    server.listen(0, host, () => ok({ server, release: () => release() })),
  );
}
const portOf = (s: http.Server) => (s.address() as AddressInfo).port;

const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

/** The server as the agent of the launch `argv` reaches it: with its own token. */
function agentServer(argv: string[]) {
  const entry = JSON.parse(fs.readFileSync(argv[argv.indexOf('--mcp-config') + 1], 'utf8')).mcpServers.escouade;
  return { url: entry.url as string, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
}

type Entry = { t: number; level: string; text: string; source: string; url?: string; line?: number; from: string };
type Net = { t: number; method: string; url: string; status?: number; error?: string; type: string; from: string };
type Logs = { entries: Entry[]; network: Net[]; cursor: string; dropped: number };

/** A fake agent: `initialize`, then its tools, as Claude Code calls them. */
async function fakeAgent(server: { url: string; token: string }) {
  let id = 0;
  const rpc = async (method: string, params: object, notify = false) => {
    const res = await fetch(server.url, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${server.token}`,
        'content-type': 'application/json',
        accept: 'application/json, text/event-stream',
      },
      body: JSON.stringify({ jsonrpc: '2.0', ...(notify ? {} : { id: ++id }), method, params }),
    });
    const text = await res.text();
    return text ? JSON.parse(text) : null;
  };
  await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc('notifications/initialized', {}, true);
  const call = async (name: string, args: object) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    return { texts: (r.content as { text: string }[]).map((c) => c.text), error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, r.texts.join('\n')).toBe(false);
    return JSON.parse(r.texts[0]);
  };
  /** The console read: its notice, then its JSON between the marks of third-party data. */
  const consoleOf = async (args: object): Promise<Logs> => {
    const r = await call('browser_console', args);
    expect(r.error, r.texts.join('\n')).toBe(false);
    expect(r.texts[0]).toContain('third_party_data');
    return JSON.parse(r.texts[1].replace(/^<third_party_data>\n/, '').replace(/\n<\/third_party_data>$/, ''));
  };
  return { call, json, consoleOf };
}

const section = (page: Page) => page.getByRole('region', { name: 'Navigateurs' });

/** « + » of « Navigateurs »: a browser opened alone at `address`, shown alone. */
async function openBrowser(page: Page, address: string, title: string) {
  await section(page).getByRole('button', { name: 'Nouveau navigateur' }).click();
  const field = section(page).getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
  await field.fill(address);
  await field.press('Enter');
  await expect(section(page).locator('.row-item.browser .nm', { hasText: title })).toBeVisible({ timeout: 30_000 });
}

test.describe('a fake agent reads the console of the browsers', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it reads the launch’s console from a cursor, its failed requests, and another origin’s with the user’s leave (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the console is exercised on WebView2 here (WebKit by the probe)');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      const launch = await serve(LAUNCH_PAGE, 'localhost');
      const other = await serve(OTHER_PAGE, '127.0.0.1');
      const lp = portOf(launch.server);
      const op = portOf(other.server);
      try {
        await addProject(page, app.repo);
        // The project's launch: a command whose address is the dev server's (declared, not run).
        await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
        const dialog = page.getByRole('dialog', { name: 'Réglages' });
        await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
        const cfg = dialog.getByRole('group', { name: 'Commande 1' });
        await cfg.getByLabel('Nom').fill('web');
        await cfg.getByLabel('Commande', { exact: true }).fill('node -e "1"');
        await cfg.getByRole('checkbox', { name: 'Ouvrir à son lancement' }).check();
        await cfg.getByLabel('Adresse à ouvrir').fill(`http://localhost:${lp}/`);
        await closeSettings(dialog);
        await unfoldSections(page);

        await expect.poll(() => streams(app).length, { timeout: 30_000 }).toBeGreaterThanOrEqual(1);
        const agent = await fakeAgent(agentServer(streams(app)[0].argv));

        await openBrowser(page, `localhost:${lp}/console`, 'Console — demo');
        const key = (await agent.json('list_browsers', {})).browsers[0].browser as string;

        // Without asking, on the launch: what the page told while it loaded, told by the browser.
        let logs: Logs = { entries: [], network: [], cursor: '', dropped: 0 };
        await expect
          .poll(
            async () => {
              logs = await agent.consoleOf({ browser: key });
              return (
                logs.entries.some((e) => e.text.includes('Échec du chargement')) && logs.network.some((n) => n.url.endsWith('/api/prix'))
              );
            },
            { timeout: 20_000 },
          )
          .toBe(true);
        const error = logs.entries.find((e) => e.text.includes('Échec du chargement'))!;
        expect(error).toMatchObject({ level: 'error', from: 'browser' });
        expect(error.url).toBe(`http://localhost:${lp}/console`);
        expect(error.line).toBeGreaterThan(0);
        expect(logs.entries.map((e) => e.text)).toEqual(expect.arrayContaining(['demo prête', 'info du démarrage']));
        const missing = logs.network.find((n) => n.url.endsWith('/api/prix'))!;
        // Its address without its query string, where the token was.
        expect(missing).toMatchObject({ method: 'GET', status: 404, url: `http://localhost:${lp}/api/prix`, from: 'browser' });
        expect(JSON.stringify(logs)).not.toContain('tok-query-123');
        expect(logs.dropped).toBe(0);
        expect(logs.entries.some((e) => e.text === 'plus tard')).toBe(false);

        // Filtered by level, bounded by limit.
        const errors = await agent.consoleOf({ browser: key, levels: ['error'], network: false });
        expect(errors.entries.length).toBeGreaterThanOrEqual(1);
        expect(errors.entries.every((e) => e.level === 'error')).toBe(true);
        expect(errors.network).toEqual([]);
        const one = await agent.consoleOf({ browser: key, limit: 1 });
        expect(one.entries).toHaveLength(1);
        expect(one.network).toHaveLength(1);

        // From the cursor: nothing until the page tells more, then only that.
        const cursor = logs.cursor;
        expect((await agent.consoleOf({ browser: key, since: cursor })).entries).toEqual([]);
        launch.release();
        let next: Logs = logs;
        await expect
          .poll(
            async () => {
              next = await agent.consoleOf({ browser: key, since: cursor });
              return next.entries.some((e) => e.text === 'plus tard') && next.network.some((n) => n.url.endsWith('/api/apres'));
            },
            { timeout: 20_000 },
          )
          .toBe(true);
        expect(next.entries.some((e) => e.text.includes('Échec du chargement'))).toBe(false);
        expect(next.entries.find((e) => e.text === 'plus tard')!.level).toBe('warn');
        expect(JSON.stringify(next)).not.toContain('tok-after-456');
        // From an instant later than everything: nothing.
        const after = Math.max(...next.entries.map((e) => e.t), ...next.network.map((n) => n.t)) + 1;
        expect((await agent.consoleOf({ browser: key, since: new Date(after).toISOString() })).entries).toEqual([]);

        // Another origin, shown alone: the first is on no screen, and its console is read still.
        await openBrowser(page, `127.0.0.1:${op}/`, 'Mon compte');
        const both = (await agent.json('list_browsers', {})).browsers;
        expect(both[0].placement).toBe('hidden');
        const otherKey = both[1].browser as string;
        expect((await agent.consoleOf({ browser: key, levels: ['warn'] })).entries[0].text).toBe('plus tard');

        // Outside the launch: asked in the band; nothing of the console reaches the agent before the leave.
        const asked = await agent.json('browser_console', { browser: otherKey });
        expect(asked).toMatchObject({ asked: true, origin: `http://127.0.0.1:${op}` });
        expect(JSON.stringify(asked)).not.toContain('erreur du tiers');
        const band = page.getByTestId('consent-band');
        await expect(band).toBeVisible();
        await expect(band).toContainText(`veut lire 127.0.0.1:${op}`);
        await expect(band.getByRole('button', { name: 'Permettre' })).toBeInViewport();
        await band.getByRole('button', { name: 'Permettre' }).click();
        await expect(band).toBeHidden();
        await expect
          .poll(async () => (await agent.consoleOf({ browser: otherKey })).entries.map((e) => e.text), { timeout: 20_000 })
          .toContain('erreur du tiers');
        // The agent was told by a message that it may read it again.
        await page.locator('aside.side .agent').first().click();
        await expect(page.getByText(/read_browser ou browser_console/).first()).toBeVisible({ timeout: 20_000 });
      } finally {
        launch.release();
        launch.server.close();
        other.server.close();
      }
    });
  }
});
