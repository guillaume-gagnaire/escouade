// The view of the agents' pages (ESC-675, AP-3; docs/AGENT-PAGES.md § 4.4, § 5 and § 12, direction A), by a fake agent
// that calls Escouade's real MCP server (tests/fixtures/fake-claude.mjs, « [page …] »): it shows a review while the user
// looks at its conversation (the page opens beside it, its row page:p-1 in « Navigateurs », its card in the conversation,
// the trust mention above it), updates it (reloaded where it is, its card « Mise à jour », v2), the user goes back to v1
// and to the last, shows it at a phone's width, the agent leaves a diagram that waits (open: quiet), then closes the
// review (its row and its webview gone, the page kept: « Rouvrir » opens it again beside the conversation), and
// « Pages des agents » off says so on the cards. At 1440 × 900 and 1024 × 728.
// ESCOUADE_E2E_SHOTS=<folder>: captures of the window as Windows draws it (the page is a native webview over the window),
// in French and in English, dark and light, at both sizes.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, send, test, unfoldSections, windowScale, type App } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

const SIZES = [
  { width: 1440, height: 900 },
  { width: 1024, height: 728 },
];

const WORDS = {
  fr: {
    versions: 'Versions de la page',
    v1: 'Version 1',
    mobile: 'Mobile',
    full: 'Pleine largeur',
    more: 'Plus d’actions sur la page',
    reopen: 'Rouvrir',
    browsers: 'Navigateurs',
    trust: 'hors réseau, sans accès à Escouade ni à tes fichiers',
    updated: 'Mise à jour',
    older: 'la dernière est la v2',
    toLatest: 'Revenir à la v2',
    off: 'Les pages des agents sont coupées',
  },
  en: {
    versions: 'Versions of the page',
    v1: 'Version 1',
    mobile: 'Mobile',
    full: 'Full width',
    more: 'More actions on the page',
    reopen: 'Reopen',
    browsers: 'Browsers',
    trust: 'offline, no access to Escouade or your files',
    updated: 'Updated',
    older: 'the latest is v2',
    toLatest: 'Back to v2',
    off: 'Agents’ pages are off',
  },
} as const;

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;

async function invoke<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

/** The text of page `p` in its shelter, read in Escouade's world, once it says `wanted` (or no longer `gone`). */
async function pageText(page: Page, p: string, wanted: string, gone?: string) {
  await expect
    .poll(
      async () => {
        const text = await invoke<string>(page, 'agentpage_read', { page: p, selector: null, text: true }).catch((e) => String(e));
        return text.includes(wanted) && (!gone || !text.includes(gone));
      },
      { timeout: 30_000 },
    )
    .toBe(true);
}

/** A string for PowerShell, single-quoted: taken as it is. */
const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

/**
 * The app's window as Windows draws it, native webviews included (`PrintWindow`), into `file`, cut to the viewport the
 * test emulates (`width` × `height` CSS pixels at `scale`, from the window's top left: the window has no frame of its own).
 */
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
  await page.waitForTimeout(500);
  capture(path.join(SHOTS, `${name}-${lang}-${theme}-${size.width}.png`), size.width, size.height, await windowScale(page));
}

/** The width `name` chosen in the bar, or in « … » when the bar folded the widths; checked after. */
async function width(page: Page, view: Locator, name: string, more: string) {
  const radio = view.getByRole('radio', { name });
  if (await radio.isVisible()) {
    await radio.click();
    await expect(radio).toHaveAttribute('aria-checked', 'true');
    return;
  }
  await view.getByRole('button', { name: more }).click();
  await page.getByRole('menuitemradio', { name }).click();
  await view.getByRole('button', { name: more }).click();
  await expect(page.getByRole('menuitemradio', { name })).toHaveAttribute('aria-checked', 'true');
  await page.keyboard.press('Escape');
}

/** The keys of the panes shown, in order. */
const panes = (page: Page) => page.getByTestId('pane').evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.paneKey));

const card = (page: Page, n: number) => page.locator('[data-page-card="p-1"]').nth(n);

