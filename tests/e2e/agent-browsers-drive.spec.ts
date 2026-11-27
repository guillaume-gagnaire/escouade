// An agent drives its launch and its page (ESC-679, AB-7 with AP-6; docs/AGENT-BROWSERS.md § 3.3, § 9): a fake agent,
// with its own token of the MCP server, opens its project's launch beside its conversation and drives it gesture by
// gesture: it scrolls to an element, highlights it, goes to the phone's width and back, clicks a button (a real input of
// WebView2: the page hears it with isTrusted), types a name, chooses an option, opens a declared tab; a password is
// refused without the user's leave and typed once given (C2); a file, a download, a form that goes to another origin
// and a secret of Escouade are refused. The user's own gesture in the page (a real wheel of Windows over it) takes the
// hand: the next gesture is refused until « La rendre ». wait_user waits for « Continuer »; look joins an image. Then
// it shows a page of its own (the template review) and drives it by the keys of its template: scroll_to, highlight,
// tab, version; a click is refused there. At 1440 × 900 and at 1024 × 728. On Windows.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, test, unfoldSections, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

const LAUNCH_PAGE = `<!doctype html><html><head><title>Tarifs — demo</title>
<style>body{font:16px system-ui;margin:24px} .tall{height:1800px;background:linear-gradient(#eee,#ccc)}</style></head>
<body><h1>Tarifs</h1><div class="tall"></div>
<section id="tarifs"><p id="pro">Offre Pro 12 €</p>
<button id="go" onclick="fetch('/heard?click=' + event.isTrusted)">Choisir</button>
<input id="name" oninput="document.getElementById('echo').textContent = this.value"><p id="echo"></p>
<input id="pw" type="password">
<select id="plan" onchange="document.getElementById('plan-echo').textContent = this.value"><option value="free">Gratuit</option><option value="pro">Pro</option></select><p id="plan-echo"></p>
<input id="file" type="file"><a id="dl" href="/x.zip" download>Télécharger</a>
<form action="https://evil.escouade-e2e.invalid/steal"><input id="cross" name="q"></form>
<div role="tablist"><button role="tab" id="t1">Avant</button><button role="tab" id="t2" onclick="fetch('/heard?tab=' + event.isTrusted)">Après</button></div>
<div class="tall"></div></section>
<script>addEventListener('wheel', function (e) { fetch('/heard?wheel=' + e.isTrusted); }, { passive: true });</script></body></html>`;

/** A server of the launch page, that keeps what the page tells it it heard. */
function serve(): Promise<{ server: http.Server; heard: string[] }> {
  const heard: string[] = [];
  const server = http.createServer((req, res) => {
    if (req.url?.startsWith('/heard')) {
      heard.push(req.url.slice('/heard?'.length));
      res.writeHead(204).end();
      return;
    }
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(LAUNCH_PAGE);
  });
  return new Promise((ok) => server.listen(0, 'localhost', () => ok({ server, heard })));
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
  const tools = async () => ((await rpc('tools/list', {})).result.tools as { name: string }[]).map((t) => t.name);
  return { instructions: init.result.instructions as string, call, json, page, drive, tools };
}

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;
async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

/**
 * A real wheel of Windows at the point (`x`, `y`) of the app's window, in CSS pixels of its client area: the user's own
 * gesture, `isTrusted`, over the native webview there (never the user's own Escouade: another program's path). The
 * window of the tests is off every screen, where no cursor goes: the wheel is the message Windows sends a window under
 * the cursor (`WM_MOUSEWHEEL`), posted to Chromium's render window of the webview that holds the point. The point is
 * the middle of what the window shows of the webview, its box cut by the window's client area: a page made wider than
 * its window (`setViewportSize` of 1440 × 900 in the CI runner's window of 1028 × 779) puts most of the webview past
 * the window's edge, where Chromium drops a wheel as no user could give it there (ESC-694). Says where it aimed.
 */
