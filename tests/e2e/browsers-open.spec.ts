// An agent opens a real browser tab beside its conversation and closes it (ESC-678, AB-6; docs/AGENT-BROWSERS.md § 3.2,
// § 5.1): a fake agent, with its own token of the MCP server, opens its project's launch beside its conversation (the
// tab asks the dev server with no cookie: its project's storage, never the window's); while the user looks at the
// Kanban, another opening waits in « Navigateurs » and opens under the conversation once they come back, the first one
// leaving its pane (one pane per agent); an address outside the launch is asked in a band of the conversation, opened
// once allowed, and the agent told; this machine at a port no launch declares and the other schemes are refused; the
// agent closes the tab it opened, stops driving the launch's page (its row stays), and cannot close the user's. At
// 1440 × 900 and at 1024 × 728. On Windows.
import fs from 'node:fs';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, test, unfoldSections, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

const LAUNCH_PAGE = `<!doctype html><html><head><title>Tarifs — demo</title></head>
<body style="font:16px system-ui;margin:24px"><h1>Tarifs</h1><p id="pro">Offre Pro 12 €</p></body></html>`;
/** A site outside the launch: `.invalid` never resolves (RFC 2606), so nothing leaves the machine. */
const OTHER = 'http://docs.escouade-e2e.invalid';

/** A server of `html` on `host`, that keeps the Cookie header of each request it gets. */
function serve(html: string, host: string): Promise<{ server: http.Server; cookies: (string | undefined)[] }> {
  const cookies: (string | undefined)[] = [];
  const server = http.createServer((req, res) => {
    cookies.push(req.headers.cookie);
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Set-Cookie': 'seen=1; Path=/' });
    res.end(html);
  });
  return new Promise((ok) => server.listen(0, host, () => ok({ server, cookies })));
}
const portOf = (s: http.Server) => (s.address() as AddressInfo).port;

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
  const init = await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc('notifications/initialized', {}, true);
  const call = async (name: string, args: object) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    return { text: (r.content as { text: string }[]).map((c) => c.text).join('\n'), error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, r.text).toBe(false);
    return JSON.parse(r.text);
  };
  return { instructions: init.result.instructions as string, call, json };
}

const section = (page: Page) => page.getByRole('region', { name: 'Navigateurs' });
const row = (page: Page, key: string) => section(page).locator(`.row-item.browser[data-browser="${key}"]`);

