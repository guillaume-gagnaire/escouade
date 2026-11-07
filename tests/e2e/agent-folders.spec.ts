// The folders of agents, end to end (docs/superpowers/specs/2026-11-06-dossiers-d-agents-design.md): one made by hand,
// dragged into and out of and kept after a restart; one folded over an agent that waits for an answer; and the one an
// agent gets when it creates agents through MCP.

import fs from 'node:fs';
import { addProject, expect, send, test, type App } from './fixture';

/** The ids of the cards drawn, in the order the list shows them (folders' cards included). */
const order = (page: import('@playwright/test').Page) =>
  page.locator('aside.side .agent').evaluateAll((els) => els.map((e) => e.getAttribute('data-agent')));

/** The ids of the cards inside the folder `name`. */
const inside = (page: import('@playwright/test').Page, name: string) =>
  page
    .locator('aside.side section.folder', { has: page.getByRole('group', { name, exact: true }) })
    .locator('.agent')
    .evaluateAll((els) => els.map((e) => e.getAttribute('data-agent')));

test('a folder made by hand takes an agent dragged in and out, and a click moves nothing', async ({ app }) => {
  let { page } = app;
  await addProject(page, app.repo);
  const side = () => page.locator('aside.side');
  await expect(side().locator('.agent')).toHaveCount(1);
  await page.keyboard.press('Control+N');
  await page.keyboard.press('Control+N');
  await expect(side().locator('.agent')).toHaveCount(3);
  const [a, b, c] = (await order(page)) as string[];

  // « Nouveau dossier », in the menu beside « Nouvel agent »; its name asked at once.
  await page.keyboard.press('Control+Shift+N');
  await page.getByRole('menuitem', { name: 'Nouveau dossier', exact: true }).click();
  const rename = page.getByRole('textbox', { name: 'Nom du dossier' });
  await rename.fill('Front');
  await rename.press('Enter');
  const folder = side().locator('section.folder');
  await expect(folder).toHaveCount(1);
  await expect(folder.getByRole('group', { name: 'Front', exact: true })).toBeVisible();

  // The second card dragged onto the header goes in.
  await side().locator(`.agent[data-agent="${b}"]`).dragTo(folder.locator('.folder-h'));
  await expect.poll(() => inside(page, 'Front')).toEqual([b]);

  // A click on another card opens it and moves nothing.
  const before = await order(page);
  await side().locator(`.agent[data-agent="${c}"]`).click();
  await expect(side().locator(`.agent[data-agent="${c}"]`)).toHaveAttribute('aria-current', 'true');
  expect(await order(page)).toEqual(before);
  await expect.poll(() => inside(page, 'Front')).toEqual([b]);

  // The same layout after a restart.
  page = await app.restart();
  await expect.poll(() => inside(page, 'Front')).toEqual([b]);
  expect(await order(page)).toEqual(before);

  // Dragged out, onto the loose cards: it is no longer in the folder.
  await side()
    .locator(`.agent[data-agent="${b}"]`)
    .dragTo(side().locator(`.agent[data-agent="${a}"]`), { targetPosition: { x: 40, y: 4 } });
  await expect.poll(() => inside(page, 'Front')).toEqual([]);
  await expect(side().locator('section.folder .agent')).toHaveCount(0);
});

