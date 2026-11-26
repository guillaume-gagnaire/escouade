// The page of one's own (ESC-677, AP-7; docs/AGENT-PAGES.md § 7.4): a fake agent asks for `custom` and is refused while
// no page is set; the repository proposes one by `pages.custom` of its .escouade.yml, which waits for the user's trust
// (Réglages › Projets › Équipe, with the commands) and is then shown in the shelter of AP-1 (its CSP, its world), fed by
// `window.escouade`, frozen and without a path; then this machine's page, chosen in Réglages › Projets › Lancement,
// comes first. At 1440 × 900 and 1024 × 728. On Windows: the read goes through WebView2's protocol (the view of AP-3 is
// not there yet: the test places the page).
import { execFileSync } from 'node:child_process';
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
  await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc('notifications/initialized', {}, true);
  const call = async (name: string, args: object) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result;
    return { texts: (r.content as { text: string }[]).map((c) => c.text), error: !!r.isError };
  };
  const json = async (name: string, args: object) => {
    const r = await call(name, args);
    expect(r.error, r.texts.join('\n')).toBe(false);
    return JSON.parse(r.texts[0]);
  };
  return { call, json };
}

/** A page of one's own that writes what `window.escouade` gives it, and what it could not change. */
const PAGE = (who: string) => `<!doctype html><html><head><title>${who}</title><link rel="stylesheet" href="page.css"></head>
<body><p id="who">${who}</p><p id="said">?</p><script src="page.js"></script></body></html>`;
const SCRIPT = `const e = window.escouade;
try { window.escouade = null; } catch {}
try { e.data.n = 99; } catch {}
const csp = !!document.querySelector('meta[http-equiv="Content-Security-Policy"]');
document.getElementById('said').textContent = [
  'titre=' + e.title, 'page=' + e.page, 'v=' + e.version, 'modele=' + e.template, 'agent=' + e.agent.name,
  'n=' + e.data.n, 'gele=' + (Object.isFrozen(e) && Object.isFrozen(e.data)), 'reste=' + (window.escouade === e),
  'csp=' + csp, 'cles=' + Object.keys(e).sort().join(','),
].join(' ');`;

function writePage(dir: string, who: string) {
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'page.html'), PAGE(who));
  fs.writeFileSync(path.join(dir, 'page.js'), SCRIPT);
  fs.writeFileSync(path.join(dir, 'page.css'), '#who { color: rgb(4, 5, 6); }');
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

test.describe('a fake agent shows the page of one’s own', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  for (const size of SIZES) {
    test(`refused without one, the repository’s once trusted, then this machine’s first (${size.width} × ${size.height})`, async ({
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
      const ask = (title: string) => ({ title, template: 'custom', data: { n: 7, note: '</script><b>x</b>' } });

      // None set: refused, saying what to take.
      let refused = await agent.call('show_page', ask('Ma revue'));
      expect(refused.error).toBe(true);
      expect(refused.texts[0]).toContain('prends review, diagram, compare ou html');

      // The repository proposes one: read, waiting for the user's trust.
      writePage(path.join(app.repo, 'design'), 'EQUIPE');
      fs.writeFileSync(path.join(app.repo, '.escouade.yml'), 'version: 1\npages:\n  custom: design/page.html\n');
      execFileSync('git', ['add', '-A'], { cwd: app.repo });
      execFileSync('git', ['commit', '-qm', 'page à soi'], { cwd: app.repo });
      await expect
        .poll(async () => (await agent.call('show_page', ask('Ma revue'))).texts[0], { timeout: 30_000 })
        .toContain('attend l’accord de l’utilisateur');

      // Said in Réglages › Projets › Lancement, trusted in « Équipe » with the commands.
      const dialog = await openSettingsPage(page, 'Lancement');
      await expect(dialog.getByRole('heading', { name: 'Pages des agents' })).toBeVisible();
      await expect(dialog.getByTestId('custom-page-from')).toContainText('design/page.html, en attente de ton accord');
      await openSettingsPage(dialog, 'Équipe');
      const pending = dialog.getByRole('region', { name: 'Commandes en attente de ton accord' });
      await expect(pending.getByText('design/page.html')).toBeVisible();
      await pending.getByRole('button', { name: 'Faire confiance à ces commandes' }).click();
      await expect(pending).toBeHidden();
      await openSettingsPage(dialog, 'Lancement');
      await expect(dialog.getByTestId('custom-page-from')).toContainText('Proposée par le dépôt (.escouade.yml) : design/page.html');

      // Shown in its shelter, fed by window.escouade: read-only, without a path, its own files served.
      const shown = await agent.json('show_page', ask('Ma revue'));
      expect(shown).toMatchObject({ page: 'p-1', browser: 'page:p-1', version: 1, files: 3 });
      const bounds = { x: Math.round(size.width / 2), y: 48, width: Math.round(size.width / 2), height: size.height - 96 };
      await invoke(page, 'agentpage_place', { page: 'p-1', bounds, placement: 'beside' });
      const text = await textOf(page, 'p-1', 'titre=Ma revue');
      expect(text).toContain('EQUIPE');
      expect(text).toContain('page=p-1 v=1 modele=custom agent=');
      expect(text).toContain('n=7 gele=true reste=true csp=true');
      expect(text).toContain('cles=agent,ask,data,lang,page,project,template,theme,ticket,title,version');
      const body = await invoke<string>(page, 'agentpage_read', { page: 'p-1', selector: 'body', text: false });
      expect(body).not.toContain('<b>x</b>');
      expect(body).not.toContain(app.repo.replace(/\\/g, '\\\\'));
      const ext = await invoke<string>(page, 'agentpage_capture', { page: 'p-1', path: test.info().outputPath(`custom-${size.width}`) });
      expect(fs.statSync(test.info().outputPath(`custom-${size.width}.${ext}`)).size).toBeGreaterThan(1000);

      // This machine's page, typed in « Page à soi »: it comes first, and says so.
      writePage(path.join(app.repo, 'mine'), 'POSTE');
      await dialog.getByRole('textbox', { name: 'Page à soi' }).fill('mine/page.html');
      await expect(dialog.getByTestId('custom-page-from')).toContainText('Réglage de ce poste : il passe devant');
      await closeSettings(dialog);
      await expect
        .poll(() => /"customPage":\s*"mine\/page\.html"/.test(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')), {
          timeout: 15_000,
        })
        .toBe(true);
      expect(await agent.json('show_page', { ...ask('Ma revue'), page: 'p-1' })).toMatchObject({ page: 'p-1', version: 2 });
      expect(await textOf(page, 'p-1', 'POSTE')).not.toContain('EQUIPE');
      expect((await agent.json('get_page', { page: 'p-1' })).template).toBe('custom');

      // A page out of the repository: the file is told, nothing of it applies.
      fs.writeFileSync(path.join(app.repo, '.escouade.yml'), "version: 1\npages:\n  custom: '../dehors.html'\n");
      execFileSync('git', ['commit', '-qam', 'dehors'], { cwd: app.repo });
      const team = await openSettingsPage(page, 'Équipe');
      await expect(team.getByText(/pages\.custom.*sort du dépôt/)).toBeVisible({ timeout: 30_000 });
      await expect(team.getByRole('region', { name: 'Commandes en attente de ton accord' })).toBeHidden();
      await closeSettings(team);
      refused = await agent.call('show_page', ask('Encore'));
      // This machine's page still comes first.
      expect(refused.error).toBe(false);
    });
  }
});
