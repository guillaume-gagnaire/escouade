// The agents capture the browsers (ESC-670, AB-3; docs/AGENT-BROWSERS.md § 3 bis): a fake agent, with its own token of the
// MCP server, captures a browser on the project's launch without asking: the view, an element, a phone's width (390 px,
// emulated: the view keeps its own), the whole document. Each answer is an image of MCP (a JPEG within its bounds), the
// part taken, and the same image kept in a file. On another origin the capture waits for the user's leave, in the same
// band as a read. At 1440 × 900 and at 1024 × 728. On Windows: the capture goes through WebView2's protocol.
import fs from 'node:fs';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, test, unfoldSections, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

/** The developed app's page: a badge of a known size, a box that says the width it is laid out at, a long document. */
const LAUNCH_PAGE = `<!doctype html><html><head><title>Tarifs — demo</title>
<meta name="viewport" content="width=device-width"></head>
<body style="font:16px system-ui;margin:24px">
<h1>Tarifs</h1>
<div id="badge" style="width:200px;height:50px;background:#c33;color:#fff">Offre Pro</div>
<p id="w"></p><div style="height:2400px;background:linear-gradient(#fff,#36c)"></div>
<script>const w = () => (document.getElementById('w').textContent = 'largeur ' + innerWidth); addEventListener('resize', w); w();</script>
</body></html>`;

/** A third party's site, where the user would be signed in. */
const OTHER_PAGE = `<!doctype html><title>Mon compte</title><body><h1>Compte de Ada</h1></body>`;

function serve(html: string, host: string): Promise<http.Server> {
  const server = http.createServer((_req, res) => {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(html);
  });
  return new Promise((ok) => server.listen(0, host, () => ok(server)));
}
const portOf = (s: http.Server) => (s.address() as AddressInfo).port;

const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

/** The server as the agent of the launch `argv` reaches it: with its own token. */
function agentServer(argv: string[]) {
  const entry = JSON.parse(fs.readFileSync(argv[argv.indexOf('--mcp-config') + 1], 'utf8')).mcpServers.escouade;
  return { url: entry.url as string, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
}

type Block = { type: string; text?: string; data?: string; mimeType?: string };

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
  const init = await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc('notifications/initialized', {}, true);
  const call = async (name: string, args: object) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    return { blocks: r.content as Block[], error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, JSON.stringify(r.blocks)).toBe(false);
    return JSON.parse(r.blocks[0].text!);
  };
  /** A capture: its image, its notice, then its JSON between the marks of third-party data. */
  const capture = async (args: object) => {
    const r = await call('capture_browser', args);
    expect(r.error, JSON.stringify(r.blocks)).toBe(false);
    const [image, notice, data] = r.blocks;
    expect(image).toMatchObject({ type: 'image', mimeType: 'image/jpeg' });
    expect(notice.text).toContain('third_party_data');
    const said = JSON.parse(data.text!.replace(/^<third_party_data>\n/, '').replace(/\n<\/third_party_data>$/, ''));
    return { jpeg: Buffer.from(image.data!, 'base64'), said };
  };
  return { instructions: init.result.instructions as string, call, json, capture };
}

/** The pixels of a JPEG, by its frame's header (SOF0 to SOF2). */
function jpegSize(b: Buffer): { width: number; height: number } {
  expect([b[0], b[1]]).toEqual([0xff, 0xd8]);
  let at = 2;
  while (at < b.length) {
    const marker = b[at + 1];
    const length = b.readUInt16BE(at + 2);
    if (marker >= 0xc0 && marker <= 0xc2) return { height: b.readUInt16BE(at + 5), width: b.readUInt16BE(at + 7) };
    at += 2 + length;
  }
  throw new Error('no frame in the JPEG');
}