test('a folded folder keeps showing its agent that waits for an answer', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const side = page.locator('aside.side');
  await send(page, 'question');
  const asking = side.locator('.agent.waiting');
  await expect(asking).toContainText('Quelle base de données ?');
  const asker = (await asking.getAttribute('data-agent'))!;
  // The asking agent goes into a new folder (« Nouveau dossier avec cet agent »); another agent is open.
  await asking.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Nouveau dossier avec cet agent' }).click();
  const rename = page.getByRole('textbox', { name: 'Nom du dossier' });
  await rename.fill('Données');
  await rename.press('Enter');
  await page.keyboard.press('Control+N');
  await expect(side.locator('.agent')).toHaveCount(2);
  await expect(side.locator('.agent[aria-current="true"]')).not.toHaveAttribute('data-agent', asker);

  // Folded, the folder still shows the card that waits, and its choices.
  await side.getByRole('button', { name: 'Replier Données' }).click();
  await expect(side.getByRole('button', { name: 'Déplier Données' })).toBeVisible();
  const card = side.locator(`.agent[data-agent="${asker}"]`);
  await expect(card).toBeVisible();
  await expect(card).toContainText('Quelle base de données ?');
  // Answered, the card goes in.
  await card.getByRole('button', { name: '2 SQLite' }).click();
  await expect(side.locator(`.agent[data-agent="${asker}"]`)).toHaveCount(0);
});

test.describe('through MCP', () => {
  test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

  const streams = (app: App) =>
    app.launches().filter((l) => l.argv.includes('--input-format') && !l.argv.includes('--no-session-persistence'));

  /** The server as the agent of the launch `argv` reaches it: with its own token. */
  function agentServer(argv: string[]) {
    const entry = JSON.parse(fs.readFileSync(argv[argv.indexOf('--mcp-config') + 1], 'utf8')).mcpServers.escouade;
    return { url: entry.url, token: String(entry.headers.Authorization).replace(/^Bearer /, '') };
  }

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

  test('an agent that creates agents through MCP gets a folder with them, starred', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    const side = page.locator('aside.side');
    await expect.poll(() => streams(app).length).toBe(1);
    const agent = agentServer(streams(app)[0].argv);
    const init = { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'e2e', version: '1.0.0' } };
    expect((await rpc(agent, 1, 'initialize', init)).status).toBe(200);
    await rpc(agent, null, 'notifications/initialized', {});
    let n = 1;
    const call = async (name: string, args: object) => {
      const answer = await rpc(agent, ++n, 'tools/call', { name, arguments: args });
      return JSON.parse(answer.json.result.content[0].text);
    };
    const mine = (await side.locator('.agent').first().getAttribute('data-agent'))!;
    const myName = (await side.locator('.agent .name').first().textContent())!.trim();

    const one = await call('create_agent', { project: 'demo-api', message: 'bonjour' });
    const two = await call('create_agent', { project: 'demo-api', message: 'bonjour' });
    expect(one.folder.name).toBe(myName);
    expect(two.folder.id).toBe(one.folder.id);

    // A folder named after the agent, the agent first with its star, the two created inside.
    const folder = side.locator('section.folder');
    await expect(folder).toHaveCount(1);
    await expect(folder.getByRole('group', { name: myName, exact: true })).toBeVisible();
    await expect(folder.locator('.agent')).toHaveCount(3);
    await expect(folder.locator('.agent').first()).toHaveAttribute('data-agent', mine);
    await expect(folder.locator('.agent').first().getByRole('img', { name: 'Agent principal' })).toBeVisible();
    await expect(folder.locator(`.agent[data-agent="${one.id}"]`)).toHaveCount(1);
    await expect(folder.locator(`.agent[data-agent="${two.id}"]`)).toHaveCount(1);

    // list_folders tells the same.
    const folders = await call('list_folders', { project: 'demo-api' });
    expect(folders).toHaveLength(1);
    expect(folders[0].name).toBe(myName);
    expect(folders[0].principal.id).toBe(mine);
    expect(folders[0].agents.map((x: { id: string }) => x.id)).toEqual([mine, one.id, two.id]);

    // An empty `folder`: out of any folder.
    const loose = await call('create_agent', { project: 'demo-api', message: 'bonjour', folder: '' });
    expect(loose.folder).toBeNull();
    await expect(side.locator(`.agent[data-agent="${loose.id}"]`)).toHaveCount(1);
    await expect(folder.locator(`.agent[data-agent="${loose.id}"]`)).toHaveCount(0);
  });
});
