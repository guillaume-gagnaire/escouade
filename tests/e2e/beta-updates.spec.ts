// « Recevoir les versions bêta » (ESC-374), through the real check of the backend: a server of the test serves
// latest.json and beta.json (`ESCOUADE_UPDATE_ENDPOINT`, read by a debug build), and the app reads them as it would
// GitHub's. Off by default, the check reads latest.json alone; ticked, it is saved at once and a check reads beta.json
// first, then latest.json when there is none. Only the download is held, in the page (`fetch` of Tauri's IPC): nothing
// is installed. ESCOUADE_PORT_BASE gives the server's port when set (a worktree's own), else one the system picks.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import type { AddressInfo } from 'node:net';
import { expect, test, openSettingsPage, settingsEntry } from './fixture';

/** What the server serves and what it was asked: beta.json is absent while `beta` is null. */
const served = { latest: '9.8.0', beta: '9.9.0-beta.1' as string | null, hits: [] as string[] };

/** A latest.json of `version` whose installer the test never downloads. */
function manifest(version: string, origin: string) {
  const entry = { signature: 'test', url: `${origin}/setup-${version}.exe` };
  const targets = ['windows-x86_64', 'windows-x86_64-nsis', 'darwin-aarch64', 'darwin-x86_64', 'linux-x86_64'];
  return { version, notes: `- Notes de ${version}`, platforms: Object.fromEntries(targets.map((t) => [t, entry])) };
}

test.use({
  appEnv: async ({}, use) => {
    const server = http.createServer((req, res) => {
      const name = (req.url ?? '/').split('?')[0];
      served.hits.push(name);
      const origin = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
      const version = name === '/latest.json' ? served.latest : name === '/beta.json' ? served.beta : null;
      if (!version) {
        res.writeHead(404, { 'Content-Type': 'text/plain' }).end('not found');
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(manifest(version, origin)));
    });
    const port = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
    await new Promise<void>((resolve) => server.listen(port, '127.0.0.1', resolve));
    const { port: bound } = server.address() as AddressInfo;
    await use({ ESCOUADE_UPDATE_ENDPOINT: `http://127.0.0.1:${bound}/latest.json` });
    await new Promise((resolve) => server.close(resolve));
  },
});

test('the betas are off by default, and once ticked the check reads beta.json, then latest.json without one', async ({ app }) => {
  const { page, data } = app;
  // The download of what is found never ends: the row of the update stays as found.
  await page.evaluate(() => {
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.endsWith('/update_download')) return new Promise(() => {});
      return fetch(input, init);
    };
  });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  const beta = dialog.getByRole('switch', { name: 'Recevoir les versions bêta' });
  await expect(beta).toHaveAttribute('aria-checked', 'false');
  const found = dialog.locator('.sr', { has: page.locator('[data-update]') });

  // Off: latest.json alone, never beta.json.
  await dialog.getByRole('button', { name: 'Rechercher une mise à jour' }).click();
  await expect(found).toContainText('Escouade 9.8.0 disponible');
  expect(served.hits).toContain('/latest.json');
  expect(served.hits).not.toContain('/beta.json');

  // Ticked: saved at once, and the check that follows reads beta.json.
  await beta.click();
  await expect(beta).toHaveAttribute('aria-checked', 'true');
  await expect(found).toContainText('Escouade 9.9.0-beta.1 (bêta) disponible');
  expect(served.hits).toContain('/beta.json');
  const saved = JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8'));
  expect(saved.betaUpdates).toBe(true);
  await expect(settingsEntry(dialog, 'Général')).not.toHaveClass(/changed/);

  // No beta.json (before the first release that holds one): latest.json.
  served.beta = null;
  const before = served.hits.length;
  await dialog.getByRole('button', { name: 'Rechercher une mise à jour' }).click();
  await expect(found).toContainText('Escouade 9.8.0 disponible');
  // beta.json asked first, then latest.json (the automatic check may come in between: what it asks is the same).
  const asked = served.hits.slice(before);
  expect(asked[0]).toBe('/beta.json');
  expect(asked).toContain('/latest.json');
});
