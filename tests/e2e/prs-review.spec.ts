// « Faire relire par un agent » (ESC-66) on the real app, against the fake GitHub of tests/fixtures/fake-trackers.mjs and the
// fake claude: from the project's « Pull requests » view, an agent reads the diff of a pull request and writes its comments;
// each is kept, edited or dropped; the review leaves at once, on the click of « Publier », and nothing before it.

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** A call to the fake host, once more when a connection kept alive to the spec before was reset. */
async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

type Write = { method: string; path: string; body: Record<string, unknown> };
const writes = async (): Promise<Write[]> => (await call(`${github.url}/__writes`)).json();
/** What reached the host once the seeds are set aside: nothing but what the app wrote on it. */
const published = async (seeded: number) => (await writes()).slice(seeded).filter((w) => w.path.endsWith('/reviews'));

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 1 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
});

async function connectGithub(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await form.getByLabel(/Jeton/).fill('ghp_jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);
}

test('an agent’s review is decided comment by comment and leaves at once, on the click of « Publier » alone', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  git(app.repo, 'checkout', '-qb', 'feat/lien');
  execFileSync('node', ['-e', "require('fs').writeFileSync('lien.ts', 'export const ttl = 0;\\nexport const lien = 1;\\n')"], {
    cwd: app.repo,
  });
  git(app.repo, 'add', '.');
  git(app.repo, 'commit', '-qm', 'feat: lien magique');
  git(app.repo, 'push', '-q', 'origin', 'feat/lien');
  git(app.repo, 'checkout', '-q', 'main');
  git(app.repo, 'branch', '-qD', 'feat/lien');

  const opened = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title: 'Connexion par lien magique', body: '', head: 'feat/lien', base: 'main' }),
  });
  const { number } = (await opened.json()) as { number: number };
  await call(`${github.url}/__github/pulls/${number}`, { method: 'POST', body: JSON.stringify({ author: 'bob' }) });

  const seeded = (await writes()).length;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);

  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  const row = page
    .locator('main.prs')
    .getByRole('listitem')
    .filter({ hasText: `#${number}` });
  await row.hover();
  await row.getByRole('button', { name: 'Faire relire' }).click();

  // Read and written: comments by file and by line, the lines they are on above them.
  const view = page.locator('main.rv');
  await expect(view.getByRole('heading', { name: /Connexion par lien magique/ })).toBeVisible();
  const file = view.getByRole('region', { name: 'lien.ts' });
  await expect(file, 'the review is written').toBeVisible({ timeout: 60_000 });
  const first = view.getByRole('article', { name: 'Commentaire sur lien.ts, ligne 1' });
  await expect(first).toContainText('Cette ligne mérite un test.');
  await expect(first).toContainText('export const ttl = 0;');
  const outside = view.getByRole('article', { name: 'Commentaire sur lien.ts, ligne 9999' });
  await expect(outside).toContainText('Hors du diff');
  await expect(view.getByRole('textbox', { name: 'Texte de la relecture' })).toHaveValue(/Relu par le faux claude/);
  await expect(view.getByText('2 à trancher')).toBeVisible();
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'prs-review.png') });

  const publish = view.getByRole('button', { name: 'Publier sur GitHub' });
  await expect(publish).toBeDisabled();
  expect(await published(seeded)).toEqual([]);

  // Edited and kept, dropped, each by itself; still nothing on the host.
  await first.getByRole('button', { name: 'Corriger' }).click();
  await first.getByRole('textbox', { name: 'Corriger le commentaire' }).fill('Ajoute un test du délai.');
  await first.getByRole('button', { name: 'Garder ce texte' }).click();
  await expect(first).toContainText('Corrigé et gardé');
  await outside.getByRole('button', { name: 'Écarter' }).click();
  await expect(outside).toContainText('Écarté');
  await expect(publish).toBeEnabled();
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'prs-review-decided.png') });
  expect(await published(seeded)).toEqual([]);

  // It keeps while the user is away: the list says where it stands.
  await view.getByRole('button', { name: 'Revenir aux PR' }).click();
  await expect(row).toContainText('Relecture à publier');
  await row.hover();
  await row.getByRole('button', { name: 'Voir la relecture' }).click();
  await expect(view.getByText(/Corrigé et gardé/)).toBeVisible();

  await publish.click();
  await expect(view.getByText(/Publiée sur GitHub/)).toBeVisible();
  const sent = await published(seeded);
  expect(sent).toHaveLength(1);
  expect(sent[0].path).toBe(`/github/repos/acme/demo/pulls/${number}/reviews`);
  expect(sent[0].body.event).toBe('COMMENT');
  expect(sent[0].body.comments).toEqual([{ path: 'lien.ts', line: 1, side: 'RIGHT', body: 'Ajoute un test du délai.' }]);
  await expect(view.getByRole('button', { name: 'Publier sur GitHub' })).toHaveCount(0);

  // No branch moved, no reference left: the reviewer wrote nothing in the repository.
  expect(git(app.repo, 'for-each-ref', 'refs/escouade')).toBe('');
  expect(git(app.repo, 'branch', '--list', 'feat/lien')).toBe('');
});