/** A capture's image and its file: a JPEG within the bounds, the same bytes, of the pixels said. */
function checkShot(shot: { jpeg: Buffer; said: { pixels: { width: number; height: number }; file: string } }) {
  const size = jpegSize(shot.jpeg);
  expect(size).toEqual(shot.said.pixels);
  expect(Math.max(size.width, size.height)).toBeLessThanOrEqual(1600);
  expect(shot.jpeg.length).toBeLessThanOrEqual(600_000);
  expect(shot.said.file).toMatch(/[\\/]captures[\\/][^\\/]+[\\/]\d+\.jpg$/);
  expect(fs.readFileSync(shot.said.file).equals(shot.jpeg)).toBe(true);
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

test.describe('a fake agent captures the browsers', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it captures the launch, an element, a phone’s width and the document, and another origin with the user’s leave (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the capture is exercised on WebView2 here (WebKit by the probe)');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      const launch = await serve(LAUNCH_PAGE, 'localhost');
      const other = await serve(OTHER_PAGE, '127.0.0.1');
      const lp = portOf(launch);
      const op = portOf(other);
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
        // The server's instructions name the tool, and what a capture of a signed-in page shows.
        expect(agent.instructions).toContain('capture_browser');
        expect(agent.instructions).toContain('signed in shows what surrounds it');

        await openBrowser(page, `localhost:${lp}/tarifs`, 'Tarifs — demo');
        const key = (await agent.json('list_browsers', {})).browsers[0].browser as string;

        // The view, without asking.
        const view = await agent.capture({ browser: key });
        checkShot(view);
        expect(view.said.url).toBe(`http://localhost:${lp}/tarifs`);
        expect(view.said.selector).toBeNull();
        expect(view.said.width).toBeGreaterThan(400);
        const room = view.said.width as number;

        // An element: its box alone.
        const badge = await agent.capture({ browser: key, selector: '#badge' });
        checkShot(badge);
        expect(badge.said).toMatchObject({ width: 200, height: 50, selector: '#badge' });
        expect(badge.said.pixels.width).toBe(Math.round(200 * badge.said.scale));
        const missing = await agent.call('capture_browser', { browser: key, selector: '#absent' });
        expect(missing.error).toBe(true);
        expect(missing.blocks[0].text).toContain('Aucun élément visible pour « #absent »');

        // A phone's width, emulated: the part taken is 390 px wide; the view keeps its own, and the page its layout.
        const phone = await agent.capture({ browser: key, width: 'mobile', selector: '#badge' });
        checkShot(phone);
        expect(phone.said.width).toBe(200);
        const mobile = await agent.capture({ browser: key, width: 'mobile' });
        checkShot(mobile);
        expect(mobile.said.width).toBe(390);
        expect((await agent.capture({ browser: key })).said.width).toBe(room);

        // The whole document: taller than the view.
        const whole = await agent.capture({ browser: key, full_page: true });
        checkShot(whole);
        expect(whole.said.height).toBeGreaterThan(2400);
        expect(whole.said.height).toBeGreaterThan(view.said.height);
        // The files are numbered, one per capture.
        expect(whole.said.file).not.toBe(view.said.file);

        // Another origin: asked in the band, the same as a read; allowed, it is captured.
        await openBrowser(page, `127.0.0.1:${op}/compte/ada`, 'Mon compte');
        const otherKey = (await agent.json('list_browsers', {})).browsers[1].browser as string;
        const asked = await agent.json('capture_browser', { browser: otherKey });
        expect(asked).toMatchObject({ asked: true, origin: `http://127.0.0.1:${op}` });
        const band = page.getByTestId('consent-band');
        await expect(band).toBeVisible();
        await expect(band).toContainText(`veut lire 127.0.0.1:${op}`);
        await band.getByRole('button', { name: 'Permettre' }).click();
        await expect(band).toBeHidden();
        const shot = await agent.capture({ browser: otherKey });
        checkShot(shot);
        expect(shot.said.url).toBe(`http://127.0.0.1:${op}/compte/ada`);
        // The first browser, now on no screen, is not captured.
        const hidden = await agent.call('capture_browser', { browser: key });
        expect(hidden.error).toBe(true);
        expect(hidden.blocks[0].text).toContain('n’est pas affiché');
      } finally {
        launch.close();
        other.close();
      }
    });
  }
});
