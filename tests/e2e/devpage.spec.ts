// A launch's page inside Escouade (ESC-311): « ▶ Tester » ready, the page of the developed app opens by itself in the
// agent's view, in a webview apart: no command of Escouade answers it, its storage is its project's, another origin
// goes to the system's browser; its devtools open from its bar.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

/** The titles of the top-level windows of the session (Windows): the devtools of WebView2 are one of their own. */
function windowTitles(): string[] {
  const script = `
Add-Type @"
using System; using System.Text; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class Wins {
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static string All() {
    var all = new List<string>();
    EnumWindows((h, l) => { var s = new StringBuilder(512); if (GetWindowText(h, s, 512) > 0) all.Add(s.ToString()); return true; }, IntPtr.Zero);
    return String.Join("\\n", all);
  }
}
"@
[Wins]::All()`;
  return execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' })
    .split(/\r?\n/)
    .filter(Boolean);
}

/** What the window asks to open outside, answered here: no browser opens (as links.spec.ts). */
async function watchOutside(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { esc311Opened: string[] };
    w.esc311Opened = [];
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('open_url')) {
        w.esc311Opened.push(JSON.parse(String(init?.body ?? '{}')).url);
        return Promise.resolve(new Response('null', { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }));
      }
      return fetch(input, init);
    };
  });
  return () => page.evaluate(() => (window as unknown as { esc311Opened: string[] }).esc311Opened);
}

