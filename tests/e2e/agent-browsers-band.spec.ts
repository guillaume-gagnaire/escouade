// What the user sees of the driving (ESC-680, AB-8 with AP-6; docs/AGENT-BROWSERS.md § 4, § 9, docs/AGENT-PAGES.md § 12,
// direction A), by a fake agent that calls Escouade's real MCP server (tests/fixtures/fake-claude.mjs, « [page montre] »
// then « [pilote p-1] »): it shows its review beside its conversation, then drives it under the user's eyes — scrolls
// to the criteria, highlights the third, scrolls to the changes. Each gesture is a numbered row in the conversation and
// a pin on the page's rail; the band says « pagination pilote » with the last gesture's number and caption; its row in
// « Navigateurs » wears the agent's initials. Under Windows the highlight and the ghost cursor are the browser's
// overlay: nothing of them is in the page's DOM. The user takes the hand from the band: the agent's next gesture is
// refused by the core (its row of a tool says so); « La rendre » lets it go on; « Continuer » ends its wait_user; a pin
// takes the page back where the agent had put it. At 1440 × 900 and 1024 × 728.
// ESCOUADE_E2E_SHOTS=<folder>: captures of the window as Windows draws it, in French and in English, dark and light.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test, unfoldSections, windowScale } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

const WORDS = {
  fr: {
    browsers: 'Navigateurs',
    drives: 'pilote',
    take: 'Reprendre la main',
    youHave: 'Tu as la main',
    give: 'La rendre',
    cont: 'Continuer',
    rail: /Repères de/,
    driven: 'Piloté par',
    refused: 'L’utilisateur a repris la main',
    given: 'te rend la main',
    highlight: 'Met en évidence',
  },
  en: {
    browsers: 'Browsers',
    drives: 'is driving',
    take: 'Take over',
    youHave: 'You have control',
    give: 'Hand it back',
    cont: 'Continue',
    rail: /marks$/,
    driven: 'Driven by',
    refused: 'The user took the hand back',
    given: 'gives you the hand back',
    highlight: 'Highlights',
  },
} as const;

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;

async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

type Drive = { hand: string; waiting: boolean; agent: string | null; step: number; pins: { n: number; y: number }[] };
const state = (page: Page) => invoke<Drive>(page, 'browser_drive_state', { key: 'page:p-1' });
const where = (page: Page) => invoke<{ y: number; height: number; view: number } | null>(page, 'browser_drive_place', { key: 'page:p-1' });

/** A string for PowerShell, single-quoted: taken as it is. */
const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

/** The app's window as Windows draws it, native webviews included (`PrintWindow`), cut to the viewport. */
function capture(file: string, width: number, height: number, scale: number) {
  const script = `
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Shot {
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
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
$h = [Shot]::Largest($pids)
if ($h -eq [IntPtr]::Zero) { throw ('no window for ' + $exe) }
$r = New-Object Shot+RECT
[void][Shot]::GetWindowRect($h, [ref]$r)
$b = New-Object System.Drawing.Bitmap ($r.Right - $r.Left), ($r.Bottom - $r.Top)
$g = [System.Drawing.Graphics]::FromImage($b)
$dc = $g.GetHdc()
[void][Shot]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$w = [Math]::Min($b.Width, ${Math.round(width * scale)})
$hh = [Math]::Min($b.Height, ${Math.round(height * scale)})
$c = $b.Clone((New-Object System.Drawing.Rectangle 0, 0, $w, $hh), $b.PixelFormat)
$c.Save(${ps(file)})
`;
  execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' });
}

/** A capture of the window as it is now, when the run asks for them: `<state>-<lang>-<theme>-<width>.png`. */
async function shot(page: Page, name: string, lang: string, theme: string, size: { width: number; height: number }) {
  if (!SHOTS || process.platform !== 'win32') return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await page.mouse.move(2, 2);
  await page.waitForTimeout(400);
  capture(path.join(SHOTS, `${name}-${lang}-${theme}-${size.width}.png`), size.width, size.height, await windowScale(page));
}

