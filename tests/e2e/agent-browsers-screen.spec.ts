// The screen (ESC-684, AB-9; docs/AGENT-BROWSERS.md § 2, écran 2, and § 5.5): a fake agent, with its own token of the MCP
// server, opens its project's launch while the user looks at the Kanban: nothing comes on their screen, the tool answers
// `waiting`, the row waits in « Navigateurs », and it opens beside the conversation when they come back, the
// conversation staying the active pane. Another tab it opens takes the place of the first (one pane per agent). The
// user closes its pane: the agent's next opening answers `closed`, nothing shows nor waits, until the user has left the
// conversation and come back. « Laisser piloter par… » lends a browser the user opened alone to the agent, told by a
// message, which may then drive it. At 1440 × 900 and at 1024 × 728 (and at the CI's size with ESCOUADE_E2E_VIEWPORT).
// On Windows.
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
<body style="font:16px system-ui;margin:24px"><h1>Tarifs</h1><div style="height:3000px"></div><p id="pro">Offre Pro 12 €</p></body></html>`;

/** A server of `html` on localhost. */
function serve(html: string): Promise<http.Server> {
  const server = http.createServer((_req, res) => {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(html);
  });
  return new Promise((ok) => server.listen(0, 'localhost', () => ok(server)));
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
  await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
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
  return { call, json };
}

type Row = { browser: string; placement: string; by: string; drivable: boolean };

const section = (page: Page) => page.getByRole('region', { name: 'Navigateurs' });
const row = (page: Page, key: string) => section(page).locator(`.row-item.browser[data-browser="${key}"]`);
const pane = (page: Page, key: string) => page.locator(`[data-pane-key="${key}"]`);

test.describe('an agent’s browser on the user’s screen', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it waits while the user looks away, opens on their return, one pane, nothing reopened after a close, lent by the user (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the browsers are exercised on WebView2 here');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      const launch = await serve(LAUNCH_PAGE);
      const lp = portOf(launch);
      try {
        await addProject(page, app.repo);
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
        const rows = async () => (await agent.json('list_browsers', {})).browsers as Row[];
        const places = async () => (await rows()).map((b) => `${b.browser}=${b.placement}`).sort();
        const conversation = page.locator('aside.side .agent').first();
        const kanban = page.getByRole('button', { name: 'Kanban' }).first();

        // The user looks at the Kanban: the agent's opening waits, nothing comes on their screen.
        await kanban.click();
        await page.waitForTimeout(500);
        const first = await agent.json('open_browser', { launch: true });
        expect(first.place).toBe('waiting');
        expect(first.why).toContain('ne regarde pas ta conversation');
        const launchKey = first.browser as string;
        await expect(row(page, launchKey)).toHaveClass(/waits/, { timeout: 30_000 });
        await expect(row(page, launchKey)).toContainText('veut te montrer la page');
        await page.waitForTimeout(1_000);
        expect(await places()).toEqual([`${launchKey}=hidden`]);
        await expect(pane(page, `page:${launchKey}`)).toHaveCount(0);

        // Back on its conversation: it opens beside it, the conversation staying the active pane.
        await conversation.click();
        await expect(row(page, launchKey)).not.toHaveClass(/waits/);
        await expect.poll(places).toEqual([`${launchKey}=beside`]);
        await expect(page.locator('[data-pane-key^="agent:"]')).toHaveAttribute('data-pane-active', 'true');
        await expect(pane(page, `page:${launchKey}`)).toBeVisible();
        await expect(page.locator('main.conv').first()).toBeInViewport();

        // Another tab it opens takes the place of the first: one pane per agent.
        const second = await agent.json('open_browser', { url: `http://localhost:${lp}/tarifs` });
        expect(second.place).toBe('beside');
        const tabKey = second.browser as string;
        await expect.poll(places).toEqual([`${launchKey}=hidden`, `${tabKey}=beside`].sort());
        await expect(page.locator('[data-pane-key^="page:"]')).toHaveCount(1);

        // The user closes its pane: nothing reopens, nothing waits, and the tool says so.
        await pane(page, `page:${tabKey}`).hover();
        await pane(page, `page:${tabKey}`).getByTestId('pane-close').click();
        await expect(pane(page, `page:${tabKey}`)).toHaveCount(0);
        await page.waitForTimeout(300);
        for (const args of [{ launch: true }, { url: `http://localhost:${lp}/tarifs`, place: 'below' }]) {
          const again = await agent.json('open_browser', args);
          expect(again.place, JSON.stringify(args)).toBe('closed');
          expect(again.why).toContain('a fermé ton volet');
        }
        await page.waitForTimeout(1_000);
        expect(await places()).toEqual([`${launchKey}=hidden`, `${tabKey}=hidden`].sort());
        await expect(row(page, launchKey)).not.toHaveClass(/waits/);
        await expect(row(page, tabKey)).not.toHaveClass(/waits/);
        await expect(page.locator('[data-pane-key^="page:"]')).toHaveCount(0);
        // Elsewhere and back: the agent may show it again.
        await kanban.click();
        await page.waitForTimeout(400);
        await conversation.click();
        await page.waitForTimeout(400);
        expect((await agent.json('open_browser', { launch: true })).place).toBe('beside');
        await expect.poll(places).toEqual([`${launchKey}=beside`, `${tabKey}=hidden`].sort());

        // « Laisser piloter par… »: a browser the user opened alone, lent to the agent.
        await section(page).getByRole('button', { name: 'Nouveau navigateur' }).click();
        const field = section(page).getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
        await field.fill(`localhost:${lp}/compte`);
        await field.press('Enter');
        await expect.poll(async () => (await rows()).filter((b) => b.by === 'user').length).toBe(1);
        const mine = (await rows()).find((b) => b.by === 'user')!;
        expect(mine.drivable).toBe(false);
        const refused = await agent.call('drive_browser', { browser: mine.browser, action: 'scroll', by: 300 });
        expect(refused.error).toBe(true);
        expect(refused.text).toContain('Laisser piloter par');
        await row(page, mine.browser).click({ button: 'right' });
        await page.getByRole('menuitem', { name: 'Laisser piloter par…' }).hover();
        const submenu = page.getByRole('menu').last();
        await submenu.getByRole('menuitemradio').first().click();
        await expect.poll(async () => (await rows()).find((b) => b.browser === mine.browser)?.drivable).toBe(true);
        // Shown (its row clicked), the agent drives it.
        await row(page, mine.browser).click();
        await expect.poll(async () => (await rows()).find((b) => b.browser === mine.browser)?.placement).not.toBe('hidden');
        const driven = await agent.call('drive_browser', { browser: mine.browser, action: 'scroll', by: 300 });
        expect(driven.error, driven.text).toBe(false);
        expect(driven.text).toMatch(/"done":\s*true/);
        // The agent was told by a message, in its conversation.
        await conversation.click();
        await expect(page.getByText(/te laisse piloter le navigateur/).first()).toBeAttached({ timeout: 20_000 });
      } finally {
        launch.close();
      }
    });
  }
});
