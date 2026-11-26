// The agents' pages by the MCP server (ESC-673, AP-2; docs/AGENT-PAGES.md § 3–4, § 9): a fake agent, with its own token
// of the MCP server, shows a page (the template review), the page opens in its shelter (AP-1) and is read in Escouade's
// world; it then gives its own HTML file with the files it names, as a new version of the same page: the shelter serves
// the copy, its style sheet and its script among it, never the secret beside it nor its .env. get_page says its versions,
// list_browsers lists its row page:p-1 (read_browser refuses it), close_page closes it, kept. « Pages des agents » off
// in Réglages › Agents: the tools leave the list and the instructions, a call is refused. At 1440 × 900 and 1024 × 728.
// On Windows: the read goes through WebView2's protocol (the view of AP-3 is not there yet: the test places the page).
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;

async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
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
  const initialize = async () => {
    const init = await rpc('initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'e2e', version: '1.0.0' },
    });
    await rpc('notifications/initialized', {}, true);
    return init.result.instructions as string;
  };
  const instructions = await initialize();
  const call = async (name: string, args: object) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    return { texts: (r.content as { text: string }[]).map((c) => c.text), error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, r.texts.join('\n')).toBe(false);
    return JSON.parse(r.texts[0]);
  };
  const tools = async () => ((await rpc('tools/list', {})).result.tools as { name: string }[]).map((t) => t.name);
  return { instructions, initialize, call, json, tools };
}

const REVIEW = {
  subject: { ticket: 'POR-41', branch: 'ticket/por-41', target: 'main' },
  facts: { files: 3, added: 214, removed: 38, tests: { passed: 48, total: 48 } },
  criteria: [
    { text: 'La liste montre 25 adhérents par page', state: 'met' },
    { text: 'Le tri gardé dans l’adresse', state: 'partial' },
  ],
};

/** The agent's own page, in its folder: a style sheet and a script it names, a secret beside it, an .env in it. */
function ownPage(cwd: string) {
  const dir = path.join(cwd, 'design');
  fs.mkdirSync(path.join(dir, 'js'), { recursive: true });
  fs.writeFileSync(path.join(cwd, 'secret-hors.txt'), 'TOP-SECRET-HORS');
  fs.writeFileSync(path.join(dir, '.env'), 'TOKEN=TOP-SECRET-ENV');
  fs.writeFileSync(
    path.join(dir, 'revue.html'),
    `<!doctype html><html><head><title>Ma revue</title><link rel="stylesheet" href="style.css"></head>
<body><h1 id="titre">Ma revue v2</h1><p id="css">?</p><p id="js">?</p>
<img src="../secret-hors.txt"><script src=".env"></script>
<script type="module" src="js/app.js"></script></body></html>`,
  );
  fs.writeFileSync(path.join(dir, 'style.css'), '#titre { color: rgb(1, 2, 3); }');
  fs.writeFileSync(
    path.join(dir, 'js', 'app.js'),
    `import { said } from './said.js';
document.getElementById('js').textContent = said;
document.getElementById('css').textContent = getComputedStyle(document.getElementById('titre')).color;
fetch('../secret-hors.txt').then((r) => r.text()).then((t) => { document.body.dataset.leak = t; }).catch(() => {});`,
  );
  fs.writeFileSync(path.join(dir, 'js', 'said.js'), "export const said = 'script relatif servi';");
}

/** The text of page `p` in its shelter, read in Escouade's world, once it says `wanted`. */
async function textOf(page: Page, p: string, wanted: string) {
  let text = '';
  await expect
    .poll(
      async () => {
        text = await invoke<string>(page, 'agentpage_read', { page: p, selector: null, text: true }).catch((e) => String(e));
        return text;
      },
      { timeout: 30_000 },
    )
    .toContain(wanted);
  return text;
}

