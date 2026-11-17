// The server mode (ESC-514, docs/REMOTE.md « Le mode serveur »): `escouade-server` (its debug build) started on a data
// folder of its own, a device paired by `escouade-server pair`, and the desktop app's window on its API through a local
// forward, as a client of a VPS through `ssh -L`. A project, an agent of the fake `claude` and its reply, all on the
// server; then the client closed and opened again finds the agent there.

import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, openNewProject, send, test } from './fixture';

test.use({ server: true });

test('the desktop app on an escouade-server through a local forward: a project, an agent’s reply, the client closed and back', async ({
  app,
}) => {
  const server = app.server!;
  let { page } = app;
  // The window speaks to the server's API, through the forward, not to its own core.
  const link = await page.evaluate(() => (window as unknown as { __ESCOUADE_REMOTE__?: { url: string } }).__ESCOUADE_REMOTE__);
  expect(link?.url).toBe(server.url);
  expect(server.url).toContain(`:${server.forward}/`);
  // Its handshake passed (the server's log says the device), then the state it read: « Nouveau projet » opens. Its first
  // click may come while the state is still on its way, through the forward, and be lost: tried again until it opens.
  await expect.poll(() => server.log(), { timeout: 30_000 }).toContain('« Poste de test » connected');
  await expect(async () => {
    await openNewProject(page);
    await expect(page.getByRole('dialog', { name: 'Nouveau projet' })).toBeVisible({ timeout: 3000 });
  }).toPass({ timeout: 30_000 });
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Nouveau projet' })).toBeHidden();
  await addProject(page, app.repo);
  await expect(page.getByRole('button', { name: /demo-api/ }).first()).toBeVisible();
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await expect(page.getByText('Tâche terminée')).toBeVisible();
  // All of it on the server: its state has the project, the app's own core nothing.
  await expect.poll(() => fs.readFileSync(path.join(server.data, 'state.json'), 'utf8'), { timeout: 20_000 }).toContain('demo-api');
  const own = path.join(app.data, 'state.json');
  expect(fs.existsSync(own) ? fs.readFileSync(own, 'utf8') : '').not.toContain('demo-api');
  // The server's log says the device and its connection, never its token.
  expect(server.log()).toContain('« Poste de test » connected');
  expect(server.log()).not.toContain(server.token);
  // The secrets of a server are in a file of its data: its handshake says so to the client.
  expect(fs.existsSync(path.join(server.data, 'secrets.json'))).toBe(true);
  // The client closed and opened again: the agent and its conversation are still there, on the server.
  page = await app.restart();
  await expect(page.locator('.card', { hasText: 'bonjour-fake' }).first()).toBeVisible({ timeout: 45_000 });
  await page.locator('.card', { hasText: 'bonjour-fake' }).first().click();
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  // And it answers again, through the server.
  await send(page, 'Encore');
  await expect(page.getByText('Bonjour, tu as dit : Encore')).toBeVisible();
});
