// The frames (ESC-682, AB-11; docs/AGENT-BROWSERS.md § 2 bis.4, § 3.3): a fake agent, with its own token of the MCP
// server, opens its project's launch beside its conversation. The launch's page (on localhost) holds a frame of
// another origin and another site (127.0.0.1), as claude.ai holds an artifact. Under Windows, drive_browser scrolls to
// an element of the frame and highlights it in the frame itself (its own isolated world, reached by the frame's
// session of the DevTools protocol): the frame says it scrolled and it holds Escouade's contour; clear takes it off.
// Without target, the frame as a whole is highlighted (WebKit's way). The frame's content stays out of read_browser
// (an empty <iframe src=…>) and of browser_console. At 1440 × 900 and at 1024 × 728 (and at the CI's size with
// ESCOUADE_E2E_VIEWPORT). On Windows.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import type { AddressInfo } from 'node:net';
import { addProject, closeSettings, expect, test, unfoldSections, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

/** The page of the other origin, in the frame: tall, an element far down, and what it says of itself to its server. */
const FRAME_PAGE = `<!doctype html><html><head><title>Artefact du cadre</title>
<style>body{font:16px system-ui;margin:16px} .tall{height:2400px;background:linear-gradient(#fafafa,#ddd)}</style></head>
<body><h1>Plan de la 2.4</h1><p>CONTENU-SECRET-DU-CADRE</p><div class="tall"></div>
<section id="risques"><h2>Tableau des risques</h2><p>Le risque principal.</p></section><div class="tall"></div>
<script>
console.log('JOURNAL-SECRET-DU-CADRE');
setInterval(function () {
  var mark = !!document.querySelector('[data-escouade="escouade-drive-highlight"]');
  fetch('/state?y=' + Math.round(scrollY) + '&mark=' + mark).catch(function () {});
}, 200);
</script></body></html>`;

const launchPage = (framePort: number) => `<!doctype html><html><head><title>Artefact — demo</title>
<style>body{font:16px system-ui;margin:24px} .tall{height:900px;background:#eee}
iframe{display:block;width:90%;height:420px;border:1px solid #888}</style></head>
<body><h1>La page</h1><div class="tall"></div>
<iframe id="art" title="Artefact" src="http://127.0.0.1:${framePort}/art"></iframe>
<div class="tall"></div></body></html>`;

type Heard = { y: number; mark: boolean };

/** The frame's server, on 127.0.0.1: another origin and another site than the launch's localhost. */
function serveFrame(): Promise<{ server: http.Server; last: () => Heard | null }> {
  let last: Heard | null = null;
  const server = http.createServer((req, res) => {
    if (req.url?.startsWith('/state')) {
      const q = new URL(req.url, 'http://x').searchParams;
      last = { y: Number(q.get('y')), mark: q.get('mark') === 'true' };
      res.writeHead(204).end();
      return;
    }
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(FRAME_PAGE);
  });
  return new Promise((ok) => server.listen(0, '127.0.0.1', () => ok({ server, last: () => last })));
}

function serveLaunch(framePort: number): Promise<http.Server> {
  const server = http.createServer((_req, res) => {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(launchPage(framePort));
  });
  return new Promise((ok) => server.listen(0, 'localhost', () => ok(server)));
}
const portOf = (s: http.Server) => (s.address() as AddressInfo).port;

const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

function agentServer(argv: string[]) {
  const entry = JSON.parse(fs.readFileSync(argv[argv.indexOf('--mcp-config') + 1], 'utf8')).mcpServers.escouade;
  return { url: entry.url as string, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
}

type Block = { type: string; text?: string };
type Called = { blocks: Block[]; text: string; error: boolean };

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
  const call = async (name: string, args: object): Promise<Called> => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    const blocks = r.content as Block[];
    return { blocks, text: blocks.map((c) => c.text ?? '').join('\n'), error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, r.text).toBe(false);
    return JSON.parse(r.text);
  };
  /** A page's answer, marked as third-party data: what is between the marks. */
  const page = (r: Called) => {
    expect(r.error, r.text).toBe(false);
    const marked = r.blocks.map((b) => b.text ?? '').find((t) => t.startsWith('<third_party_data>'));
    expect(marked, r.text).toBeTruthy();
    return JSON.parse(marked!.replace(/^<third_party_data>\n/, '').replace(/\n<\/third_party_data>$/, ''));
  };
  const drive = async (args: object) => page(await call('drive_browser', args));
  return { call, json, page, drive };
}