async function scenario(app: App, size: { width: number; height: number }, lang: 'fr' | 'en', theme: 'dark' | 'light') {
  const w = WORDS[lang];
  const { page } = app;
  await page.setViewportSize(size);
  await addProject(page, app.repo, { lang });
  await unfoldSections(page);
  const browsers = page.getByRole('region', { name: w.browsers });

  // Shown while the user looks at the conversation: beside it, which stays active; its row, its card, its trust band.
  await send(page, 'Montre-moi la revue avant fusion [page montre]');
  await expect(card(page, 0)).toHaveAttribute('data-state', 'beside', { timeout: 30_000 });
  const view = page.locator('[data-agent-page="p-1"]');
  await expect(view).toBeVisible();
  const keys = await panes(page);
  expect(keys).toHaveLength(2);
  expect(keys[1]).toBe('page:page:p-1');
  expect(keys[0]).toMatch(/^agent:/);
  const row = browsers.locator('[data-browser="page:p-1"]');
  await expect(row).toBeVisible();
  await expect(row.getByTestId('page-row-version')).toHaveText('v1');
  await expect(view.getByTestId('drive-band')).toContainText(w.trust);
  await expect(view.getByRole('button', { name: w.versions })).toHaveText('v1');
  await pageText(page, 'p-1', 'La liste montre 25 adhérents par page');
  await expect.poll(async () => (await invoke<{ loaded: boolean } | null>(page, 'agentpage_view', { page: 'p-1' }))?.loaded).toBe(true);
  await shot(page, 'montree', lang, theme, size);

  // A new version: reloaded where it is; the new card says « Mise à jour », the first one that a v2 came since.
  await send(page, 'Le tri est gardé ? [page maj p-1]');
  await expect(card(page, 1)).toHaveAttribute('data-state', 'beside', { timeout: 30_000 });
  await expect(card(page, 1)).toContainText(w.updated);
  await expect(card(page, 0)).toContainText('v2');
  await expect(row.getByTestId('page-row-version')).toHaveText('v2');
  await expect(view.getByRole('button', { name: w.versions })).toHaveText('v2');
  expect(await panes(page)).toEqual(keys);
  await pageText(page, 'p-1', 'La page garde le filtre', 'pas gardé dans l’adresse');
  await shot(page, 'mise-a-jour', lang, theme, size);

  // « v2 ▾ »: back to v1 (its risk again), said above the page; then to the last.
  await view.getByRole('button', { name: w.versions }).click();
  await page
    .getByRole('menuitemradio', { name: new RegExp(w.v1) })
    .or(page.getByRole('menuitem', { name: new RegExp(w.v1) }))
    .first()
    .click();
  await expect(view.getByTestId('page-older')).toContainText(w.older);
  await pageText(page, 'p-1', 'pas gardé dans l’adresse');
  await shot(page, 'version-ancienne', lang, theme, size);
  await view.getByRole('button', { name: w.toLatest }).click();
  await expect(view.getByTestId('page-older')).toHaveCount(0);
  await pageText(page, 'p-1', 'La page garde le filtre', 'pas gardé dans l’adresse');

  // A phone's width, then the whole room again: by its bar, or by « … » when the pane is too narrow for them (1024).
  await width(page, view, w.mobile, w.more);
  await shot(page, 'mobile', lang, theme, size);
  await width(page, view, w.full, w.more);

  // A diagram that waits (open: quiet): its row with its point, its card « en attente », nothing moved on screen.
  await send(page, 'Et le schéma du tri ? [page calme]');
  const quiet = page.locator('[data-page-card="p-2"]');
  await expect(quiet).toHaveAttribute('data-state', 'waiting', { timeout: 30_000 });
  const waiting = browsers.locator('[data-browser="page:p-2"]');
  await expect(waiting.locator('.pg.unseen')).toHaveCount(1);
  expect(await panes(page)).toEqual(keys);
  await shot(page, 'en-attente', lang, theme, size);

  // Closed by its agent: its row and its webview gone, its cards « fermée »; « Rouvrir » opens it again beside.
  await send(page, 'Ferme la revue [page ferme p-1]');
  await expect(card(page, 0)).toHaveAttribute('data-state', 'closed', { timeout: 30_000 });
  await expect(card(page, 1)).toHaveAttribute('data-state', 'closed');
  await expect(row).toHaveCount(0);
  await expect.poll(() => invoke(page, 'agentpage_view', { page: 'p-1' }), { timeout: 10_000 }).toBeNull();
  await card(page, 1).scrollIntoViewIfNeeded();
  await shot(page, 'fermee', lang, theme, size);
  await card(page, 1).getByRole('button', { name: w.reopen }).click();
  await expect(card(page, 1)).toHaveAttribute('data-state', 'beside', { timeout: 20_000 });
  await expect(row).toBeVisible();
  await expect(view).toBeVisible();
  await pageText(page, 'p-1', 'La page garde le filtre');
}

for (const lang of ['fr', 'en'] as const) {
  for (const theme of ['dark', 'light'] as const) {
    test.describe(`the view of the agents' pages (${lang}, ${theme})`, () => {
      test.use({ language: lang, preStart: { run: () => ({ mcpEnabled: true, theme }) } });
      // French and dark always run; the other languages and themes are for the captures.
      const always = lang === 'fr' && theme === 'dark';

      for (const size of SIZES) {
        test(`a fake agent shows its page beside its conversation, updates and closes it; its card and its row (${size.width} × ${size.height})`, async ({
          app,
        }) => {
          test.skip(process.platform !== 'win32', 'the pages are read through WebView2 here (WebKit by the probe)');
          test.skip(!always && !SHOTS, 'other languages and themes: captures on demand (ESCOUADE_E2E_SHOTS)');
          test.setTimeout(300_000);
          await scenario(app, size, lang, theme);
          if (!always) return;
          // « Pages des agents » off: the rows close, the cards say where they turn back on.
          const dialog = await openSettingsPage(app.page, 'Agents');
          const sw = dialog.getByRole('switch', { name: 'Pages des agents' });
          await sw.click();
          await expect(sw).toHaveAttribute('aria-checked', 'false');
          await closeSettings(dialog);
          await expect(card(app.page, 0)).toHaveAttribute('data-state', 'off');
          await expect(card(app.page, 0)).toContainText(WORDS.fr.off);
          await expect(app.page.locator('[data-browser^="page:"]')).toHaveCount(0);
        });
      }
    });
  }
}
