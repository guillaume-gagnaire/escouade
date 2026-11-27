// Signing in to a site in a tab of Escouade (ESC-695): a tab is a real browser. The developed app on one origin sends the
// tab to its identity provider on another (a redirection of the server, then the provider's script sends it back): the
// tab keeps it, its bar says where it is. The app opens the provider's sign-in window (`window.open`): it opens in
// Escouade, on the tab's session, sets its cookie, answers `window.opener` by `postMessage` and closes itself; the tab,
// sent to the provider by an address pasted in its bar, comes with that cookie. A few windows at a time; those left close
// with their tab. This machine at a port the project does not declare is never loaded, from the page as from the bar.
// The provider is on this machine too (no network in the tests), at a port a launch command of the project declares.
import { execFileSync } from 'node:child_process';
import http from 'node:http';
import type { AddressInfo } from 'node:net';
import path from 'node:path';
import { addProject, closeSettings, expect, test, unfoldSections } from './fixture';

const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

/** A string for PowerShell, single-quoted: taken as it is. */
const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

/** The titles of the windows of the test's program (never the user's own Escouade: another program's path). */
function titles(): string[] {
  const script = `
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class Wins {
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  public static string[] Of(uint[] pids) {
    var all = new List<string>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      var s = new System.Text.StringBuilder(512);
      if (Array.IndexOf(pids, pid) >= 0 && IsWindowVisible(h) && GetWindowText(h, s, 512) > 0) all.Add(s.ToString());
      return true;
    }, IntPtr.Zero);
    return all.ToArray();
  }
}
"@
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$exe = ${ps(path.resolve(EXE))}
$pids = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -ieq $exe } | ForEach-Object { [uint32]$_.ProcessId })
[Wins]::Of($pids) | ForEach-Object { $_ }
`;
  const out = execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' });
  return out.split(/\r?\n/).filter(Boolean);
}

/** A server of this machine that keeps what it was asked, with the cookies sent. */
function server(name: string, answer: (url: URL, res: http.ServerResponse, cookie: string) => void) {
  const hits: { path: string; query: URLSearchParams; cookie: string }[] = [];
  const s = http.createServer((req, res) => {
    const url = new URL(req.url ?? '/', 'http://x');
    const cookie = req.headers.cookie ?? '';
    hits.push({ path: url.pathname, query: url.searchParams, cookie });
    answer(url, res, cookie);
  });
  return { name, s, hits, saw: (p: string) => hits.filter((h) => h.path === p) };
}

const html = (res: http.ServerResponse, body: string, headers: Record<string, string> = {}) => {
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store', ...headers });
  res.end(`<!doctype html><body style="font:15px system-ui;margin:24px">${body}`);
};

