import fs from 'node:fs';
import path from 'node:path';
import {
  addProject,
  expect,
  FAKE_CODEX,
  FAKE_COPILOT,
  FAKE_OPENCODE,
  send,
  test,
  type App,
  openSettingsPage,
  closeSettings,
} from './fixture';

// Escouade's MCP server, declared in Claude Code by the fake `claude mcp add` (in the .claude.json of a folder of the test's),
// and called as Claude Code calls it: JSON-RPC over HTTP, with the token it was declared with.

// The one switch (ESC-91, ESC-119): every agent of Escouade, and every harness installed outside Escouade.
const SWITCH = 'Les agents peuvent piloter Escouade';

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

/** The line of a declaration under the switch: `where` is « <harness> · compte <name> ». */
function row(dialog: import('@playwright/test').Locator, where: string) {
  return dialog.getByRole('list', { name: 'Déclarations dans les harness' }).getByRole('listitem').filter({ hasText: where });
}

test('Claude drives Escouade: a ticket created through MCP arrives in the Kanban', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await expect(dialog.getByRole('heading', { level: 1, name: 'Agents' })).toBeVisible();
  // Off to begin with: nothing declared anywhere.
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  expect(declared(app)).toBeNull();

  // On: the server starts and is declared in Claude Code for Principal, saved at once (no « Enregistrer »).
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(row(dialog, 'Claude · compte Principal')).toContainText('Déclaré');
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

  // The activity shows them, newest first, live (the dialog has been open all along), in the page's « Avancé »: the failed
  // call, the ticket made, and the request without a token, refused, from a client it does not know.
  await dialog
    .locator('#settings-panel')
    .getByRole('button', { name: /^Avancé/ })
    .click();
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
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(row(dialog, 'Claude · compte Principal')).toContainText('Déclaré');
  const before = declared(app)!;
  expect((await rpc(before, 1, 'ping', {})).status).toBe(200);

  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByRole('list', { name: 'Déclarations dans les harness' })).toBeHidden();
  await expect.poll(() => declared(app)).toBeNull();
  const removes = app.launches().filter((l) => l.argv[0] === 'mcp' && l.argv[1] === 'remove');
  expect(removes.map((l) => l.argv.slice(1))).toEqual([['remove', 'escouade', '--scope', 'user']]);
  // The server is stopped with it: the old token opens nothing.
  await expect(async () => {
    const res = await fetch(before.url, { method: 'POST', headers: { authorization: `Bearer ${before.token}` }, body: '{}' }).catch(
      () => null,
    );
    expect(res === null || res.status === 401).toBe(true);
  }).toPass();

  // On again: declared again, with another token.
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(row(dialog, 'Claude · compte Principal')).toContainText('Déclaré');
  const after = declared(app)!;
  expect(after.token).not.toBe(before.token);
});

/** The launches of an agent's process (a stream), apart from `claude mcp …`, the one-shot calls (naming…) and the readings of an account's models (ESC-124: no session). */
const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));
const optionValue = (argv: string[], name: string) => argv[argv.indexOf(name) + 1];

test('the one switch reaches agents already running: their conversation says so, and « Relancer » resumes the session with the tools', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  // Started with the switch off: Escouade's tools refused, and its server kept out of it (else an entry of the user's
  // config would still give it the server's instructions).
  const first = streams(app).at(-1)!;
  expect(first.argv).toContain('--disallowedTools');
  expect(first.argv).not.toContain('--mcp-config');
  expect(JSON.parse(fs.readFileSync(optionValue(first.argv, '--settings'), 'utf8'))).toEqual({
    deniedMcpServers: [{ serverName: 'escouade' }],
  });
  const banner = page.getByRole('region', { name: 'Outils d’Escouade au prochain démarrage' });
  await expect(banner).toBeHidden();

  // On: the settings count the agent already running, which gets the tools at its next start.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: SWITCH }).click();
  await expect(row(dialog, 'Claude · compte Principal')).toContainText('Déclaré');
  await expect(
    dialog.getByText('1 agent déjà lancé aura les outils au prochain démarrage de son process : « Relancer » sur sa conversation.'),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Fermer' }).click();

  // Its conversation says so; « Relancer » starts its process again on its session, with the server as itself.
  await expect(banner).toBeVisible();
  const before = streams(app).length;
  await banner.getByRole('button', { name: 'Relancer' }).click();
  await expect(banner).toBeHidden();
  await expect.poll(() => streams(app).length).toBe(before + 1);
  const restarted = streams(app).at(-1)!;
  // The session of its first process, where « Bonjour » went (the fake names a new one after its process id).
  expect(first.argv.some((a) => a.startsWith('--resume='))).toBe(false);
  expect(restarted.argv).toContain(`--resume=sess-${first.pid}`);
  expect(restarted.argv).toContain('--mcp-config');
  expect(restarted.argv).not.toContain('--disallowedTools');
  expect(restarted.argv).not.toContain('--settings');
  // Nothing of the conversation is lost, and it goes on.
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await send(page, 'Encore');
  await expect(page.getByText('Bonjour, tu as dit : Encore')).toBeVisible();
});