test.describe('a fake agent shows its pages', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it shows a page, updates it by a version, reads its state and closes it; off, the tools go (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the read bridge is exercised on WebView2 here (WebKit by the probe)');
      test.setTimeout(240_000);
      const { page } = app;
      await page.setViewportSize(size);
      await addProject(page, app.repo);
      await expect.poll(() => streams(app).length, { timeout: 30_000 }).toBeGreaterThanOrEqual(1);
      const launch = streams(app)[0];
      const agent = await fakeAgent(agentServer(launch.argv));
      // The switch is on: the server's instructions and its list name the tools.
      expect(agent.instructions).toContain('show_page');
      expect(await agent.tools()).toEqual(expect.arrayContaining(['show_page', 'get_page', 'close_page']));

      // The template: a page p-1, its row waiting in « Navigateurs ».
      const shown = await agent.json('show_page', { title: 'Revue de POR-41', template: 'review', data: REVIEW });
      expect(shown).toMatchObject({ page: 'p-1', browser: 'page:p-1', version: 1, opened: 'waiting', ask: null });
      let rows = (await agent.json('list_browsers', {})).browsers;
      expect(rows.find((r: { browser: string }) => r.browser === 'page:p-1')).toMatchObject({
        name: 'Revue de POR-41',
        by: 'agent',
        placement: 'hidden',
        launch: false,
      });
      // Put beside the conversation, as the view of AP-3 will: it is on screen, and read in Escouade's world.
      const bounds = { x: Math.round(size.width / 2), y: 48, width: Math.round(size.width / 2), height: size.height - 96 };
      await invoke(page, 'agentpage_place', { page: 'p-1', bounds, placement: 'beside' });
      await textOf(page, 'p-1', 'La liste montre 25 adhérents par page');
      await expect.poll(async () => (await agent.json('get_page', { page: 'p-1' })).loaded, { timeout: 20_000 }).toBe(true);
      let state = await agent.json('get_page', { page: 'p-1' });
      expect(state).toMatchObject({ page: 'p-1', template: 'review', version: 1, row: 'open', placement: 'beside', on_screen: true });
      rows = (await agent.json('list_browsers', {})).browsers;
      expect(rows.find((r: { browser: string }) => r.browser === 'page:p-1')).toMatchObject({ placement: 'beside', loaded: true });
      // read_browser does not read an agent's page.
      const refused = await agent.call('read_browser', { browser: 'page:p-1' });
      expect(refused.error).toBe(true);
      expect(refused.texts[0]).toContain('get_page');

      // A new version: the agent's own HTML file, with the files it names, from its own folder alone.
      ownPage(launch.cwd);
      const second = await agent.json('show_page', { title: 'Ma revue', template: 'html', file: 'design/revue.html', page: 'p-1' });
      expect(second).toMatchObject({ page: 'p-1', version: 2, files: 4 });
      expect([...second.left_out].sort()).toEqual(['../secret-hors.txt', '.env']);
      // Reloaded where it is: its style sheet and its scripts served, nothing from outside.
      const text = await textOf(page, 'p-1', 'script relatif servi');
      expect(text).toContain('Ma revue v2');
      expect(text).toContain('rgb(1, 2, 3)');
      const html = await invoke<string>(page, 'agentpage_read', { page: 'p-1', selector: 'body', text: false });
      expect(html).not.toContain('TOP-SECRET');
      state = await agent.json('get_page', { page: 'p-1' });
      expect(state.version).toBe(2);
      expect(state.versions.map((v: { version: number }) => v.version)).toEqual([1, 2]);
      expect(state.title).toBe('Ma revue');
      // What the page itself shows (its native webview, which a capture of the window does not hold).
      const ext = await invoke<string>(page, 'agentpage_capture', { page: 'p-1', path: test.info().outputPath(`page-${size.width}`) });
      expect(fs.statSync(test.info().outputPath(`page-${size.width}.${ext}`)).size).toBeGreaterThan(1000);

      // Closed: its row and its webview gone, the page kept.
      expect(await agent.json('close_page', { page: 'p-1' })).toMatchObject({ closed: true, was_open: true, kept: true });
      await expect.poll(() => invoke(page, 'agentpage_view', { page: 'p-1' }), { timeout: 10_000 }).toBeNull();
      rows = (await agent.json('list_browsers', {})).browsers;
      expect(rows.some((r: { browser: string }) => r.browser === 'page:p-1')).toBe(false);
      expect((await agent.json('get_page', { page: 'p-1' })).row).toBe('closed');

      // « Pages des agents » off in Réglages › Agents: out of the list and of the instructions, a call refused.
      const dialog = await openSettingsPage(page, 'Agents');
      const sw = dialog.getByRole('switch', { name: 'Pages des agents' });
      await expect(sw).toHaveAttribute('aria-checked', 'true');
      await sw.click();
      await expect(sw).toHaveAttribute('aria-checked', 'false');
      await closeSettings(dialog);
      await expect.poll(async () => (await agent.tools()).includes('show_page'), { timeout: 10_000 }).toBe(false);
      expect(await agent.tools()).toContain('list_browsers');
      expect(await agent.initialize()).not.toContain('show_page');
      const off = await agent.call('show_page', { title: 'x', template: 'html', html: '<p>x</p>' });
      expect(off.error).toBe(true);
      expect(off.texts[0]).toContain('Pages des agents');
    });
  }
});