async function scenario(page: Page, repo: string, size: { width: number; height: number }, lang: 'fr' | 'en', theme: string) {
  const w = WORDS[lang];
  await page.setViewportSize(size);
  await addProject(page, repo, { lang });
  await unfoldSections(page);
  const browsers = page.getByRole('region', { name: w.browsers });

  // Its review, beside the conversation, loaded.
  await send(page, 'Montre-moi la revue avant fusion [page montre]');
  const view = page.locator('[data-agent-page="p-1"]');
  await expect(view).toBeVisible({ timeout: 30_000 });
  await expect
    .poll(async () => (await invoke<{ loaded: boolean } | null>(page, 'agentpage_view', { page: 'p-1' }))?.loaded, { timeout: 30_000 })
    .toBe(true);
  // Nobody drives it yet: the trust mention alone, no rail, the row without initials.
  await expect(view.getByTestId('drive-hand')).toHaveCount(0);
  await expect(view.getByTestId('drive-rail')).toHaveCount(0);

  // It drives its page: three gestures, three numbered rows, three pins, the band and the row say who drives.
  await send(page, 'Montre-moi le critère qui coince [pilote p-1]');
  const rows = page.getByTestId('drive-row');
  await expect(rows).toHaveCount(3, { timeout: 30_000 });
  await expect(rows.nth(2)).toHaveAttribute('data-step', '3', { timeout: 30_000 });
  expect(await rows.evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.step))).toEqual(['1', '2', '3']);
  await expect(rows.nth(1)).toContainText(w.highlight);
  const band = view.getByTestId('drive-band');
  await expect(band.getByTestId('drive-hand')).toContainText(w.drives);
  await expect(band.getByTestId('drive-last')).toContainText('3');
  const rail = view.getByRole('navigation', { name: w.rail });
  await expect(rail.locator('[data-pin]')).toHaveCount(3);
  await expect(rail.locator('[data-pin="3"]')).toHaveClass(/cur/);
  await expect(view.getByTestId('drive-rail-view')).toBeVisible({ timeout: 10_000 });
  const row = browsers.locator('[data-browser="page:p-1"]');
  await expect(row).toHaveAttribute('data-driven', 'agent');
  await expect(row.getByTestId('driven-avatar')).toBeVisible();
  await expect(row).toContainText(w.driven);
  // The highlight and the ghost cursor are not in the page's DOM: the browser's overlay draws them (Windows).
  const html = await invoke<string>(page, 'agentpage_read', { page: 'p-1', selector: null, text: false });
  expect(html).not.toContain('escouade-drive-highlight');
  expect(html).not.toContain('escouade-drive-cursor');
  expect(html).not.toContain('data-escouade');
  // The third gesture left the page at the changes: the core says where it is, the rail draws it.
  const pins = (await state(page)).pins;
  expect(pins.map((p) => p.n)).toEqual([1, 2, 3]);
  await shot(page, 'pilotee', lang, theme, size);

  // « Reprendre la main » in the band: the core's; the agent's next gesture is refused, nothing done.
  await band.getByRole('button', { name: w.take }).click();
  await expect(band.getByTestId('drive-hand')).toContainText(w.youHave);
  await expect.poll(async () => (await state(page)).hand).toBe('user');
  await expect(row).toHaveAttribute('data-driven', 'user');
  const before = (await where(page))?.y;
  await send(page, 'Continue [pilote p-1 encore]');
  // Its call refused: a row of a tool in error, not a numbered one; the agent says it waits.
  await expect(page.locator('main.conv').getByText('j’attends que tu me la rendes').first()).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('main.conv .tool.err')).toHaveCount(1);
  await page.locator('main.conv .tool.err .row').first().click();
  await expect(page.locator('main.conv .tool.err')).toContainText(w.refused);
  await expect(rows).toHaveCount(3);
  expect((await state(page)).step).toBe(3);
  expect((await where(page))?.y).toBe(before);
  await shot(page, 'main-reprise', lang, theme, size);

  // « La rendre »: the agent is told, and goes on: a fourth row and pin.
  await band.getByRole('button', { name: w.give }).click();
  await expect(band.getByTestId('drive-hand')).toContainText(w.drives);
  await expect(page.locator('main.conv').getByText(w.given).first()).toBeVisible({ timeout: 30_000 });
  await send(page, 'Vas-y [pilote p-1 encore]');
  await expect(rows).toHaveCount(4, { timeout: 30_000 });
  await expect(rows.nth(3)).toHaveAttribute('data-step', '4', { timeout: 30_000 });
  await expect(rail.locator('[data-pin]')).toHaveCount(4);

  // wait_user: « Continuer » in the band ends the agent's wait.
  await send(page, 'Regarde bien [pilote p-1 attends]');
  await expect(band.getByRole('button', { name: w.cont })).toBeVisible({ timeout: 30_000 });
  await shot(page, 'continuer', lang, theme, size);
  await band.getByRole('button', { name: w.cont }).click();
  await expect(band.getByRole('button', { name: w.cont })).toHaveCount(0, { timeout: 15_000 });
  await expect(rows).toHaveCount(5, { timeout: 30_000 });

  // A pin of the rail: the page goes back where the agent had put it.
  const second = (await state(page)).pins.find((p) => p.n === 2)!;
  await rail.locator('[data-pin="2"]').click();
  await expect.poll(async () => Math.abs(((await where(page))?.y ?? -999) - second.y), { timeout: 10_000 }).toBeLessThanOrEqual(2);
}

for (const lang of ['fr', 'en'] as const) {
  for (const theme of ['dark', 'light'] as const) {
    test.describe(`what the user sees of the driving (${lang}, ${theme})`, () => {
      test.use({ language: lang, preStart: { run: () => ({ mcpEnabled: true, theme }) } });
      // French and dark always run; the other languages and themes are for the captures.
      const always = lang === 'fr' && theme === 'dark';

      for (const size of SIZES) {
        test(`a fake agent drives its page under the user's eyes, who takes the hand and gives it back (${size.width} × ${size.height})`, async ({
          app,
        }) => {
          test.skip(process.platform !== 'win32', 'the overlay is WebView2’s (WebKit draws elements, by the probe)');
          test.skip(!always && !SHOTS, 'other languages and themes: captures on demand (ESCOUADE_E2E_SHOTS)');
          test.setTimeout(300_000);
          await scenario(app.page, app.repo, size, lang, theme);
        });
      }
    });
  }
}