test.describe('the switch saved on, the app starting', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  test('a new agent of a project reaches the server as itself, with nothing to allow in the project', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await expect.poll(() => streams(app).length).toBeGreaterThan(0);
    const launch = streams(app).at(-1)!;
    expect(launch.argv).not.toContain('--disallowedTools');
    const entry = JSON.parse(fs.readFileSync(optionValue(launch.argv, '--mcp-config'), 'utf8')).mcpServers.escouade;
    const agent = { url: entry.url, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
    // Its own token, not the one of Claude outside Escouade, and the server's tools with it.
    await expect.poll(() => declared(app)?.token ?? null).not.toBeNull();
    expect(agent.token).not.toBe(declared(app)!.token);
    const init = await rpc(agent, 1, 'initialize', {
      protocolVersion: '2025-11-25',
      capabilities: {},
      clientInfo: { name: 'e2e', version: '1.0.0' },
    });
    expect(init.status).toBe(200);
    await rpc(agent, null, 'notifications/initialized', {});
    const tools = await rpc(agent, 2, 'tools/list', {});
    expect(tools.json.result.tools.map((t: { name: string }) => t.name)).toContain('create_ticket');
    // And the project's settings have no switch of their own any more.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Git et worktrees');
    await expect(dialog.getByRole('switch', { name: 'Un worktree par agent' })).toBeVisible();
    await expect(dialog.getByRole('switch', { name: 'Les agents peuvent utiliser Escouade' })).toHaveCount(0);
  });
});

// « Projet privé » (ESC-139): what the agents of each project see of the others through the server.
test.describe('a private project', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  /** The server as the agent of the launch `argv` reaches it: with its own token. */
  function agentServer(argv: string[]) {
    const entry = JSON.parse(fs.readFileSync(optionValue(argv, '--mcp-config'), 'utf8')).mcpServers.escouade;
    return { url: entry.url, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
  }

  /** The names of the projects `list_projects` gives `server`'s caller. */
  async function projectNames(server: { url: string; token: string }) {
    const init = { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } };
    expect((await rpc(server, 1, 'initialize', init)).status).toBe(200);
    await rpc(server, null, 'notifications/initialized', {});
    const answer = await rpc(server, 2, 'tools/call', { name: 'list_projects', arguments: {} });
    const rows = JSON.parse(answer.json.result.content[0].text) as { name: string }[];
    return rows.map((r) => r.name).sort();
  }

  test('is listed to its own agents alone, who see no other, and the user sees both', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await expect.poll(() => streams(app).length).toBe(1);
    const open = agentServer(streams(app)[0].argv);
    await addProject(page, app.repo, { name: 'coffre' });
    await expect.poll(() => streams(app).length).toBe(2);
    const vault = agentServer(streams(app)[1].argv);
    // Before: neither is private, each agent sees both.
    expect(await projectNames(open)).toEqual(['coffre', 'demo-api']);
    // « Projet privé » switched on in the settings of « coffre ».
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Confidentialité');
    await dialog.getByRole('button', { name: 'coffre' }).click();
    await dialog.getByRole('switch', { name: 'Projet privé' }).click();
    await closeSettings(dialog);
    // Its tile says so; the other's does not.
    const lock = 'Projet privé : invisible pour les agents des autres projets';
    await expect(page.getByRole('tab', { name: 'coffre, privé' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'coffre, privé' }).locator('.lock')).toBeVisible();
    await expect(page.getByRole('tab', { name: /^demo-api/ }).locator('.lock')).toHaveCount(0);
    expect(await projectNames(open)).toEqual(['demo-api']);
    expect(await projectNames(vault)).toEqual(['coffre']);
    // Claude outside Escouade, the user's own token: both.
    await expect.poll(() => declared(app)?.token ?? null).not.toBeNull();
    expect(await projectNames(declared(app)!)).toEqual(['coffre', 'demo-api']);
  });
});

