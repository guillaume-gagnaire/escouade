// The flagship case in mode 2, a real tab of a browser (ESC-685, AB-12; docs/AGENT-BROWSERS.md § 2 and § 2 bis): a fake
// agent, with its own token of the MCP server, opens its project's launch beside the conversation the user looks at,
// scrolls to its prices, highlights the Pro offer, opens a tab of the page and goes to the phone's width, each gesture
// seen by a real capture of the window as Windows draws it (`PrintWindow`, the native webview included). The user turns
// the wheel over the page (a real wheel of Windows): the hand passes, the next gesture is refused, « La rendre » gives it
// back. A password is refused without the user's leave, and still when they refuse it. Another origin is asked in the
// conversation and opens once allowed. Outside the launch (its artifact, served locally as ESCOUADE_E2E_ARTIFACT_ORIGIN):
// to show goes without asking and tells nothing of the page; a click waits for the leave to read and act, then goes; a
// password stays refused, even with that leave. At 1440 × 900 and at 1024 × 728 (and at the CI's size with
// ESCOUADE_E2E_VIEWPORT). On Windows.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, test, unfoldSections, windowScale, type App } from './fixture';

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

/** The artifacts' origin of this test: a port of the worktree's range when it has one (browsers-artifact takes + 7). */
const BASE = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
const PORT = BASE > 0 ? BASE + 8 : 48_000 + (process.pid % 1000);
const ORIGIN = `http://127.0.0.1:${PORT}`;
const ARTIFACT = `${ORIGIN}/code/artifact/plan-24`;
/** A site outside the launch: `.invalid` never resolves (RFC 2606), so nothing leaves the machine. */
const OTHER = 'http://docs.escouade-e2e.invalid';

const LAUNCH_PAGE = `<!doctype html><html><head><title>Tarifs — demo</title>
<style>body{font:16px system-ui;margin:24px;background:#fff} .tall{height:1600px;background:linear-gradient(#fff,#9ab)}
#tarifs{background:#246;color:#fff;padding:24px} .carte-pro{padding:16px;max-width:600px}
[role=tabpanel][hidden]{display:none}</style></head>
<body><h1>Tarifs</h1><div class="tall"></div>
<section id="tarifs"><h2>Tarifs</h2><div class="carte-pro"><p>Offre Pro</p>
<div role="tablist"><button role="tab" id="mensuel" aria-selected="true">Mensuel</button><button role="tab" id="annuel" aria-selected="false"
 onclick="fetch('/heard?tab=' + event.isTrusted); document.getElementById('p-m').hidden = true; document.getElementById('p-a').hidden = false; this.setAttribute('aria-selected', 'true')">Annuel</button></div>
<p role="tabpanel" id="p-m">12 € par mois</p><p role="tabpanel" id="p-a" hidden style="font-size:48px">120 € par an</p></div>
<input id="pw" type="password"></section><div class="tall"></div>
<script>addEventListener('wheel', function (e) { fetch('/heard?wheel=' + e.isTrusted); }, { passive: true });</script></body></html>`;

const ARTIFACT_PAGE = `<!doctype html><html><head><title>Plan de la 2.4</title>
<style>body{font:16px system-ui;margin:24px} .tall{height:1400px;background:linear-gradient(#fff,#cba)}</style></head>
<body><h1>Plan de la 2.4</h1><p>Compte de Ada, privé</p><div class="tall"></div>
<table id="risques"><tr><td>Risques</td></tr></table>
<button id="ok" onclick="fetch('/heard?artifact-click=' + event.isTrusted)">Valider</button>
<input id="pw" type="password"><div class="tall"></div></body></html>`;

/** A server of `html`, that keeps what the page tells it it heard. */
function serve(html: string, port: number, host: string): Promise<{ server: http.Server; heard: string[] }> {
  const heard: string[] = [];
  const server = http.createServer((req, res) => {
    if (req.url?.startsWith('/heard')) {
      heard.push(req.url.slice('/heard?'.length));
      res.writeHead(204).end();
      return;
    }
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' });
    res.end(html);
  });
  return new Promise((ok, ko) => {
    server.once('error', ko);
    server.listen(port, host, () => ok({ server, heard }));
  });
}
const portOf = (s: http.Server) => (s.address() as AddressInfo).port;

const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

/** The server as the agent of the launch `argv` reaches it: with its own token. */
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
  return { call, json, drive };
}

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;
async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

type Drive = { hand: string; waiting: boolean; agent: string | null; passwordAsks: string[]; step: number; what: string };
const state = (page: Page, key: string) => invoke<Drive>(page, 'browser_drive_state', { key });