function nativeWheel(box: { x: number; y: number; width: number; height: number }, dpr: number): string {
  const script = `
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class Wheel {
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr p, Each e, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public static IntPtr Largest(uint[] pids) {
    IntPtr best = IntPtr.Zero; long area = 0;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (Array.IndexOf(pids, pid) < 0) return true;
      RECT r; GetWindowRect(h, out r);
      long a = (long)(r.Right - r.Left) * (r.Bottom - r.Top);
      if (a > area) { area = a; best = h; }
      return true;
    }, IntPtr.Zero);
    return best;
  }
  // The visible render window of Chromium whose box is the webview's (within 3 px): never the window's own.
  public static string Seen = "";
  public static IntPtr Boxed(IntPtr top, int x, int y, int w, int hh) {
    IntPtr best = IntPtr.Zero;
    EnumChildWindows(top, (h, l) => {
      var s = new System.Text.StringBuilder(256);
      GetClassName(h, s, 256);
      if (s.ToString() != "Chrome_RenderWidgetHostHWND" || !IsWindowVisible(h)) return true;
      RECT r; GetWindowRect(h, out r);
      Seen += r.Left + "," + r.Top + "," + (r.Right - r.Left) + "x" + (r.Bottom - r.Top) + " ";
      if (Math.Abs(r.Left - x) <= 3 && Math.Abs(r.Top - y) <= 3 && Math.Abs(r.Right - r.Left - w) <= 3 && Math.Abs(r.Bottom - r.Top - hh) <= 3) best = h;
      return true;
    }, IntPtr.Zero);
    return best;
  }
}
"@
[void][Wheel]::SetProcessDPIAware()
$exe = ${ps(path.resolve(EXE))}
$pids = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -ieq $exe } | ForEach-Object { [uint32]$_.ProcessId })
$top = [Wheel]::Largest($pids)
if ($top -eq [IntPtr]::Zero) { throw 'no window' }
$o = New-Object Wheel+POINT
$o.X = ${Math.round(box.x * dpr)}; $o.Y = ${Math.round(box.y * dpr)}
[void][Wheel]::ClientToScreen($top, [ref]$o)
$h = [Wheel]::Boxed($top, $o.X, $o.Y, ${Math.round(box.width * dpr)}, ${Math.round(box.height * dpr)})
if ($h -eq [IntPtr]::Zero) { throw ('no webview at ' + $o.X + ',' + $o.Y + ' among ' + [Wheel]::Seen) }
$c = New-Object Wheel+RECT; [void][Wheel]::GetClientRect($top, [ref]$c)
$co = New-Object Wheel+POINT; [void][Wheel]::ClientToScreen($top, [ref]$co)
$left = [Math]::Max($o.X, $co.X); $right = [Math]::Min($o.X + ${Math.round(box.width * dpr)}, $co.X + $c.Right)
$up = [Math]::Max($o.Y, $co.Y); $down = [Math]::Min($o.Y + ${Math.round(box.height * dpr)}, $co.Y + $c.Bottom)
$shown = 'webview ' + $o.X + ',' + $o.Y + ' ${Math.round(box.width * dpr)}x${Math.round(box.height * dpr)} in a window ' + $co.X + ',' + $co.Y + ' ' + $c.Right + 'x' + $c.Bottom
if ($right - $left -lt 8 -or $down - $up -lt 8) { throw ('the window shows none of the ' + $shown) }
$p = New-Object Wheel+POINT
$p.X = [int](($left + $right) / 2); $p.Y = [int](($up + $down) / 2)
$l = [IntPtr](([int64]($p.Y -band 0xFFFF) -shl 16) -bor ([int64]($p.X -band 0xFFFF)))
$w = [IntPtr]([int64](0xFF880000))
foreach ($i in 1..3) { [void][Wheel]::PostMessage($h, 0x020A, $w, $l); Start-Sleep -Milliseconds 120 }
Write-Output ($shown + ', at ' + $p.X + ',' + $p.Y)
`;
  return execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim();
}