test.describe('a fake agent shows an element of a frame of another origin', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it scrolls to it and highlights it in the frame, the frame's content stays unread (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the frame is reached by WebView2 here (its sessions of the DevTools protocol)');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      const frame = await serveFrame();
      const launch = await serveLaunch(portOf(frame.server));
      const fp = portOf(frame.server);
      try {
        await addProject(page, app.repo);
        await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
        const dialog = page.getByRole('dialog', { name: 'Réglages' });
        await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
        const cfg = dialog.getByRole('group', { name: 'Commande 1' });
        await cfg.getByLabel('Nom').fill('web');
        await cfg.getByLabel('Commande', { exact: true }).fill('node -e "1"');
        await cfg.getByRole('checkbox', { name: 'Ouvrir à son lancement' }).check();
        await cfg.getByLabel('Adresse à ouvrir').fill(`http://localhost:${portOf(launch)}/`);
        await closeSettings(dialog);
        await unfoldSections(page);

        await expect.poll(() => streams(app).length, { timeout: 30_000 }).toBeGreaterThanOrEqual(1);
        const agent = await fakeAgent(agentServer(streams(app)[0].argv));
        await page.locator('aside.side .agent').first().click();
        await expect(page.locator('main.conv').first()).toBeVisible();
        await page.waitForTimeout(500);

        const opened = await agent.json('open_browser', { launch: true });
        expect(opened.place).toBe('beside');
        const key = opened.browser as string;
        const row = async () => (await agent.json('list_browsers', {})).browsers.find((b: { browser: string }) => b.browser === key);
        await expect.poll(async () => (await row())?.loaded, { timeout: 30_000 }).toBe(true);
        await expect.poll(async () => (await row())?.placement, { timeout: 15_000 }).toBe('beside');
        // The frame is loaded and tells its server where it is.
        await expect.poll(() => frame.last(), { timeout: 20_000 }).not.toBeNull();
        expect(frame.last()!.y).toBe(0);

        // The frame's content stays out of read_browser: an empty <iframe src=…>.
        const html = agent.page(await agent.call('read_browser', { browser: key, what: 'html' }));
        const htmlText = JSON.stringify(html);
        expect(htmlText).toContain('<iframe');
        expect(htmlText).toContain(`127.0.0.1:${fp}/art`);
        expect(htmlText).not.toContain('CONTENU-SECRET-DU-CADRE');
        expect(htmlText).not.toContain('Tableau des risques');
        const text = JSON.stringify(agent.page(await agent.call('read_browser', { browser: key, what: 'text' })));
        expect(text).toContain('La page');
        expect(text).not.toContain('CONTENU-SECRET-DU-CADRE');

        // scroll_to in the frame: the frame's own document scrolls to the element.
        const scrolled = await agent.drive({ browser: key, action: 'scroll_to', frame: '#art', target: '#risques' });
        expect(scrolled).toMatchObject({ done: true, frame: 'inside', hand: 'agent', title: 'Artefact — demo' });
        expect(scrolled.what).toBe('Défile jusqu’à « #risques »');
        await expect.poll(() => frame.last()!.y, { timeout: 10_000 }).toBeGreaterThan(1500);
        // The <iframe> itself was brought into the view of the page.
        expect(scrolled.scrollY).toBeGreaterThan(300);

        // highlight in the frame: Escouade's contour is in the frame's document.
        const lit = await agent.drive({ browser: key, action: 'highlight', frame: '#art', target: '#risques', label: 'Les risques' });
        expect(lit).toMatchObject({ done: true, frame: 'inside', what: 'Met en évidence « Les risques »' });
        await expect.poll(() => frame.last()!.mark, { timeout: 10_000 }).toBe(true);
        const looked = await agent.call('drive_browser', { browser: key, action: 'scroll', by: 1, look: true });
        expect(looked.blocks.map((b) => b.type)).toContain('image');
        // clear takes it off the frame.
        await agent.drive({ browser: key, action: 'clear' });
        await expect.poll(() => frame.last()!.mark, { timeout: 10_000 }).toBe(false);

        // Without target: the frame as a whole, highlighted in the page (as under WebKit); nothing in the frame.
        const whole = await agent.drive({ browser: key, action: 'highlight', frame: '#art', label: 'L’artefact' });
        expect(whole).toMatchObject({ done: true, frame: 'whole', what: 'Met en évidence « L’artefact »' });
        await page.waitForTimeout(600);
        expect(frame.last()!.mark).toBe(false);
        // A highlight in the frame after it takes the page's off, and back.
        expect(await agent.drive({ browser: key, action: 'highlight', frame: '#art', target: '#risques' })).toMatchObject({
          frame: 'inside',
        });
        await expect.poll(() => frame.last()!.mark, { timeout: 10_000 }).toBe(true);
        await agent.drive({ browser: key, action: 'highlight', target: 'h1', label: 'Le titre' });
        await expect.poll(() => frame.last()!.mark, { timeout: 10_000 }).toBe(false);

        // An element the frame does not hold, a frame that is none, a target that is no <iframe>.
        for (const [args, said] of [
          [{ action: 'scroll_to', frame: '#art', target: '#nulle-part' }, 'Aucun élément visible pour « #nulle-part »'],
          [{ action: 'highlight', frame: '#pas-de-cadre', target: '#risques' }, 'Aucun cadre visible'],
          [{ action: 'highlight', frame: 'h1', target: '#risques' }, 'ne désigne pas un <iframe>'],
          [{ action: 'click', frame: '#art', target: '#risques' }, 'scroll_to et highlight seuls'],
        ] as const) {
          const r = await agent.call('drive_browser', { browser: key, ...args });
          expect(r.error, JSON.stringify(args)).toBe(true);
          expect(r.text, JSON.stringify(args)).toContain(said);
        }

        // The way the frame was reached, told by the log: its own session when it lives in its own process.
        const reached = fs
          .readFileSync(path.join(app.data, 'app.log'), 'utf8')
          .split('\n')
          .filter((l) => l.includes('a frame of'));
        expect(reached.slice(-1)[0], 'a frame of another site lives in its own process').toContain('by its own session');
        // Nothing of the frame's console reaches browser_console, nor of its answers what it holds.
        const logs = JSON.stringify(agent.page(await agent.call('browser_console', { browser: key })));
        expect(logs).not.toContain('JOURNAL-SECRET-DU-CADRE');
        expect(JSON.stringify([scrolled, lit, whole])).not.toContain('Artefact du cadre');
      } finally {
        launch.close();
        frame.server.close();
      }
    });
  }
});