test.describe('a fake agent opens and closes browsers', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it opens its launch beside its conversation, waits while the user looks away, asks another origin, closes (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the browsers are exercised on WebView2 here');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      const launch = await serve(LAUNCH_PAGE, 'localhost');
      const lp = portOf(launch.server);
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
        expect(agent.instructions).toContain('open_browser');
        await page.locator('aside.side .agent').first().click();
        const composer = page.locator('main.conv').first();
        await expect(composer).toBeVisible();
        // The window tells the backend whose conversation is on screen.
        await page.waitForTimeout(500);

        // Its launch, beside its conversation: both on screen.
        const first = await agent.json('open_browser', { launch: true });
        expect(first).toMatchObject({ place: 'beside', url: `http://localhost:${lp}/` });
        const launchKey = first.browser as string;
        await expect(row(page, launchKey)).toBeVisible({ timeout: 30_000 });
        await expect
          .poll(
            async () =>
              (await agent.json('list_browsers', {})).browsers.find((b: { browser: string }) => b.browser === launchKey)?.placement,
          )
          .toBe('beside');
        await expect(composer).toBeInViewport();
        await expect.poll(() => launch.cookies.length).toBeGreaterThanOrEqual(1);
        // The tab asked with no cookie: its project's storage, nothing of the window's.
        expect(launch.cookies[0]).toBeUndefined();

        // The user looks at the Kanban: the next opening waits in « Navigateurs ».
        await page.getByRole('button', { name: 'Kanban' }).first().click();
        await page.waitForTimeout(500);
        const second = await agent.json('open_browser', { url: `http://localhost:${lp}/tarifs`, place: 'below' });
        expect(second.place).toBe('waiting');
        expect(second.why).toContain('ne regarde pas ta conversation');
        const tabKey = second.browser as string;
        await expect(row(page, tabKey)).toHaveClass(/waits/);
        // Back on the conversation: it opens under it, and the first one leaves its pane (one pane per agent).
        await page.locator('aside.side .agent').first().click();
        await expect(row(page, tabKey)).not.toHaveClass(/waits/);
        await expect
          .poll(async () => {
            const rows = (await agent.json('list_browsers', {})).browsers as { browser: string; placement: string; by: string }[];
            return rows.map((b) => `${b.browser}=${b.placement}`).sort();
          })
          .toEqual([`${launchKey}=hidden`, `${tabKey}=below`].sort());
        expect(
          ((await agent.json('list_browsers', {})).browsers as { browser: string; by: string }[]).find((b) => b.browser === tabKey)?.by,
        ).toBe('agent');

        // Never this machine at a port no launch declares, nor another scheme: refused, nothing asked.
        for (const url of ['http://127.0.0.1:9/', 'http://localhost:1/', 'file:///C:/Windows/win.ini', 'javascript:alert(1)']) {
          const r = await agent.call('open_browser', { url });
          expect(r.error, url).toBe(true);
        }
        await expect(page.getByTestId('open-ask-band')).toBeHidden();

        // Another origin: asked in the conversation, nothing open before the answer.
        const asked = await agent.json('open_browser', { url: `${OTHER}/docs` });
        expect(asked).toMatchObject({ place: 'asked', origin: OTHER });
        const band = page.getByTestId('open-ask-band');
        await expect(band).toBeVisible();
        await expect(band).toContainText('veut ouvrir docs.escouade-e2e.invalid');
        await expect(band.getByRole('button', { name: 'Permettre une fois' })).toBeInViewport();
        await expect(band.getByRole('button', { name: 'Refuser' })).toBeInViewport();
        // Nothing is open before the answer.
        expect(((await agent.json('list_browsers', {})).browsers as unknown[]).length).toBe(2);
        await band.getByRole('button', { name: 'Permettre une fois' }).click();
        await expect(band).toBeHidden();
        await expect(page.getByText(/l’utilisateur te permet d’ouvrir/).first()).toBeVisible({ timeout: 20_000 });
        const rows = (await agent.json('list_browsers', {})).browsers as {
          browser: string;
          origin: string;
          by: string;
          placement: string;
        }[];
        const otherRow = rows.find((b) => b.origin === OTHER);
        expect(otherRow).toMatchObject({ by: 'agent' });
        const otherKey = otherRow!.browser;
        await expect(row(page, otherKey)).toBeVisible();
        // Asked again for that origin: its tab, no new request.
        expect((await agent.json('open_browser', { url: `${OTHER}/autre` })).browser).toBe(otherKey);
        await expect(band).toBeHidden();

        // It closes the tab it opened: its row goes. The launch's page: its driving stops, its row stays.
        expect(await agent.json('close_browser', { browser: otherKey })).toMatchObject({ closed: true });
        await expect(row(page, otherKey)).toHaveCount(0);
        expect(await agent.json('close_browser', { browser: launchKey })).toMatchObject({ closed: false });
        await expect(row(page, launchKey)).toBeVisible();

        // A browser the user opened alone is not the agent's to close.
        await section(page).getByRole('button', { name: 'Nouveau navigateur' }).click();
        const field = section(page).getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
        await field.fill(`localhost:${lp}/compte`);
        await field.press('Enter');
        const userRow = section(page).locator('.row-item.browser', { hasText: 'Tarifs — demo' }).last();
        await expect
          .poll(async () => ((await agent.json('list_browsers', {})).browsers as { by: string }[]).filter((b) => b.by === 'user').length)
          .toBe(1);
        const userKey = ((await agent.json('list_browsers', {})).browsers as { browser: string; by: string }[]).find(
          (b) => b.by === 'user',
        )!.browser;
        const refused = await agent.call('close_browser', { browser: userKey });
        expect(refused.error).toBe(true);
        expect(refused.text).toContain('L’utilisateur a ouvert ce navigateur');
        await expect(userRow).toBeVisible();
      } finally {
        launch.server.close();
      }
    });
  }
});