test.describe('« Les agents peuvent piloter Escouade » in every harness installed', () => {
  test.use({ appEnv: { ESCOUADE_CODEX: FAKE_CODEX, ESCOUADE_OPENCODE: FAKE_OPENCODE, ESCOUADE_COPILOT: FAKE_COPILOT } });

  /** What the user had in Principal's `config.toml` of Codex. */
  const USERS = '# mon modèle\nmodel = "gpt-5"\n\n[mcp_servers.node_repl]\ncommand = "node" # le mien\n';
  /** What the user had in OpenCode's `opencode.json`: comments and a server of theirs. */
  const USERS_OPENCODE =
    '{\n  // mon modèle\n  "model": "ollama/qwen3-coder",\n  "mcp": {\n    "node": { "type": "local", "command": ["node"] }\n  }\n}\n';
  /** Copilot's `mcp-config.json` as `copilot mcp remove` leaves it. */
  const USERS_COPILOT = '{\n  "mcpServers": {}\n}';

  /** The server as Codex's `config.toml` declares it, if it does. */
  function inCodex(app: App): { url: string; token: string } | null {
    const text = fs.existsSync(app.codexToml) ? fs.readFileSync(app.codexToml, 'utf8') : '';
    const url = /\[mcp_servers\.escouade\]\nurl = "([^"]+)"/.exec(text)?.[1];
    const token = /http_headers = \{ Authorization = "Bearer ([^"]+)" \}/.exec(text)?.[1];
    return url && token ? { url, token } : null;
  }

  /** The server as a JSON user config declares it under `servers`, if it does (comments taken out). */
  function inJson(file: string, servers: string): { url: string; token: string; type: string } | null {
    if (!fs.existsSync(file)) return null;
    // Comments and trailing commas out (OpenCode and Copilot read both: Escouade writes a comma after an entry it puts
    // in an empty object of the servers).
    const text = fs
      .readFileSync(file, 'utf8')
      .replace(/^\s*\/\/.*$/gm, '')
      .replace(/,(\s*[}\]])/g, '$1');
    const entry = JSON.parse(text)[servers]?.escouade;
    if (!entry) return null;
    return { url: entry.url, token: String(entry.headers?.Authorization ?? '').replace(/^Bearer /, ''), type: entry.type };
  }

  test('one switch declares the server in Claude Code, Codex, OpenCode and Copilot, lists each, and turned off leaves every file as it was', async ({
    app,
  }) => {
    for (const [file, text] of [
      [app.codexToml, USERS],
      [app.opencodeJson, USERS_OPENCODE],
      [app.copilotJson, USERS_COPILOT],
    ]) {
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, text);
    }
    const { page } = app;
    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Agents');
    // One switch, no harness of its own; the token in plain text, said beside it.
    await expect(dialog.getByRole('switch', { name: /piloter Escouade/ })).toHaveCount(1);
    await expect(
      dialog.getByText(/Le jeton y est écrit en clair\. Désactiver change le jeton, puis retire l’entrée partout/),
    ).toBeVisible();
    await dialog.getByRole('switch', { name: SWITCH }).click();

    // Each harness and account listed, with its file.
    const list = dialog.getByRole('list', { name: 'Déclarations dans les harness' });
    for (const [where, file] of [
      ['Claude · compte Principal', app.claudeJson],
      ['Codex · compte Principal', app.codexToml],
      ['OpenCode · compte Principal', app.opencodeJson],
      ['Copilot · compte Principal', app.copilotJson],
    ]) {
      await expect(row(dialog, where)).toContainText('Déclaré');
      await expect(row(dialog, where).locator('.file')).toHaveText(file);
    }
    const claude = declared(app)!;
    // The same token everywhere, and it opens the server.
    expect(inCodex(app)).toEqual(claude);
    expect(inJson(app.opencodeJson, 'mcp')).toEqual({ ...claude, type: 'remote' });
    expect(inJson(app.copilotJson, 'mcpServers')).toEqual({ ...claude, type: 'http' });
    expect((await rpc(claude, 1, 'ping', {})).status).toBe(200);
    // The user's lines and comments kept.
    expect(fs.readFileSync(app.codexToml, 'utf8').startsWith(USERS)).toBe(true);
    const oc = fs.readFileSync(app.opencodeJson, 'utf8');
    expect(oc).toContain('// mon modèle');
    expect(oc).toContain('"node": { "type": "local", "command": ["node"] },');

    await dialog.getByRole('switch', { name: SWITCH }).click();
    await expect(dialog.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
    await expect(list).toBeHidden();
    // Byte for byte, every file.
    await expect.poll(() => fs.readFileSync(app.codexToml, 'utf8')).toBe(USERS);
    await expect.poll(() => fs.readFileSync(app.opencodeJson, 'utf8')).toBe(USERS_OPENCODE);
    await expect.poll(() => fs.readFileSync(app.copilotJson, 'utf8')).toBe(USERS_COPILOT);
    await expect.poll(() => declared(app)).toBeNull();
    // The server is off: nothing answers the old token.
    const old = await fetch(claude.url, { method: 'POST', headers: { authorization: `Bearer ${claude.token}` }, body: '{}' }).catch(
      () => null,
    );
    expect(old?.status ?? 0).not.toBe(200);
  });
});
