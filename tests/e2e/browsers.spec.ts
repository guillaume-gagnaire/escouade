// The browsers of a project (ESC-657): « Navigateurs » in the sidebar, under « Terminaux », one row per tab. A browser
// opened alone by its « + » at an address of a dev server: its row, its page shown alone in the main area (its bar: the
// address, the widths and the devtools beside it), then in a pane beside the agent's conversation, at a phone's width
// (the page sees it), closed by its ×. ESCOUADE_E2E_SHOTS=<folder>: captures of the window as the screen shows it (the
// page is a native webview over the window: Playwright's captures would leave its room empty), light and dark.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, unfoldSections, windowScale } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

/**
 * The app's window as Windows draws it, native webviews included (`PrintWindow`, with their content), into `file`: the
 * largest visible window of the test's program (never the user's own Escouade: another program's path).
 */
/** A string for PowerShell, single-quoted: taken as it is. */
const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

function capture(file: string, title = '') {
  const script = `
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class Shot {
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  public static IntPtr Titled(string start) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      var s = new System.Text.StringBuilder(512);
      if (IsWindowVisible(h) && GetWindowText(h, s, 512) > 0 && s.ToString().StartsWith(start)) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
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
}
"@
[void][Shot]::SetProcessDPIAware()
$exe = ${ps(path.resolve(EXE))}
$pids = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -ieq $exe } | ForEach-Object { [uint32]$_.ProcessId })
$title = ${ps(title)}
$h = if ($title) { [Shot]::Titled($title) } else { [Shot]::Largest($pids) }
if ($h -eq [IntPtr]::Zero) { throw ('no window among ' + ($pids -join ',') + ' for ' + $exe) }
$r = New-Object Shot+RECT
[void][Shot]::GetWindowRect($h, [ref]$r)
$b = New-Object System.Drawing.Bitmap ($r.Right - $r.Left), ($r.Bottom - $r.Top)
$g = [System.Drawing.Graphics]::FromImage($b)
$dc = $g.GetHdc()
[void][Shot]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$b.Save(${ps(file)})
`;
  execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' });
}

/** The theme of the app, as « Thème » sets it. */
const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** Captures of the window in both themes, named `<name>-<width>-<theme>.png`. */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.mouse.move(2, 2);
  const width = page.viewportSize()?.width ?? (await page.evaluate(() => innerWidth));
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await page.waitForTimeout(400);
    capture(path.join(SHOTS, `${name}-${width}-${t}.png`));
  }
  await theme(page, 'dark');
}

/** The keys of the panes shown, in order. */
const keys = (page: Page) => page.getByTestId('pane').evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.paneKey));

