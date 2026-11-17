// The band of the team a repository names (TS-7): a repository whose `.escouade.yml` carries the key `team`, opened on a
// device of no such team. The band says the team's name and its server, said to come from the repository, the name
// cleaned (a direction character, a control character); « Demander une invitation » opens the screen of the team's
// entry with the address filled in, nothing else. A server that counts its requests stands at the file's address: it
// receives none, before nor after the gesture. « Ne plus proposer pour ce projet » dismisses it, until the file names
// another team; a file whose address does not pass shows no band.
//
// The port of the counting server: the worktree's range (`ESCOUADE_PORT_BASE` + 3: the specs run one at a time) when the
// run has one, else one the system gives.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const TEAM = '0123456789abcdef0123456789abcdef';
const OTHER = 'fedcba9876543210fedcba9876543210';

let counter: http.Server | null = null;
let hits: string[] = [];
let port = 0;

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 3;
  return new Promise((resolve) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => {
      const p = (s.address() as net.AddressInfo).port;
      s.close(() => resolve(p));
    });
  });
}

test.beforeAll(async () => {
  port = await freePort();
  counter = http.createServer((req, res) => {
    hits.push(`${req.method} ${req.url}`);
    res.writeHead(200, { 'content-type': 'text/plain' });
    res.end('ok');
  });
  await new Promise<void>((r) => counter!.listen(port, '127.0.0.1', () => r()));
});

test.afterAll(async () => {
  await new Promise((r) => counter?.close(r));
});

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['light', 'dark'] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
  }
  await page.evaluate(() => {
    delete document.documentElement.dataset.theme;
    document.documentElement.style.colorScheme = '';
  });
}

const teamKey = (server: string, id: string, name: string) =>
  ['version: 1', 'team:', `  server: ${server}`, `  id: ${id}`, `  name: ${JSON.stringify(name)}`, ''].join('\n');

test('a repository of a team shows the band of the invitation and reaches nothing before the gesture', async ({ app }) => {
  test.setTimeout(240_000);
  const page = app.page;
  hits = [];
  // A name that would turn on itself (U+202E) and ring (U+0007): shown cleaned.
  fs.writeFileSync(path.join(app.repo, '.escouade.yml'), teamKey(`http://127.0.0.1:${port}`, TEAM, 'Acme‮ Corp\u0007'));
  execFileSync('git', ['-C', app.repo, 'add', '.escouade.yml']);
  execFileSync('git', ['-C', app.repo, '-c', 'user.name=Test', '-c', 'user.email=test@example.com', 'commit', '-q', '-m', 'team']);
  await addProject(page, app.repo, { firstAgent: false, name: 'acme-api' });

  const band = page.getByTestId('team-hint-band');
  await expect(band).toBeVisible({ timeout: 15_000 });
  await expect(band).toHaveAttribute('data-kind', 'invite');
  await expect(band).toContainText(`Ce projet appartient à l’équipe Acme Corp sur 127.0.0.1:${port}, d’après le dépôt.`);
  await expect(band).toContainText('Escouade ne joint ce serveur qu’à ton geste');
  expect(await band.innerText()).not.toMatch(/[‮\u0007]/);
  await shots(page, 'team-hint-band');
  expect(hits, 'no request before the gesture').toEqual([]);

  // « Demander une invitation »: the screen of the entry, the address filled in, nothing asked of it.
  await band.getByRole('button', { name: 'Demander une invitation' }).click();
  const join = page.getByRole('dialog', { name: 'Rejoindre une équipe' });
  await expect(join.getByTestId('team-join-expected')).toContainText(`D’après le dépôt, l’équipe est sur 127.0.0.1:${port}`);
  await shots(page, 'team-hint-join');
  await page.waitForTimeout(1500);
  expect(hits, 'no request after it either').toEqual([]);
  await page.keyboard.press('Escape');
  await expect(join).toBeHidden();

  // « Ne plus proposer pour ce projet »: gone, and so after a restart.
  await band.getByRole('button', { name: 'Ne plus proposer pour ce projet' }).click();
  await expect(band).toBeHidden();
  const again = await app.restartAs(app.data);
  await expect(again.getByRole('button', { name: /acme-api/ }).first()).toBeVisible({ timeout: 15_000 });
  await again.waitForTimeout(1500);
  await expect(again.getByTestId('team-hint-band')).toBeHidden();

  // The file changed between two openings: another team, the band again; an address that does not pass, none.
  fs.writeFileSync(path.join(app.repo, '.escouade.yml'), teamKey(`http://127.0.0.1:${port}`, OTHER, 'Autre'));
  await expect(again.getByTestId('team-hint-band')).toContainText('l’équipe Autre', { timeout: 20_000 });
  fs.writeFileSync(path.join(app.repo, '.escouade.yml'), teamKey('file:///c/evil', TEAM, 'Piège'));
  await expect(again.getByTestId('team-hint-band')).toBeHidden({ timeout: 20_000 });
  fs.writeFileSync(path.join(app.repo, '.escouade.yml'), teamKey(`https://user:secret@127.0.0.1:${port}`, OTHER, 'Piège'));
  await again.waitForTimeout(3000);
  await expect(again.getByTestId('team-hint-band')).toBeHidden();
  expect(hits, 'never a request to the address of the file').toEqual([]);
});