const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

/** The part of PowerShell that finds the app's largest window, among the processes of the worktree's escouade.exe. */
const LARGEST = `
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
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
  }`;
const TOP = `
$exe = ${ps(path.resolve(EXE))}
$pids = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -ieq $exe } | ForEach-Object { [uint32]$_.ProcessId })`;

type Box = { x: number; y: number; width: number; height: number };

/**
 * The webview's box of the app's window as Windows draws it (`PrintWindow` with its content, native webviews
 * included), into `file`: its pixels' hash, and how many colours a grid of 24 × 24 points of it sees (a blank view
 * sees one). What the user sees, not what the page says it shows.
 */
function capture(file: string, box: Box, scale: number): { hash: string; colours: number } {
  const [x, y, w, h] = [box.x, box.y, box.width, box.height].map((v) => Math.round(v * scale));
  const script = `
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Shot {${LARGEST}
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
}
"@
[void][Shot]::SetProcessDPIAware()${TOP}
$h = [Shot]::Largest($pids)
if ($h -eq [IntPtr]::Zero) { throw ('no window for ' + $exe) }
$r = New-Object Shot+RECT
[void][Shot]::GetWindowRect($h, [ref]$r)
$o = New-Object Shot+POINT
[void][Shot]::ClientToScreen($h, [ref]$o)
$b = New-Object System.Drawing.Bitmap ($r.Right - $r.Left), ($r.Bottom - $r.Top)
$g = [System.Drawing.Graphics]::FromImage($b)
$dc = $g.GetHdc()
[void][Shot]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$left = ($o.X - $r.Left) + ${x}; $top = ($o.Y - $r.Top) + ${y}
$cw = [Math]::Min(${w}, $b.Width - $left); $ch = [Math]::Min(${h}, $b.Height - $top)
if ($cw -lt 8 -or $ch -lt 8) { throw ('the window shows none of the webview at ' + $left + ',' + $top) }
$c = $b.Clone((New-Object System.Drawing.Rectangle $left, $top, $cw, $ch), $b.PixelFormat)
$c.Save(${ps(file)})
$seen = @{}
foreach ($i in 0..23) { foreach ($j in 0..23) { $seen[$c.GetPixel([int]($i * ($cw - 1) / 23), [int]($j * ($ch - 1) / 23)).ToArgb()] = 1 } }
$ms = New-Object System.IO.MemoryStream
$c.Save($ms, [System.Drawing.Imaging.ImageFormat]::Bmp)
$sha = [System.Security.Cryptography.SHA256]::Create()
$hex = [BitConverter]::ToString($sha.ComputeHash($ms.ToArray())).Replace('-', '')
Write-Output ($hex + ' ' + $seen.Count)
`;
  const [hash, colours] = execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' })
    .trim()
    .split(' ');
  return { hash, colours: Number(colours) };
}

/**
 * A real wheel of Windows over the webview's box, in CSS pixels of the window's client area: the user's own gesture,
 * `isTrusted`. The window of the tests is off every screen, where no cursor goes: the wheel is the message Windows sends
 * a window under the cursor (`WM_MOUSEWHEEL`), posted to Chromium's render window whose box is the webview's, at the
 * middle of what the window shows of it (ESC-679, ESC-694). Says where it aimed.
 */