test('a sign-in: the provider’s origin kept in the tab, its window in Escouade on the same session, the loopback kept out', async ({
  app,
}) => {
  test.skip(process.platform !== 'win32', 'WebView2: the windows of a page are seen on Windows');
  test.setTimeout(240_000);
  const { page } = app;

  let appOrigin = '';
  let idpOrigin = '';
  let closedOrigin = '';
  // The window a page opens, off every screen as the test's window is: it never takes the user's keyboard.
  const offScreen = 'left=-32000,top=-32000,width=480,height=600';
  // The answer the page of `/many` waits for to close its first window.
  let waiting: http.ServerResponse | undefined;

  // The developed app, on localhost.
  const site = server('app', (url, res) => {
    if (url.pathname === '/start')
      // Its server sends the tab to the provider, which will send it back.
      return res.writeHead(302, { Location: `${idpOrigin}/login?return=${encodeURIComponent(`${appOrigin}/home`)}` }).end();
    if (url.pathname === '/sneak')
      // A redirection to a service of this machine at a port the project does not declare.
      return res.writeHead(302, { Location: `${closedOrigin}/admin` }).end();
    if (url.pathname === '/home')
      return html(
        res,
        `<title>Accueil — appli</title><h1>Accueil</h1><p id="s">…</p><script>
addEventListener('message', (e) => {
  if (e.origin !== ${JSON.stringify(idpOrigin)}) return;
  fetch('/got?m=' + encodeURIComponent(e.data) + '&from=' + encodeURIComponent(e.origin));
});
const w = window.open(${JSON.stringify(`${idpOrigin}/popup`)}, 'connexion', ${JSON.stringify(offScreen)});
fetch('/opened?w=' + (w ? 1 : 0));
</script>`,
      );
    if (url.pathname === '/many')
      return html(
        res,
        `<title>Fenêtres — appli</title><script>
const ws = [];
for (let i = 0; i < 4; i++) ws.push(window.open(${JSON.stringify(`${idpOrigin}/stay?i=`)} + i, 'f' + i, ${JSON.stringify(offScreen)}));
// The opener closes the first one when the test says so.
fetch('/wait').then(() => ws[0].close());
</script>`,
      );
    if (url.pathname === '/wait') {
      waiting = res;
      return;
    }
    if (url.pathname === '/got' || url.pathname === '/opened') return res.end('ok');
    html(res, '<title>Appli — demo</title><h1>Appli</h1>');
  });
  // Its identity provider, on another origin of this machine.
  const idp = server('idp', (url, res, cookie) => {
    if (url.pathname === '/login')
      return html(
        res,
        `<title>Connexion — fournisseur</title><h1>Connexion</h1><script>
setTimeout(() => { location.href = ${JSON.stringify(`${appOrigin}/back`)}; }, 1500);
</script>`,
        { 'Set-Cookie': 'idp=vu; Path=/; SameSite=Lax' },
      );
    if (url.pathname === '/popup')
      return html(
        res,
        `<title>Se connecter — fournisseur</title><h1>Se connecter</h1><script>
fetch('/opener?has=' + (window.opener ? 1 : 0));
window.opener.postMessage('connecté', '*');
setTimeout(() => window.close(), 2500);
</script>`,
        { 'Set-Cookie': 'sid=s3cret; Path=/; SameSite=Lax' },
      );
    if (url.pathname === '/stay') return html(res, `<title>Reste — fournisseur</title><p>${url.searchParams.get('i')}</p>`);
    if (url.pathname === '/opener') return res.end('ok');
    html(res, `<title>Session — fournisseur</title><p>${cookie.includes('sid=s3cret') ? 'connecté' : 'anonyme'}</p>`);
  });
  // A service of this machine the project does not declare: never reached.
  const closed = server('closed', (_u, res) => html(res, '<title>Console — base</title><script>fetch("/ran")</script>'));

  const listen = (x: ReturnType<typeof server>, host: string) =>
    new Promise<string>((ok) => x.s.listen(0, host, () => ok(`http://${host}:${(x.s.address() as AddressInfo).port}`)));
  appOrigin = await listen(site, 'localhost');
  idpOrigin = await listen(idp, '127.0.0.1');
  closedOrigin = await listen(closed, '127.0.0.1');
  try {
    await addProject(page, app.repo);
    // The provider's port is the project's: a launch command declares it.
    await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
    const cfg = dialog.getByRole('group', { name: 'Commande 1' });
    await cfg.getByLabel('Nom').fill('Fournisseur');
    await cfg.getByLabel('Commande', { exact: true }).fill('node -e "0"');
    await cfg.getByRole('checkbox', { name: 'Ouvrir à son lancement' }).check();
    await cfg.getByLabel('Adresse à ouvrir').fill(`${idpOrigin}/`);
    await expect(cfg.getByLabel('Adresse à ouvrir')).toHaveAttribute('aria-invalid', 'false');
    await closeSettings(dialog);
    await unfoldSections(page);

    // The tab opens on the app, which sends it to the provider: kept in the tab, the bar says where it is.
    const section = page.getByRole('region', { name: 'Navigateurs' });
    await section.getByRole('button', { name: 'Nouveau navigateur' }).click();
    const field = section.getByRole('textbox', { name: 'Adresse du nouveau navigateur' });
    await field.fill(`${appOrigin}/start`);
    await field.press('Enter');
    const row = section.locator('.row-item.browser');
    await expect(row).toHaveCount(1);
    const address = page.getByRole('textbox', { name: 'Adresse de la page' });
    await expect(address).toHaveValue(new RegExp(`^${idpOrigin}/login\\?return=`), { timeout: 30_000 });
    await expect(row.locator('.nm')).toHaveText('Connexion — fournisseur');
    // « Ouvrir dans le navigateur du système » stays, for whoever wants it: in the bar, or in its « … » when narrow.
    const system = page.getByRole('button', { name: 'Ouvrir dans le navigateur du système' });
    if (!(await system.isVisible())) {
      await page.getByRole('button', { name: 'Plus d’actions sur la page' }).click();
      await expect(page.getByRole('menuitem', { name: 'Ouvrir dans le navigateur du système' })).toBeVisible();
      await page.keyboard.press('Escape');
    }
    // Its script sends it back: kept again.
    await expect(address).toHaveValue(`${appOrigin}/back`, { timeout: 15_000 });
    await expect(row.locator('.nm')).toHaveText('Appli — demo');
    expect(idp.saw('/login')).toHaveLength(1);
    expect(site.saw('/back')).toHaveLength(1);
    // Nothing went to the system's browser.
    await expect(page.getByText(/s’est ouvert dans ton navigateur|hors d’Escouade/)).toHaveCount(0);

    // The app opens the provider's sign-in window: in Escouade, its title the provider's origin first.
    await address.fill(`${appOrigin}/home`);
    await address.press('Enter');
    await expect.poll(() => site.saw('/opened').at(-1)?.query.get('w'), { timeout: 30_000 }).toBe('1');
    await expect.poll(() => idp.saw('/popup').length, { timeout: 15_000 }).toBe(1);
    await expect
      .poll(() => titles().filter((t) => t.startsWith(idpOrigin)), { timeout: 15_000 })
      .toEqual([`${idpOrigin} — Se connecter — fournisseur`]);
    // It is the page's: `window.opener` and `postMessage` reach the tab, from the provider's origin.
    await expect.poll(() => idp.saw('/opener').at(-1)?.query.get('has'), { timeout: 15_000 }).toBe('1');
    await expect.poll(() => site.saw('/got').at(-1)?.query.get('m'), { timeout: 15_000 }).toBe('connecté');
    expect(site.saw('/got').at(-1)?.query.get('from')).toBe(idpOrigin);
    // It closes itself, as a sign-in's does.
    await expect.poll(() => titles().filter((t) => t.startsWith(idpOrigin)), { timeout: 20_000 }).toEqual([]);

    // A link to the provider, pasted in the bar (a sign-in's link of an email): the tab goes with the window's session.
    await address.fill(`${idpOrigin}/whoami`);
    await address.press('Enter');
    await expect(address).toHaveValue(`${idpOrigin}/whoami`, { timeout: 15_000 });
    await expect(row.locator('.nm')).toHaveText('Session — fournisseur');
    const who = idp.saw('/whoami').at(-1)!;
    expect(who.cookie).toContain('sid=s3cret');
    expect(who.cookie).toContain('idp=vu');

    // This machine at a port the project does not declare: refused from the page (a redirection) as from the bar.
    await address.fill(`${appOrigin}/sneak`);
    await address.press('Enter');
    await expect(page.getByText(`La page a voulu ouvrir ${closedOrigin}/admin : refusé.`).first()).toBeVisible({ timeout: 15_000 });
    await address.fill(`${closedOrigin}/`);
    await address.press('Enter');
    await expect(page.getByText(`La page a voulu ouvrir ${closedOrigin}/ : refusé.`).first()).toBeVisible({ timeout: 15_000 });
    // Nor a file, typed in the bar.
    await address.fill('file:///C:/Windows/win.ini');
    await address.press('Enter');
    await expect(page.getByText(/La page a voulu ouvrir file:\/\/\/C:\/Windows\/win\.ini : refusé\./).first()).toBeVisible({
      timeout: 15_000,
    });
    // Never loaded in the tab: it stays on the provider's page. (WebView2 sends the request of a navigation it is told to
    // refuse, as ESC-667 saw: its answer is never shown, as a page's own `fetch` could reach it anyway.)
    await page.waitForTimeout(1500);
    await expect(row.locator('.nm')).toHaveText('Session — fournisseur');
    expect(await address.inputValue()).not.toContain(closedOrigin);
    // Nothing the page of that service would ask after its document: it never ran.
    expect(closed.hits.filter((h) => h.path !== '/admin' && h.path !== '/')).toEqual([]);

    // Four windows asked at once: three open, the fourth is refused and said.
    await address.fill(`${appOrigin}/many`);
    await address.press('Enter');
    await expect(page.getByText('La page a déjà 3 fenêtres ouvertes : ferme-en une pour en ouvrir une autre.').first()).toBeVisible({
      timeout: 30_000,
    });
    await expect.poll(() => titles().filter((t) => t.startsWith(`${idpOrigin} — Reste`)).length, { timeout: 15_000 }).toBe(3);
    expect(idp.saw('/stay')).toHaveLength(3);
    // The opener closes one (`w.close()` from another origin: the window's own close, not its page's).
    await expect.poll(() => !!waiting, { timeout: 15_000 }).toBe(true);
    waiting!.end('ok');
    await expect.poll(() => titles().filter((t) => t.startsWith(`${idpOrigin} — Reste`)).length, { timeout: 15_000 }).toBe(2);

    // The tab closes: the windows its page left open close with it.
    await row.hover();
    await row.getByRole('button', { name: 'Fermer le navigateur' }).click();
    await expect(row).toHaveCount(0);
    await expect.poll(() => titles().filter((t) => t.startsWith(idpOrigin)), { timeout: 15_000 }).toEqual([]);
    // Escouade's own window is still there.
    expect(titles().length).toBeGreaterThan(0);
  } finally {
    for (const x of [site, idp, closed]) x.s.close();
  }
});
