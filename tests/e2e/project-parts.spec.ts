import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { addProject, closeSettings, expect, openSettingsPage, test, type App } from './fixture';

// The parts of a project switched on or off (ESC-658), in the real app: in Réglages › Projets › Parties du projet, the
// Docs and the Kanban turned off take their tab and their MCP tools away for the project (a call naming it is refused
// with the reason); turned on again, they come back with their page and their ticket. A default the repository's
// .escouade.yml proposes holds while the device chose nothing, and the device's choice goes first, even after the file
// is read again.

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

/** A tool's answer: its text, and whether it is an error. */
async function call(server: { url: string; token: string }, name: string, args: object) {
  const r = await rpc(server, 11, 'tools/call', { name, arguments: args });
  return { text: String(r.result.content[0].text), error: !!r.result.isError };
}

/** The MCP server on, as Réglages › Agents turns it on: its address and its token, the session initialized. */
async function mcpOn(app: App) {
  const dialog = await openSettingsPage(app.page, 'Agents');
  await dialog.getByRole('switch', { name: MCP_SWITCH }).click();
  await expect(dialog.getByRole('switch', { name: MCP_SWITCH })).toHaveAttribute('aria-checked', 'true');
  await expect
    .poll(() => fs.existsSync(app.claudeJson) && !!JSON.parse(fs.readFileSync(app.claudeJson, 'utf8')).mcpServers?.escouade)
    .toBe(true);
  const server = declared(app);
  await rpc(server, 1, 'initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } });
  await rpc(server, null, 'notifications/initialized', {});
  return { dialog, server };
}

test('the Docs and the Kanban turned off for a project take their tab and their tools away, and come back with their data', async ({
  app,
}) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  const { dialog, server } = await mcpOn(app);
  // A ticket and a spec page, made while every part is on.
  expect((await call(server, 'create_ticket', { project: 'demo-api', title: 'Ticket gardé' })).error).toBe(false);
  const written = await call(server, 'write_spec_page', {
    project: 'demo-api',
    path: 'general/garde',
    title: 'Page gardée',
    body: 'Elle reste.',
  });
  expect(written.error, written.text).toBe(false);

  // Réglages › Projets › demo-api › Parties du projet: all on; the Docs off.
  await openSettingsPage(dialog, 'Parties du projet');
  await expect(dialog.getByRole('heading', { level: 1, name: 'Parties du projet' })).toBeVisible();
  const docs = dialog.getByRole('switch', { name: 'Docs', exact: true });
  await expect(docs).toHaveAttribute('aria-checked', 'true');
  await docs.click();
  await expect(docs).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByTestId('part-docs-by')).toHaveText(/Choisi sur ce poste/);
  await closeSettings(dialog);
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Kanban/]);
  const refused = await call(server, 'read_spec_page', { project: 'demo-api', path: 'general/garde' });
  expect(refused.error).toBe(true);
  expect(refused.text).toContain('« Docs » est coupée pour le projet « demo-api »');

  // On again: the tab, the tool and the page.
  await openSettingsPage(page, 'Parties du projet', { project: 'demo-api' });
  await dialog.getByRole('switch', { name: 'Docs', exact: true }).click();
  await expect(dialog.getByRole('switch', { name: 'Docs', exact: true })).toHaveAttribute('aria-checked', 'true');
  // The Kanban off: the project has agents alone; its settings' pages go from the bar.
  const kanban = dialog.getByRole('switch', { name: 'Kanban', exact: true });
  await kanban.click();
  await expect(kanban).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByRole('navigation').getByText('Tickets importés')).toHaveCount(0);
  await closeSettings(dialog);
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Docs/]);
  const back = await call(server, 'read_spec_page', { project: 'demo-api', path: 'general/garde' });
  expect(back.error, back.text).toBe(false);
  expect(back.text).toContain('Elle reste.');
  const noTicket = await call(server, 'list_tickets', { project: 'demo-api' });
  expect(noTicket.error).toBe(true);
  expect(noTicket.text).toContain('« Kanban » est coupée pour le projet « demo-api »');

  // The Kanban on again: its ticket is there.
  await openSettingsPage(page, 'Parties du projet', { project: 'demo-api' });
  await dialog.getByRole('switch', { name: 'Kanban', exact: true }).click();
  await expect(dialog.getByRole('switch', { name: 'Kanban', exact: true })).toHaveAttribute('aria-checked', 'true');
  await closeSettings(dialog);
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Kanban/, /^Docs/]);
  await tabs.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByText('Ticket gardé')).toBeVisible();
  expect((await call(server, 'list_tickets', { project: 'demo-api' })).text).toContain('Ticket gardé');

  // The launch and the browsers off: their sections of the sidebar go; on again, they come back.
  const side = page.locator('aside.side');
  await expect(side.getByRole('region', { name: 'Lancement' })).toBeVisible();
  await expect(side.getByRole('region', { name: 'Navigateurs' })).toBeVisible();
  await openSettingsPage(page, 'Parties du projet', { project: 'demo-api' });
  const launch = dialog.getByRole('switch', { name: 'Lancement et navigateurs', exact: true });
  await launch.click();
  await expect(launch).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByRole('navigation').getByText('Lancement', { exact: true })).toHaveCount(0);
  await closeSettings(dialog);
  await expect(side.getByRole('region', { name: 'Lancement' })).toHaveCount(0);
  await expect(side.getByRole('region', { name: 'Navigateurs' })).toHaveCount(0);
  await openSettingsPage(page, 'Parties du projet', { project: 'demo-api' });
  await dialog.getByRole('switch', { name: 'Lancement et navigateurs', exact: true }).click();
  await closeSettings(dialog);
  await expect(side.getByRole('region', { name: 'Lancement' })).toBeVisible();
  await expect(side.getByRole('region', { name: 'Navigateurs' })).toBeVisible();
});

