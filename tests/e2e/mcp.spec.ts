import fs from 'node:fs';
import { addProject, expect, test, type App } from './fixture';

// Escouade's MCP server, declared in Claude Code by the fake `claude mcp add` (in the .claude.json of a folder of the test's),
// and called as Claude Code calls it: JSON-RPC over HTTP, with the token it was declared with.

const SWITCH = 'Claude peut piloter Escouade';

/** What the fake `claude mcp add` wrote of the server in Principal's .claude.json, if it did. */
function declared(app: App): { url: string; token: string } | null {
  if (!fs.existsSync(app.claudeJson)) return null;
  const entry = JSON.parse(fs.readFileSync(app.claudeJson, 'utf8')).mcpServers?.escouade;
  if (!entry) return null;
  return { url: entry.url, token: String(entry.headers?.Authorization ?? '').replace(/^Bearer /, '') };
}

/** A JSON-RPC request to the server, as an MCP client sends it; the answer's JSON. */
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
  return { status: res.status, json: text ? JSON.parse(text) : null };
}

test('Claude drives Escouade: a ticket created through MCP arrives in the Kanban', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByTitle('Réglages (Ctrl+,)').click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Claude Code' }).click();
  await expect(dialog.getByRole('heading', { name: 'Escouade dans Claude' })).toBeVisible();
  // Off to begin with: nothing declared anywhere.
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  expect(declared(app)).toBeNull();

  // On: the server starts and is declared in Claude Code for Principal, saved at once (no « Enregistrer »).
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByText('Déclaré dans Claude · compte Principal')).toBeVisible();
  const server = declared(app);
  expect(server).not.toBeNull();
  expect(server!.url).toMatch(/^http:\/\/127\.0\.0\.1:\d+\/mcp$/);
  expect(server!.token.length).toBeGreaterThan(30);
  // `claude mcp add` ran once, in the data folder, as Claude Code is told: user scope, HTTP, the header last.
  const adds = app.launches().filter((l) => l.argv[0] === 'mcp');
  expect(adds).toHaveLength(1);
  expect(adds[0].argv.slice(1, 8)).toEqual(['add', '--scope', 'user', '--transport', 'http', 'escouade', server!.url]);
  expect(adds[0].argv.slice(8)).toEqual(['--header', `Authorization: Bearer ${server!.token}`]);
  expect(adds[0].cwd.toLowerCase()).toBe(app.data.toLowerCase());

  // Nobody without the token comes in.
  const refused = await fetch(server!.url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{}' });
  expect(refused.status).toBe(401);

  // As Claude Code does: `initialize`, then a tool.
  const init = await rpc(server!, 1, 'initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'e2e', version: '1.0.0' },
  });
  expect(init.status).toBe(200);
  expect(init.json.result.serverInfo.name).toBe('escouade');
  expect((await rpc(server!, null, 'notifications/initialized', {})).status).toBe(202);
  const made = await rpc(server!, 2, 'tools/call', {
    name: 'create_ticket',
    arguments: { project: 'demo-api', title: 'Ticket venu de Claude', criteria: ['Le fichier existe'] },
  });
  expect(made.status).toBe(200);
  expect(made.json.result.isError).toBeFalsy();
  const ticket = JSON.parse(made.json.result.content[0].text);
  expect(ticket).toMatchObject({ title: 'Ticket venu de Claude', column: 'todo' });
  expect(ticket.description).toBe('Créé par Claude (hors Escouade) via Escouade');
  // A call that is refused is told as well: a ticket of a project that does not exist.
  const lost = await rpc(server!, 3, 'tools/call', { name: 'create_ticket', arguments: { project: 'nope', title: 'x' } });
  expect(lost.json.result.isError).toBe(true);

  // The activity shows them, newest first, live (the dialog has been open all along): the failed call, the ticket made, and
  // the request without a token, refused, from a client it does not know.
  const calls = dialog.getByRole('list', { name: 'Appels au serveur MCP, les plus récents en tête' }).getByRole('listitem');
  await expect(calls).toHaveCount(3);
  await expect(calls.nth(0)).toContainText('create_ticket');
  await expect(calls.nth(0)).toContainText('Erreur');
  await expect(calls.nth(1)).toContainText('Claude (hors Escouade)');
  await expect(calls.nth(1)).toContainText('create_ticket');
  await expect(calls.nth(1)).toContainText('Ticket venu de Claude');
  await expect(calls.nth(1)).toContainText('Fait');
  await expect(calls.nth(2)).toContainText('Client inconnu');
  await expect(calls.nth(2)).toContainText('Refusé');
  await dialog.getByRole('button', { name: 'Effacer' }).click();
  await expect(dialog.getByText('Aucun appel pour l’instant.')).toBeVisible();

  // And the ticket is on the board.
  await dialog.getByRole('button', { name: 'Fermer' }).click();
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByText('Ticket venu de Claude')).toBeVisible();
});

test('turning Claude’s control of Escouade off takes it out of Claude Code and changes the token', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByTitle('Réglages (Ctrl+,)').click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Claude Code' }).click();
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByText('Déclaré dans Claude · compte Principal')).toBeVisible();
  const before = declared(app)!;
  expect((await rpc(before, 1, 'ping', {})).status).toBe(200);

  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByText(/Déclaré dans Claude/)).toBeHidden();
  await expect.poll(() => declared(app)).toBeNull();
  const removes = app.launches().filter((l) => l.argv[0] === 'mcp' && l.argv[1] === 'remove');
  expect(removes.map((l) => l.argv.slice(1))).toEqual([['remove', 'escouade', '--scope', 'user']]);
  // The server is stopped with it (no project lets its agents use it): the old token opens nothing.
  await expect(async () => {
    const res = await fetch(before.url, { method: 'POST', headers: { authorization: `Bearer ${before.token}` }, body: '{}' }).catch(
      () => null,
    );
    expect(res === null || res.status === 401).toBe(true);
  }).toPass();

  // On again: declared again, with another token.
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByText('Déclaré dans Claude · compte Principal')).toBeVisible();
  const after = declared(app)!;
  expect(after.token).not.toBe(before.token);
});