test('a browser opened alone: its row, shown alone, then beside the conversation at a phone’s width, then closed', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: what the page sees of its width is read on Windows');
  test.setTimeout(180_000);
  const { page } = app;

  // The dev server of the test: a page with a title, that says its width at each change.
  const widths: number[] = [];
  const server = http.createServer((req, res) => {
    if (req.url?.startsWith('/w?')) {
      widths.push(Number(new URL(req.url, 'http://x').searchParams.get('w')));
      return res.end('ok');
    }
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(`<!doctype html><title>Tableau — demo</title><body style="font:16px system-ui;margin:24px;background:#f6f7f9">
<h1>Tableau</h1><p>Les ventes de la semaine.</p>
<script>const say = () => fetch('/w?w=' + innerWidth); addEventListener('resize', say); say();</script>`);
  });
  await new Promise<void>((ok) => server.listen(0, 'localhost', ok));
  const port = (server.address() as AddressInfo).port;
  try {
    await addProject(page, app.repo);
    await unfoldSections(page);
    const section = page.getByRole('region', { name: 'Navigateurs' });
    await expect(section.getByText('Aucun navigateur ouvert')).toBeVisible();

    // « + »: an address, without its scheme.
    await section.getByRole('button', { name: 'Nouveau navigateur' }).click();
    const field = section.getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
    await expect(field).toBeFocused();
    await field.fill(`localhost:${port}/tableau`);
    await field.press('Enter');

    // Its row, named by its page's title once loaded, selected; its page alone in the main area.
    const row = section.locator('.row-item.browser');
    await expect(row).toHaveCount(1);
    await expect(row).toHaveAttribute('aria-current', 'true');
    await expect(row.locator('.nm')).toHaveText('Tableau — demo', { timeout: 30_000 });
    await expect(row.locator('.d')).toHaveClass(/\bon\b/);
    const address = page.getByRole('textbox', { name: 'Adresse de la page' });
    await expect(address).toHaveValue(`http://localhost:${port}/tableau`);
    await expect(page.getByTestId('pane')).toHaveCount(0);
    // Its bar: the widths and the devtools beside the address, then the system's browser; no agent's window.
    await expect(page.getByRole('button', { name: /Outils de développement de cette page/ })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Ouvrir dans une fenêtre à part' })).toHaveCount(0);
    const scale = await windowScale(page);
    await expect.poll(() => widths.length, { timeout: 30_000 }).toBeGreaterThan(0);
    const full = widths.at(-1)!;
    expect(full).toBeGreaterThan(390 * scale);
    await shots(page, 'navigateur-plein');

    // Half the screen: the agent's conversation, then the row Ctrl+clicked beside it.
    await page.locator('.side .agent').first().click();
    await expect(page.locator('.work .devpage')).toHaveCount(0);
    await expect(row).not.toHaveAttribute('aria-current', 'true');
    await row.click({ modifiers: ['Control'] });
    await expect.poll(() => keys(page)).toEqual([expect.stringMatching(/^agent:/), expect.stringMatching(/^page:browser:/)]);
    const pane = page.locator('[data-pane-key^="page:browser:"]');
    await expect(pane.getByRole('textbox', { name: 'Adresse de la page' })).toHaveValue(`http://localhost:${port}/tableau`);
    // The page sees its pane's width, narrower than the whole area.
    await expect.poll(() => widths.at(-1)!, { timeout: 15_000 }).toBeLessThan(full - 50);

    // A phone's width: in its bar, or in its « … » when the pane is narrow.
    const radio = pane.getByRole('radio', { name: 'Mobile' });
    if (await radio.isVisible()) await radio.click();
    else {
      await pane.getByRole('button', { name: 'Plus d’actions sur la page' }).click();
      await page.getByRole('menuitemradio', { name: /^Mobile/ }).click();
    }
    const tolerance = scale === 1 ? 0 : 1;
    await expect.poll(() => widths.at(-1)!, { timeout: 15_000 }).toBeLessThanOrEqual(Math.round(390 * scale) + tolerance);
    await expect(pane.getByText(/^(390 px|Rétréci à \d+ px)$/)).toBeVisible();
    await shots(page, 'navigateur-demi');

    // The devtools, from its bar: the page's alone, in a window of their own under WebView2 (said in the view).
    const devtools = pane.getByRole('button', { name: /Outils de développement de cette page/ });
    const described = await devtools.getAttribute('aria-describedby');
    await expect(page.locator(`[id="${described}"]`)).toHaveText('Dans leur propre fenêtre : WebView2 ne les loge pas dans la page.');

    // Closed by its ×: its pane goes, the conversation fills the area again, the row is gone.
    await row.hover();
    await row.getByRole('button', { name: 'Fermer le navigateur' }).click();
    await expect(page.getByTestId('pane')).toHaveCount(0);
    await expect(row).toHaveCount(0);
    await expect(section.getByText('Aucun navigateur ouvert')).toBeVisible();
  } finally {
    server.close();
  }
});

test('an address the browsers refuse opens nothing', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await unfoldSections(page);
  const section = page.getByRole('region', { name: 'Navigateurs' });
  await section.getByRole('button', { name: 'Nouveau navigateur' }).click();
  const field = section.getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
  for (const typed of ['file:///C:/Windows/win.ini', 'javascript:alert(1)']) {
    await field.fill(typed);
    await field.press('Enter');
    await expect(page.getByText('Seule une adresse http:// ou https:// s’ouvre dans un navigateur d’Escouade.').first()).toBeVisible();
  }
  await expect(section.locator('.row-item.browser')).toHaveCount(0);
  // Escouade's own address: refused by the backend too.
  await field.fill('http://tauri.localhost/');
  await field.press('Enter');
  await expect(section.locator('.row-item.browser')).toHaveCount(0);
  await field.press('Escape');
  await expect(field).toHaveCount(0);
});

