import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test, type App } from './fixture';

// The features switched on or off (F-1, ESC-550): the Kanban and the Docs space turned off in Réglages › Fonctionnalités
// take their tab and their MCP tools away (a call is refused with the reason); turned on again, they come back with
// their ticket and their spec page. With ESCOUADE_E2E_SHOTS, the page is shot in both themes at the window's size (run
// once more with ESCOUADE_E2E_VIEWPORT=1024x728 for the small one).

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const MCP_SWITCH = 'Les agents peuvent piloter Escouade';

/** What the fake `claude mcp add` wrote of the server in Principal's .claude.json. */
function declared(app: App): { url: string; token: string } {
  const entry = JSON.parse(fs.readFileSync(app.claudeJson, 'utf8')).mcpServers.escouade;
  return { url: entry.url, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
}

/** A JSON-RPC request to the server, as an MCP client sends it. */
async function rpc(server: { url: string; token: string }, id: number | null, method: string, params: object) {
  const res = await fetch(server.url, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${server.token}`,
      'content-type': 'application/json',
      accept: 'application/json, text/event-stream',
    },
    body: JSON.stringify({ jsonrpc: '2.0', ...(id === null ? {} : { id }), method, params }),
  });
  const text = await res.text();
  return text ? JSON.parse(text) : null;
}

/** The tools the server lists now. */
async function tools(server: { url: string; token: string }): Promise<string[]> {
  const listed = await rpc(server, 10, 'tools/list', {});
  return listed.result.tools.map((t: { name: string }) => t.name);
}

/** A tool's answer: its text, and whether it is an error. */
async function call(server: { url: string; token: string }, name: string, args: object) {
  const r = await rpc(server, 11, 'tools/call', { name, arguments: args });
  return { text: String(r.result.content[0].text), error: !!r.result.isError };
}

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** Réglages › Fonctionnalités shot in both themes, at the window's size. */
async function shoot(page: Page, dialog: Locator) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const width = await page.evaluate(() => window.innerWidth);
  // Out of the way: no hover, no focus ring.
  await page.mouse.move(0, 0);
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await expect(dialog).toBeVisible();
    await page.screenshot({ path: path.join(SHOTS, `reglages-fonctionnalites-${t === 'light' ? 'clair' : 'sombre'}-${width}.png`) });
  }
  await theme(page, 'dark');
}

test('the Kanban and the Docs space turned off take their tab and their MCP tools away, and come back with their data', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: MCP_SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: MCP_SWITCH })).toHaveAttribute('aria-checked', 'true');
  await expect
    .poll(() => fs.existsSync(app.claudeJson) && !!JSON.parse(fs.readFileSync(app.claudeJson, 'utf8')).mcpServers?.escouade)
    .toBe(true);
  const server = declared(app);
  await rpc(server, 1, 'initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc(server, null, 'notifications/initialized', {});
  // A ticket and a spec page, made through the tools while both are on.
  expect((await call(server, 'create_ticket', { project: 'demo-api', title: 'Ticket gardé' })).error).toBe(false);
  const page1 = await call(server, 'write_spec_page', {
    project: 'demo-api',
    path: 'general/garde',
    title: 'Page gardée',
    body: 'Elle reste.',
  });
  expect(page1.error, page1.text).toBe(false);

  // Réglages › Fonctionnalités: every feature on, the essential first; the Kanban and the Docs space off.
  await openSettingsPage(dialog, 'Fonctionnalités');
  await expect(dialog.getByRole('heading', { level: 1, name: 'Fonctionnalités' })).toBeVisible();
  await expect(dialog.getByRole('switch', { name: 'Agents et conversation' })).toBeDisabled();
  for (const name of ['Kanban', 'Espace Docs']) {
    const sw = dialog.getByRole('switch', { name, exact: true });
    await expect(sw).toHaveAttribute('aria-checked', 'true');
    await sw.click();
    await expect(sw).toHaveAttribute('aria-checked', 'false');
  }
  // « Tâches programmées » goes with the Kanban.
  await expect(dialog.getByRole('switch', { name: 'Tâches programmées' })).toBeDisabled();
  await expect(dialog.locator('[data-feature="scheduled"]')).toContainText('Part avec « Kanban »');
  await shoot(page, dialog);
  // Saved: the core has them off.
  await expect.poll(async () => (await tools(server)).filter((t) => t === 'create_ticket' || t === 'get_spec_tree')).toEqual([]);
  await closeSettings(dialog);

  // Their tabs are gone; their tools too, and a call is refused with the reason.
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/]);
  const listed = await tools(server);
  for (const gone of ['create_ticket', 'list_tickets', 'get_board', 'get_spec_tree', 'write_spec_page', 'list_decisions'])
    expect(listed, gone).not.toContain(gone);
  for (const kept of ['list_projects', 'list_agents', 'search_memory']) expect(listed, kept).toContain(kept);
  const refused = await call(server, 'list_tickets', { project: 'demo-api' });
  expect(refused.error).toBe(true);
  expect(refused.text).toContain('« Kanban » est coupée (Réglages › Fonctionnalités)');
  const docsRefused = await call(server, 'read_spec_page', { project: 'demo-api', path: 'general/garde' });
  expect(docsRefused.text).toContain('« Espace Docs » est coupée');

  // On again: their tabs, their tools, their ticket and their page.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await openSettingsPage(dialog, 'Fonctionnalités');
  for (const name of ['Kanban', 'Espace Docs']) await dialog.getByRole('switch', { name, exact: true }).click();
  await expect(dialog.getByRole('switch', { name: 'Tâches programmées' })).toBeEnabled();
  await expect
    .poll(async () => (await tools(server)).filter((t) => t === 'create_ticket' || t === 'get_spec_tree'))
    .toEqual(['create_ticket', 'get_spec_tree']);
  await closeSettings(dialog);
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Kanban/, /^Docs/]);
  // The page written while the Docs space was on is still to share: the badge of its tab counts it (ESC-534).
  await expect(tabs.getByTestId('docs-badge')).toHaveText('1');
  expect(await tools(server)).toContain('write_spec_page');
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByText('Ticket gardé')).toBeVisible();
  await tabs.getByRole('button', { name: 'Docs' }).click();
  await expect(page.getByText('Page gardée').first()).toBeVisible();
  const back = await call(server, 'read_spec_page', { project: 'demo-api', path: 'general/garde' });
  expect(back.error).toBe(false);
  expect(back.text).toContain('Elle reste.');
});