test('a ready launch opens its page in the agent, apart from Escouade, and its devtools open from its bar', async ({ app }) => {
  test.skip(process.platform !== 'win32', 'WebView2: the devtools are a window of their own, counted by its title');
  const { page } = app;
  const report = path.join(app.data, 'rapport.json');
  const pwned = path.join(app.data, 'pwned');
  const widths = path.join(app.data, 'largeurs.txt');
  // The developed app: its page tries every way into Escouade, keeps something in its storage, says what it got, then
  // leaves for another origin (127.0.0.1 is not localhost).
  fs.writeFileSync(
    path.join(app.repo, 'serveur.js'),
    `const fs = require('node:fs');
const port = Number(process.env.ESCOUADE_PORT_BASE) + 1;
const PAGE = \`<!doctype html><title>demo</title><h1>Utilisateurs</h1><script>
// Its width, as the page sees it, at each change (what its media queries see).
addEventListener('resize', () => fetch('/width?w=' + innerWidth));
(async () => {
  const r = { tauri: typeof window.__TAURI__, ipc: typeof window.ipc };
  const tryCall = async (cmd, args) => {
    try {
      const call = window.__TAURI_INTERNALS__.invoke(cmd, args);
      const answer = await Promise.race([call, new Promise((_, no) => setTimeout(() => no(new Error('no answer')), 3000))]);
      return 'answered: ' + JSON.stringify(answer);
    } catch (e) { return 'refused: ' + String(e && e.message || e); }
  };
  r.app = await tryCall('open_url', { url: 'http://localhost:\${port}/pwned' });
  r.plugin = await tryCall('plugin:opener|open_url', { url: 'http://localhost:\${port}/pwned' });
  r.list = await tryCall('devpage_list', {});
  localStorage.setItem('mark', 'esc-311');
  r.storage = localStorage.getItem('mark');
  await fetch('/report', { method: 'POST', body: JSON.stringify(r) });
  setTimeout(() => { location.href = 'http://127.0.0.1:\${port}/ailleurs'; }, 300);
})();
</script>\`;
require('node:http').createServer((req, res) => {
  if (req.url === '/pwned') fs.writeFileSync(${JSON.stringify(pwned)}, 'x');
  if (req.url.startsWith('/width?w=')) fs.appendFileSync(${JSON.stringify(widths)}, req.url.slice(9) + '\\n');
  if (req.method === 'POST' && req.url === '/report') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => { fs.writeFileSync(${JSON.stringify(report)}, body); res.end('ok'); });
    return;
  }
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(PAGE);
}).listen(port, 'localhost');
`,
  );
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
  const opened = await watchOutside(page);

  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Page [ok] [recette]');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  await review.getByRole('button', { name: 'Tester' }).click();
  const dialog = page.getByRole('dialog', { name: 'Tester DEM-1' });
  await dialog.getByRole('button', { name: 'Lancer' }).click();

  // Ready: the test's window gives way to the agent's view « Page », on the address of the feature.
  await expect(dialog).toHaveCount(0, { timeout: 60_000 });
  await expect(page.getByRole('button', { name: 'La page de son lancement, dans Escouade' })).toHaveAttribute('aria-pressed', 'true');
  const address = page.getByRole('textbox', { name: 'Adresse de la page' });
  await expect(address).toHaveValue(/^http:\/\/localhost:\d+\/fonction$/);
  const port = Number(new URL(await address.inputValue()).port);

  // The page ran, apart: no command of Escouade answered it, neither the app's nor a plugin's, and its storage works.
  await expect.poll(() => fs.existsSync(report), { timeout: 30_000 }).toBe(true);
  const got = JSON.parse(fs.readFileSync(report, 'utf8'));
  expect(got.app).toMatch(/^refused: /);
  expect(got.plugin).toMatch(/^refused: /);
  expect(got.list).toMatch(/^refused: /);
  expect(got.tauri).toBe('undefined');
  expect(got.storage).toBe('esc-311');
  expect(fs.existsSync(pwned)).toBe(false);
  // Its cookies and storage are its project's folder, not the window's.
  const pages = path.join(app.data, 'pages');
  await expect.poll(() => fs.existsSync(pages) && fs.readdirSync(pages).length, { timeout: 15_000 }).toBe(1);

  // Another origin went to the system's browser, and the window says so; the page stayed on its own.
  await expect.poll(opened, { timeout: 15_000 }).toEqual([`http://127.0.0.1:${port}/ailleurs`]);
  await expect(page.getByText(`127.0.0.1:${port} s’est ouvert dans ton navigateur`)).toBeVisible();
  await expect(address).toHaveValue(`http://localhost:${port}/fonction`);

  // A phone's width: the page itself is 390 px wide, centred; a tablet's, 768; back to the whole room.
  const seen = () => (fs.existsSync(widths) ? fs.readFileSync(widths, 'utf8').trim().split('\n').at(-1) : null);
  // In the bar, or in « … » when the view is narrow (the CI's window, 1028 px wide).
  const choose = async (name: string) => {
    const radio = page.getByRole('radio', { name });
    if (await radio.isVisible()) return radio.click();
    await page.getByRole('button', { name: 'Plus d’actions sur la page' }).click();
    await page.getByRole('menuitemradio', { name: new RegExp(`^${name}`) }).click();
  };
  await choose('Mobile');
  await expect.poll(seen, { timeout: 15_000 }).toBe('390');
  await expect(page.getByText('390 px', { exact: true })).toBeVisible();
  // A tablet's, narrowed to the room when it is narrower (« Rétréci à 692 px » in the CI's window).
  await choose('Tablette');
  const said = page.getByText(/^(768 px|Rétréci à \d+ px)$/);
  await expect(said).toBeVisible();
  const tablet = (await said.textContent())!.match(/\d+/)![0];
  await expect.poll(seen, { timeout: 15_000 }).toBe(tablet);
  // The whole room again: wider than a tablet, or the room itself when it was narrowed to it.
  await choose('Pleine largeur');
  await expect.poll(() => Number(seen()), { timeout: 15_000 }).toBeGreaterThan(tablet === '768' ? 768 : Number(tablet) - 1);

  // Back to the conversation and to the page again: the same page, not loaded anew.
  await page.getByRole('button', { name: 'Vue Escouade' }).click();
  await page.getByRole('button', { name: 'La page de son lancement, dans Escouade' }).click();
  await expect(address).toHaveValue(`http://localhost:${port}/fonction`);

  // Its devtools, from its bar: a window of their own on this page, never on Escouade's. Last: on CI, where a
  // policy gives every WebView2 of the app the same debugging port, the page's devtools take Playwright's
  // connection to the window away; only the windows' titles are read after them.
  expect(windowTitles().filter((t) => t.startsWith('DevTools'))).toEqual([]);
  await page.getByRole('button', { name: /Outils de développement de cette page/ }).click();
  await expect
    .poll(() => windowTitles().filter((t) => t.startsWith('DevTools')), { timeout: 15_000 })
    .toEqual([expect.stringContaining(`localhost:${port}`)]);
});