test('a default the repository proposes holds until the device chooses, and the device’s choice goes first', async ({ app }) => {
  test.setTimeout(120_000);
  const { page } = app;
  // A repository of its own whose committed .escouade.yml proposes the Docs off.
  const repo = path.join(path.dirname(app.repo), 'parts-app');
  fs.mkdirSync(repo);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: repo, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  git('config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(repo, 'README.md'), '# parts-app\n');
  const file = path.join(repo, '.escouade.yml');
  fs.writeFileSync(file, 'version: 1\nparts:\n  docs: false\n');
  git('add', '-A');
  git('commit', '-qm', 'init');
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .click();
  const wizard = page.getByRole('dialog', { name: 'Nouveau projet' });
  await wizard.getByPlaceholder('C:\\chemin\\vers\\le\\projet').fill(repo);
  await expect(wizard.getByRole('radio', { name: /^Réglages de \.escouade\.yml/ })).toHaveAttribute('aria-checked', 'true');
  await wizard.getByRole('button', { name: 'Suivant', exact: true }).click();
  await wizard.getByRole('button', { name: /Récapitulatif/ }).click();
  await wizard.getByRole('switch', { name: 'Créer un premier agent' }).click();
  await wizard.getByRole('button', { name: 'Créer le projet' }).click();
  await expect(wizard).toBeHidden();
  const tabs = page.getByRole('group', { name: 'Vue du projet' });
  // Proposed off by the repository: no Docs tab.
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Kanban/]);
  const dialog = await openSettingsPage(page, 'Parties du projet', { project: 'parts-app' });
  const docs = dialog.getByRole('switch', { name: 'Docs', exact: true });
  await expect(docs).toHaveAttribute('aria-checked', 'false');
  await expect(dialog.getByTestId('part-docs-by')).toHaveText('Proposé par le dépôt (.escouade.yml) : ton choix ici passe devant.');
  // The device turns them on: its choice goes first.
  await docs.click();
  await expect(docs).toHaveAttribute('aria-checked', 'true');
  await expect(dialog.getByTestId('part-docs-by')).toContainText('Choisi sur ce poste : il passe devant le défaut que propose le dépôt');
  await closeSettings(dialog);
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Kanban/, /^Docs/]);
  // The file changed and read again (it proposes the Kanban off too): the Docs stay on, the Kanban goes.
  fs.writeFileSync(file, 'version: 1\nparts:\n  docs: false\n  kanban: false\n');
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/, /^Docs/], { timeout: 30_000 });
  // « Reprendre le défaut »: the repository's again.
  await openSettingsPage(page, 'Parties du projet', { project: 'parts-app' });
  await dialog.getByRole('button', { name: 'Reprendre le défaut de « Docs »' }).click();
  await expect(dialog.getByRole('switch', { name: 'Docs', exact: true })).toHaveAttribute('aria-checked', 'false');
  await closeSettings(dialog);
  await expect(tabs.getByRole('button')).toHaveText([/^Agents/]);
});