function nativeWheel(box: Box, dpr: number): string {
  const [bw, bh] = [Math.round(box.width * dpr), Math.round(box.height * dpr)];
  const script = `
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Wheel {${LARGEST}
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr p, Each e, IntPtr l);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
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
[void][Wheel]::SetProcessDPIAware()${TOP}
$top = [Wheel]::Largest($pids)
if ($top -eq [IntPtr]::Zero) { throw 'no window' }
$o = New-Object Wheel+POINT
$o.X = ${Math.round(box.x * dpr)}; $o.Y = ${Math.round(box.y * dpr)}
[void][Wheel]::ClientToScreen($top, [ref]$o)
$h = [Wheel]::Boxed($top, $o.X, $o.Y, ${bw}, ${bh})
if ($h -eq [IntPtr]::Zero) { throw ('no webview at ' + $o.X + ',' + $o.Y + ' among ' + [Wheel]::Seen) }
$c = New-Object Wheel+RECT; [void][Wheel]::GetClientRect($top, [ref]$c)
$co = New-Object Wheel+POINT; [void][Wheel]::ClientToScreen($top, [ref]$co)
$left = [Math]::Max($o.X, $co.X); $right = [Math]::Min($o.X + ${bw}, $co.X + $c.Right)
$up = [Math]::Max($o.Y, $co.Y); $down = [Math]::Min($o.Y + ${bh}, $co.Y + $c.Bottom)
$shown = 'webview ' + $o.X + ',' + $o.Y + ' ${bw}x${bh} in a window ' + $co.X + ',' + $co.Y + ' ' + $c.Right + 'x' + $c.Bottom
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

/** Sends `text` from the agent's conversation: its composer, never the address bar of a tab open beside it. */
async function send(page: Page, text: string) {
  const box = page.locator('main.conv').first().getByRole('textbox').last();
  await box.fill(text);
  await box.press('Enter');
}

const section = (page: Page) => page.getByRole('region', { name: 'Navigateurs' });
const row = (page: Page, key: string) => section(page).locator(`.row-item.browser[data-browser="${key}"]`);

test.describe('the flagship case in a real tab', () => {
  test.use({
    preStart: { run: () => ({ mcpEnabled: true }) },
    appEnv: { ESCOUADE_E2E_ARTIFACT_ORIGIN: ORIGIN },
  });

  for (const size of SIZES) {
    test(`an agent shows its launch beside its conversation, the user takes the hand and gives it back, outside the launch it shows and asks (${size.width} × ${size.height})`, async ({
      app,
    }) => {
      test.skip(process.platform !== 'win32', 'the flagship case is exercised on WebView2 here (its real inputs and captures)');
      test.setTimeout(360_000);
      const { page } = app;
      await page.setViewportSize(size);
      const launch = await serve(LAUNCH_PAGE, 0, 'localhost');
      const art = await serve(ARTIFACT_PAGE, PORT, '127.0.0.1');
      const lp = portOf(launch.server);
      const shots = test.info().outputPath('shots');
      fs.mkdirSync(shots, { recursive: true });
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
        await page.locator('aside.side .agent').first().click();
        const conv = page.locator('main.conv').first();
        await expect(conv).toBeVisible();
        await page.waitForTimeout(500);

        // 1. Beside its conversation: the zone splits, the conversation stays, its tab is driven by it.
        const opened = await agent.json('open_browser', { launch: true, place: 'beside' });
        expect(opened.place).toBe('beside');
        const key = opened.browser as string;
        await expect
          .poll(
            async () => {
              const b = (await agent.json('list_browsers', {})).browsers.find((r: { browser: string }) => r.browser === key);
              return `${b?.placement} ${b?.loaded}`;
            },
            { timeout: 30_000 },
          )
          .toBe('beside true');
        await expect(conv).toBeVisible();
        const room = page.locator('.room.on').first();
        await expect(room).toBeVisible();
        const box = (await room.boundingBox())!;
        const convBox = (await conv.boundingBox())!;
        // Beside, not over: the tab is right of the conversation.
        expect(box.x).toBeGreaterThanOrEqual(convBox.x + convBox.width - 2);
        const band = page
          .getByTestId('drive-band')
          .filter({ has: page.getByTestId('drive-hand') })
          .first();

        // Each gesture seen as the user sees it: a real capture of the window, the webview's box, changed by it. Below
        // the band, which says each gesture too: the page itself must show it.
        const scale = await windowScale(page);
        const view = { ...box, y: box.y + 48, height: box.height - 48 };
        let n = 0;
        let last: { hash: string; colours: number } | undefined;
        const seen = async (what: string, changes = true) => {
          await page.waitForTimeout(600);
          let shot = capture(path.join(shots, `${String(++n).padStart(2, '0')}-${what}-${size.width}.png`), view, scale);
          // A frame may lag the gesture: once more before saying it did not show.
          if (changes && shot.hash === last?.hash) {
            await page.waitForTimeout(1_000);
            shot = capture(path.join(shots, `${String(n).padStart(2, '0')}-${what}-${size.width}.png`), view, scale);
          }
          expect(shot.colours, `${what}: a blank view`).toBeGreaterThan(1);
          if (last && changes) expect(shot.hash, `${what}: the window shows no change`).not.toBe(last.hash);
          last = shot;
        };
        await seen('ouvre');

        // 2. To scroll, to highlight.
        const scrolled = await agent.drive({ browser: key, action: 'scroll_to', target: '#tarifs' });
        expect(scrolled).toMatchObject({ done: true, hand: 'agent', title: 'Tarifs — demo' });
        expect(scrolled.scrollY).toBeGreaterThan(1000);
        // From its first gesture, the band says who drives and its row wears it.
        await expect(band.getByTestId('drive-hand')).toContainText('pilote');
        await expect(row(page, key)).toHaveAttribute('data-driven', 'agent');
        await seen('defile');
        const lit = await agent.drive({ browser: key, action: 'highlight', target: '#tarifs .carte-pro', label: 'Offre Pro' });
        expect(lit).toMatchObject({ done: true, what: 'Met en évidence « Offre Pro »' });
        await expect(band.getByTestId('drive-last')).toContainText('Offre Pro');
        await seen('met-en-evidence');

        // 3. A tab of the page, « Annuel »: a real click, heard as the user's; the annual price shows.
        await agent.drive({ browser: key, action: 'clear' });
        const tabbed = await agent.drive({ browser: key, action: 'tab', target: '[role=tab]:nth-of-type(2)' });
        expect(tabbed).toMatchObject({ done: true });
        await expect.poll(() => launch.heard).toContain('tab=true');
        // It captures for itself, to check what it shows: on its launch, without asking.
        const own = await agent.call('capture_browser', { browser: key });
        expect(own.error, own.text).toBe(false);
        expect(own.blocks.map((b) => b.type)).toContain('image');
        await seen('onglet-annuel');

        // 4. The phone's width: the page is 390 px, the bar's « Mobile » follows.
        const phone = await agent.drive({ browser: key, action: 'width', width: 'mobile' });
        expect(phone.innerWidth).toBeLessThanOrEqual(400);
        expect(phone.what).toBe('Passe en largeur mobile');
        // (A narrow pane folds the segments into its menu: the radio is there, hidden.)
        await expect(page.locator('.modes [role=radio][aria-label="Mobile"]').first()).toHaveAttribute('aria-checked', 'true');
        // A pane narrower than a phone (340 px beside the conversation at 1024) is already as narrow: nothing to see.
        await seen('largeur-mobile', box.width > 400);
        expect((await state(page, key)).step).toBeGreaterThanOrEqual(5);

        // 5. A password, on the launch: refused without the user's leave, asked in the band; refused there, it holds.
        const pw = await agent.call('drive_browser', { browser: key, action: 'type', target: '#pw', text: 'motdepasse-e2e' });
        expect(pw.error).toBe(true);
        expect(pw.text).toContain('mot de passe');
        const pwBand = page.getByTestId('password-band');
        await expect(pwBand).toContainText('veut remplir un mot de passe');
        await pwBand.getByRole('button', { name: 'Refuser' }).click();
        await expect(pwBand).toBeHidden();
        const again = await agent.call('drive_browser', { browser: key, action: 'type', target: '#pw', text: 'motdepasse-e2e' });
        expect(again.error).toBe(true);
        expect(again.text).toContain('mot de passe');

        // 6. The user turns the wheel over the page: the hand is theirs, the next gesture is refused.
        await agent.drive({ browser: key, action: 'width', width: 'full' });
        await page.waitForTimeout(500);
        const wheelBox = (await room.boundingBox())!;
        const dpr = await page.evaluate(() => devicePixelRatio);
        const aimed: string[] = [];
        for (let i = 0; i < 3 && !launch.heard.includes('wheel=true'); i++) {
          aimed.push(nativeWheel(wheelBox, dpr));
          await expect
            .poll(() => launch.heard.includes('wheel=true'), { timeout: 4_000 })
            .toBe(true)
            .catch(() => {});
        }
        expect(launch.heard, `the wheel of Windows did not reach the page (${aimed.join('; ')})`).toContain('wheel=true');
        await expect
          .poll(async () => (await state(page, key)).hand, { timeout: 15_000, message: 'the page heard the wheel, the hand did not pass' })
          .toBe('user');
        await expect(band.getByTestId('drive-hand')).toContainText('Tu as la main');
        const refused = await agent.call('drive_browser', { browser: key, action: 'highlight', target: '#tarifs', label: 'Tarifs' });
        expect(refused.error).toBe(true);
        expect(refused.text).toContain('L’utilisateur a repris la main');
        // « La rendre »: told to the agent, it drives again.
        await band.getByRole('button', { name: 'La rendre' }).click();
        await expect(page.getByText(/l’utilisateur te rend la main/).first()).toBeVisible({ timeout: 20_000 });
        expect(await agent.drive({ browser: key, action: 'scroll_to', target: '#tarifs' })).toMatchObject({ done: true, hand: 'agent' });

        // 7. Another origin: asked in the conversation, nothing opened before the answer; allowed once, it opens.
        const asked = await agent.json('open_browser', { url: `${OTHER}/docs` });
        expect(asked).toMatchObject({ place: 'asked', origin: OTHER });
        const askBand = page.getByTestId('open-ask-band');
        await expect(askBand).toContainText('veut ouvrir docs.escouade-e2e.invalid');
        expect(((await agent.json('list_browsers', {})).browsers as { origin: string }[]).some((b) => b.origin === OTHER)).toBe(false);
        await askBand.getByRole('button', { name: 'Permettre une fois' }).click();
        await expect(askBand).toBeHidden();
        await expect(page.getByText(/l’utilisateur te permet d’ouvrir/).first()).toBeVisible({ timeout: 20_000 });
        const otherRow = ((await agent.json('list_browsers', {})).browsers as { browser: string; origin: string; by: string }[]).find(
          (b) => b.origin === OTHER,
        );
        expect(otherRow).toMatchObject({ by: 'agent' });
        expect(await agent.json('close_browser', { browser: otherRow!.browser })).toMatchObject({ closed: true });
        // Its launch's tab cannot be sent there either: open_browser asks it.
        const away = await agent.call('drive_browser', { browser: key, action: 'navigate', url: `${OTHER}/docs` });
        expect(away.error).toBe(true);
        expect(away.text).toContain('open_browser');

        // 8. Outside the launch, its artifact: opened without asking (it published it), beside its conversation.
        await send(page, 'Publie le plan [artefact v1]');
        await expect(page.getByText('Version 1 publiée.')).toBeVisible({ timeout: 30_000 });
        const artOpened = await agent.json('open_browser', { artifact: 'last', place: 'beside' });
        expect(artOpened).toMatchObject({ place: 'beside', url: ARTIFACT });
        const artKey = artOpened.browser as string;
        await expect
          .poll(
            async () => {
              const b = (await agent.json('list_browsers', {})).browsers.find((r: { browser: string }) => r.browser === artKey);
              return `${b?.placement} ${b?.loaded}`;
            },
            { timeout: 30_000 },
          )
          .toBe('beside true');
        await expect(page.getByTestId('open-ask-band')).toBeHidden();
        // To show, without asking: to scroll, to highlight (found or not, nothing of the page), the phone's width.
        const shown = [
          await agent.drive({ browser: artKey, action: 'scroll', by: 600 }),
          await agent.drive({ browser: artKey, action: 'scroll_to', target: '#risques' }),
          await agent.drive({ browser: artKey, action: 'highlight', target: '#risques', label: 'Les risques' }),
          await agent.drive({ browser: artKey, action: 'width', width: 'mobile' }),
        ];
        for (const s of shown) expect(s).toMatchObject({ done: true, hand: 'agent' });
        for (const leak of ['Ada', 'Plan de la 2.4', 'Valider', 'Risques']) expect(JSON.stringify(shown)).not.toContain(leak);
        const missing = await agent.call('drive_browser', { browser: artKey, action: 'highlight', target: '#absent', label: 'Rien' });
        expect(missing.error).toBe(true);
        expect(missing.text).not.toContain('Ada');
        await expect(page.getByTestId('consent-band')).toBeHidden();

        // To act waits for the leave to read and act, asked in the band; nothing is clicked before.
        const click = await agent.json('drive_browser', { browser: artKey, action: 'click', target: '#ok' });
        expect(click).toMatchObject({ asked: true, origin: ORIGIN });
        const consent = page.getByTestId('consent-band');
        await expect(consent).toContainText('veut lire');
        await page.waitForTimeout(500);
        expect(art.heard).not.toContain('artifact-click=true');
        await consent.getByRole('button', { name: 'Permettre' }).click();
        await expect(consent).toBeHidden();
        await expect
          .poll(async () => (await agent.call('drive_browser', { browser: artKey, action: 'click', target: '#ok' })).error)
          .toBe(false);
        await expect.poll(() => art.heard).toContain('artifact-click=true');
        // A password, outside the launch: refused, even with that leave, and nothing asked.
        const artPw = await agent.call('drive_browser', { browser: artKey, action: 'type', target: '#pw', text: 'motdepasse-e2e' });
        expect(artPw.error).toBe(true);
        expect(artPw.text).toContain('mot de passe');
        await expect(page.getByTestId('password-band')).toBeHidden();

        // 9. The end: its launch's driving closes, the row stays an ordinary one.
        expect(await agent.json('close_browser', { browser: key })).toMatchObject({ closed: false });
        await expect(row(page, key)).toBeVisible();
        await expect(row(page, key)).not.toHaveAttribute('data-driven', 'agent');
      } finally {
        launch.server.close();
        art.server.close();
      }
    });
  }
});