test('captures of the browsers, light and dark (ESCOUADE_E2E_SHOTS)', async ({ app }) => {
  test.skip(!SHOTS || process.platform !== 'win32', 'captures on demand, of the window as Windows draws it');
  test.setTimeout(240_000);
  const { page } = app;
  const style = 'font:15px/1.5 system-ui;margin:0;color:#1d2430;background:#f6f7f9';
  const header = (t: string) =>
    `<header style="display:flex;gap:18px;align-items:center;padding:14px 28px;background:#fff;border-bottom:1px solid #e3e6ea"><b>Demo</b><span>${t}</span></header>`;
  const server = http.createServer((req, res) => {
    // A page that never answers: its row says so after 20 s.
    if (req.url?.startsWith('/lent')) return;
    const shop = req.url?.startsWith('/boutique');
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(
      `<!doctype html><meta name="viewport" content="width=device-width"><title>${shop ? 'Boutique' : 'Tableau'} — demo</title><body style="${style}">` +
        header(shop ? 'Boutique' : 'Tableau de bord') +
        `<main style="padding:24px 28px;display:grid;gap:16px;grid-template-columns:repeat(auto-fit,minmax(220px,1fr))">` +
        ['Ventes', 'Commandes', 'Paniers', 'Retours']
          .map(
            (x, i) =>
              `<section style="background:#fff;border:1px solid #e3e6ea;border-radius:10px;padding:16px"><div style="color:#5b6472">${x}</div><div style="font-size:26px;font-weight:650">${[1284, 312, 57, 9][i]}</div></section>`,
          )
          .join('') +
        '</main>',
    );
  });
  await new Promise<void>((ok) => server.listen(0, 'localhost', ok));
  const port = (server.address() as AddressInfo).port;
  try {
    await addProject(page, app.repo);
    await unfoldSections(page);
    const section = page.getByRole('region', { name: 'Navigateurs' });
    const open = async (address: string) => {
      await section.getByRole('button', { name: 'Nouveau navigateur' }).click();
      const field = section.getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
      await field.fill(address);
      await field.press('Enter');
      await expect(field).toHaveCount(0);
    };
    await open(`localhost:${port}/lent`);
    await open(`localhost:${port}/boutique`);
    await open(`localhost:${port}/tableau`);
    const rows = section.locator('.row-item.browser');
    await expect(rows.nth(2).locator('.nm')).toHaveText('Tableau — demo', { timeout: 30_000 });
    await expect(rows.nth(1).locator('.nm')).toHaveText('Boutique — demo');
    await expect(rows.nth(0).locator('.d')).toHaveClass(/\bbad\b/, { timeout: 30_000 });
    // Alone, the whole area.
    await rows.nth(2).click();
    await page.waitForTimeout(800);
    await shots(page, 'navigateurs-plein');
    // Half the screen: beside the agent's conversation, at a tablet's width.
    await page.locator('.side .agent').first().click();
    await rows.nth(1).click({ modifiers: ['Control'] });
    const pane = page.locator('[data-pane-key^="page:browser:"]');
    await expect(pane).toBeVisible();
    const radio = pane.getByRole('radio', { name: 'Tablette' });
    if (await radio.isVisible()) await radio.click();
    await page.waitForTimeout(800);
    await shots(page, 'navigateurs-demi');
    // The row that does not answer, shown: what its view says.
    await page.keyboard.press('Control+Shift+W');
    await rows.nth(0).click();
    await page.waitForTimeout(500);
    await shots(page, 'navigateurs-sans-reponse');
    // Its devtools, from its bar: a window of their own under WebView2.
    await rows.nth(2).click();
    await page.getByRole('button', { name: /Outils de développement de cette page/ }).click();
    await page.waitForTimeout(4000);
    capture(path.join(SHOTS!, 'navigateurs-devtools-fenetre.png'), 'DevTools');
  } finally {
    server.close();
  }
});