type Drive = { hand: string; waiting: boolean; agent: string | null; passwordAsks: string[]; step: number; what: string };
const state = (page: Page, key: string) => invoke<Drive>(page, 'browser_drive_state', { key });

const REVIEW = {
  subject: { ticket: 'POR-41', branch: 'ticket/por-41', target: 'main', tickets: ['POR-41', 'POR-42'] },
  criteria: [
    { text: 'La liste montre 25 adhérents par page', state: 'met' },
    { text: 'Le tri est gardé dans l’adresse', state: 'unmet' },
  ],
  sections: [{ id: 'perf', title: 'Performances', md: 'Rien de lent.' }],
};

test.describe('a fake agent drives its launch and its page', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`it shows and acts on its launch, the user takes the hand back, it walks through its page (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the driving is exercised on WebView2 here (its real inputs)');
      test.setTimeout(300_000);
      const { page } = app;
      await page.setViewportSize(size);
      const launch = await serve();
      const lp = portOf(launch.server);
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
        expect(agent.instructions).toContain('drive_browser');
        expect(await agent.tools()).toContain('drive_browser');
        await page.locator('aside.side .agent').first().click();
        await expect(page.locator('main.conv').first()).toBeVisible();
        await page.waitForTimeout(500);

        const opened = await agent.json('open_browser', { launch: true });
        expect(opened.place).toBe('beside');
        const key = opened.browser as string;
        await expect
          .poll(async () => (await agent.json('list_browsers', {})).browsers.find((b: { browser: string }) => b.browser === key)?.loaded, {
            timeout: 30_000,
          })
          .toBe(true);
        expect((await agent.json('list_browsers', {})).browsers.find((b: { browser: string }) => b.browser === key)).toMatchObject({
          drivable: true,
          launch: true,
        });

        // To show: scroll to an element, highlight it, the phone's width and back.
        const scrolled = await agent.drive({ browser: key, action: 'scroll_to', target: '#tarifs' });
        expect(scrolled).toMatchObject({ done: true, step: 1, hand: 'agent', title: 'Tarifs — demo' });
        expect(scrolled.scrollY).toBeGreaterThan(1000);
        expect(scrolled.what).toContain('#tarifs');
        const lit = await agent.drive({ browser: key, action: 'highlight', target: 'text=Choisir', label: 'Le bouton' });
        expect(lit).toMatchObject({ done: true, step: 2, what: 'Met en évidence « Le bouton »' });
        expect((await state(page, key)).what).toBe('Met en évidence « Le bouton »');
        // What the user sees (AB-8, ESC-680): the band above the tab says who drives and the last gesture; the agent's
        // row wears its initials; the highlight is the browser's overlay, absent from the page's DOM.
        const tabBand = page
          .getByTestId('drive-band')
          .filter({ has: page.getByTestId('drive-hand') })
          .first();
        await expect(tabBand.getByTestId('drive-hand')).toContainText('pilote');
        await expect(tabBand.getByTestId('drive-last')).toContainText('Le bouton');
        await expect(page.locator(`[data-browser="${key}"]`)).toHaveAttribute('data-driven', 'agent');
        const dom = JSON.stringify(agent.page(await agent.call('read_browser', { browser: key, what: 'html' })));
        expect(dom).toContain('Choisir');
        expect(dom).not.toContain('escouade-drive');
        const phone = await agent.drive({ browser: key, action: 'width', width: 'mobile' });
        expect(phone.innerWidth).toBeLessThanOrEqual(400);
        expect(phone.what).toBe('Passe en largeur mobile');
        const full = await agent.drive({ browser: key, action: 'width', width: 'full' });
        expect(full.innerWidth).toBeGreaterThanOrEqual(phone.innerWidth);
        expect(full.what).toBe('Passe en largeur pleine');
        await agent.drive({ browser: key, action: 'clear' });

        // To act, on the launch without asking: a real click (heard as the user's), a name, an option, a declared tab.
        await agent.drive({ browser: key, action: 'click', target: '#go' });
        await expect.poll(() => launch.heard).toContain('click=true');
        await agent.drive({ browser: key, action: 'type', target: '#name', text: 'Ada' });
        await agent.drive({ browser: key, action: 'select', target: '#plan', value: 'Pro' });
        await agent.drive({ browser: key, action: 'tab', target: '#t2' });
        await expect.poll(() => launch.heard).toContain('tab=true');
        const read = agent.page(await agent.call('read_browser', { browser: key, what: 'text', selector: '#echo, #plan-echo' }));
        expect(read.text).toContain('Ada');
        expect(read.text).toContain('pro');
        // tab clicks a declared tab alone.
        const notTab = await agent.call('drive_browser', { browser: key, action: 'tab', target: '#go' });
        expect(notTab.error).toBe(true);
        expect(notTab.text).toContain('onglet déclaré');

        // A password: refused without the user's leave, asked in the band (C2); typed once given.
        const pw = await agent.call('drive_browser', { browser: key, action: 'type', target: '#pw', text: 'motdepasse-e2e' });
        expect(pw.error).toBe(true);
        expect(pw.text).toContain('mot de passe');
        const asked = await state(page, key);
        expect(asked.passwordAsks).toHaveLength(1);
        await invoke(page, 'browser_drive_password', { key, agent: asked.passwordAsks[0], allow: true });
        await expect(page.getByText(/te permet de remplir un mot de passe/).first()).toBeVisible({ timeout: 20_000 });
        expect(await agent.drive({ browser: key, action: 'type', target: '#pw', text: 'motdepasse-e2e' })).toMatchObject({ done: true });

        // Never: a file, a download, a form that goes to another origin, a secret of Escouade.
        for (const [args, said] of [
          [{ action: 'click', target: '#file' }, 'fichier'],
          [{ action: 'click', target: 'text=Télécharger' }, 'télécharge'],
          [{ action: 'type', target: '#cross', text: 'x' }, 'autre origine'],
          [{ action: 'type', target: '#name', text: 'ghp_0123456789abcdefghijklmnopqrstuvwxyzAB' }, 'secret d’Escouade'],
        ] as const) {
          const r = await agent.call('drive_browser', { browser: key, ...args });
          expect(r.error, JSON.stringify(args)).toBe(true);
          expect(r.text, JSON.stringify(args)).toContain(said);
        }

        // look: an image of the view after the gesture.
        const looked = await agent.call('drive_browser', { browser: key, action: 'scroll', by: -300, look: true });
        expect(looked.error, looked.text).toBe(false);
        expect(looked.blocks.map((b) => b.type)).toContain('image');

        // wait_user: « Continuer » in the band, the tool waits for it.
        const waiting = agent.call('drive_browser', { browser: key, action: 'scroll', by: 200, wait_user: true });
        await expect.poll(async () => (await state(page, key)).waiting, { timeout: 15_000 }).toBe(true);
        await tabBand.getByRole('button', { name: 'Continuer' }).click();
        expect(agent.page(await waiting)).toMatchObject({ user: 'continued', hand: 'agent' });

        // The user's own gesture in the page, a real wheel of Windows over it: the hand is theirs, taken by polling.
        const room = page.locator('.room.on').first();
        const box = (await room.boundingBox())!;
        const dpr = await page.evaluate(() => devicePixelRatio);
        // The wheel reaches the page (it says it heard it, isTrusted), again as a user would if it did not; then the
        // hand is the user's: a failure says which of the two did not happen.
        const aimed: string[] = [];
        for (let i = 0; i < 3 && !launch.heard.includes('wheel=true'); i++) {
          aimed.push(nativeWheel(box, dpr));
          await expect
            .poll(() => launch.heard.includes('wheel=true'), { timeout: 4_000 })
            .toBe(true)
            .catch(() => {});
        }
        expect(launch.heard, `the wheel of Windows did not reach the page (${aimed.join('; ')})`).toContain('wheel=true');
        await expect
          .poll(async () => (await state(page, key)).hand, { timeout: 15_000, message: 'the page heard the wheel, the hand did not pass' })
          .toBe('user');
        await expect(tabBand.getByTestId('drive-hand')).toContainText('Tu as la main');
        const refused = await agent.call('drive_browser', { browser: key, action: 'scroll', by: 100 });
        expect(refused.error).toBe(true);
        expect(refused.text).toContain('L’utilisateur a repris la main');
        // « La rendre »: told to the agent, it drives again.
        await tabBand.getByRole('button', { name: 'La rendre' }).click();
        await expect(page.getByText(/l’utilisateur te rend la main/).first()).toBeVisible({ timeout: 20_000 });
        expect(await agent.drive({ browser: key, action: 'scroll', by: 100 })).toMatchObject({ done: true, hand: 'agent' });
        // « Reprendre la main » from the band, then « Ne plus laisser piloter ».
        await tabBand.getByRole('button', { name: 'Reprendre la main' }).click();
        expect((await agent.call('drive_browser', { browser: key, action: 'clear' })).text).toContain('repris la main');
        await invoke(page, 'browser_drive_hand', { key, hand: 'stop' });
        expect((await agent.call('drive_browser', { browser: key, action: 'clear' })).text).toContain('ne laisse plus piloter');

        // Its own page, by the keys of its template: show, tab, version; never a click.
        // Quiet: its row waits, on no screen (it would open beside the conversation the user looks at).
        const shown = await agent.json('show_page', { title: 'Revue de POR-41', template: 'review', data: REVIEW, open: 'quiet' });
        expect(shown.opened).toBe('waiting');
        expect(shown.browser).toBe('page:p-1');
        const notYet = await agent.call('drive_browser', { browser: 'page:p-1', action: 'clear' });
        expect(notYet.text).toContain('n’est pas à l’écran');
        const bounds = { x: Math.round(size.width / 2), y: 48, width: Math.round(size.width / 2), height: size.height - 96 };
        await invoke(page, 'agentpage_place', { page: 'p-1', bounds, placement: 'beside' });
        await expect.poll(async () => (await agent.json('get_page', { page: 'p-1' })).loaded, { timeout: 20_000 }).toBe(true);
        const toCrit = await agent.drive({ browser: 'page:p-1', action: 'scroll_to', target: 'criteria.1' });
        expect(toCrit).toMatchObject({ done: true, version: 1 });
        expect(await agent.drive({ browser: 'page:p-1', action: 'highlight', target: 'criteria.0', label: 'Critère 1' })).toMatchObject({
          done: true,
        });
        expect(await agent.drive({ browser: 'page:p-1', action: 'tab', target: 'tabs.POR-42' })).toMatchObject({ done: true });
        const wrong = await agent.call('drive_browser', { browser: 'page:p-1', action: 'scroll_to', target: 'criteria.9' });
        expect(wrong.text).toContain('criteria.1');
        const click = await agent.call('drive_browser', { browser: 'page:p-1', action: 'click', target: '#subject' });
        expect(click.error).toBe(true);
        expect(click.text).toContain('Sur une page d’agent');
        // A second version, then the first shown again.
        await agent.json('show_page', { title: 'Revue de POR-41', template: 'review', data: REVIEW, page: 'p-1' });
        await expect.poll(async () => (await agent.json('get_page', { page: 'p-1' })).loaded, { timeout: 20_000 }).toBe(true);
        expect(await agent.drive({ browser: 'page:p-1', action: 'version', n: 1 })).toMatchObject({ done: true, version: 1 });
        const looked2 = await agent.call('drive_browser', { browser: 'page:p-1', action: 'scroll', by: 200, look: true });
        expect(looked2.blocks.map((b) => b.type)).toContain('image');
      } finally {
        launch.server.close();
      }
    });
  }
});
