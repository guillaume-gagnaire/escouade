// « Faire relire par un agent » (ESC-66, ESC-490 for PR-5 of the PR in Escouade) on the real app, against the fake GitHub
// of tests/fixtures/fake-trackers.mjs and the fake claude: from the project's « Pull requests » view, an agent reads the
// diff of a pull request and its remarks come as its threads under their line in the tab « Fichiers », marked « agent,
// pas publié », with their severity and their suggestion; one is dropped (resolved here), one published by a click (the
// fake GitHub receives it in the user's name, it becomes the host's thread), one kept and published after a confirmation;
// the summary is in the Conversation; read again after a commit, the remark whose code changed is said stale and what
// was dropped or published stays so. Nothing reaches the host but those clicks. ESCOUADE_E2E_SHOTS=<folder>: its
// screenshots, in both themes.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
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
/** What the app wrote on the host: its POSTs but GraphQL's reads. */
const writes = async (): Promise<Write[]> =>
  ((await (await call(`${github.url}/__writes`)).json()) as Write[]).filter((w) => w.method !== 'GET' && !w.path.endsWith('/graphql'));
const pull = async (n: number, set: Record<string, unknown> = {}) =>
  (await call(`${github.url}/__github/pulls/${n}`, { method: 'POST', body: JSON.stringify(set) })).json();

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 1 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
  process.env.ESCOUADE_PR_WATCH_EVERY_MS = '1000';
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
  delete process.env.ESCOUADE_PR_WATCH_EVERY_MS;
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

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT}` : '';
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(SHOTS, `${name}${size}-${t}.png`) });
  }
  await theme(page, 'dark');
}

const commit = (repo: string, message: string, text: string) => {
  fs.writeFileSync(path.join(repo, 'lien.ts'), text);
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', message);
  return git(repo, 'rev-parse', 'HEAD');
};

test('the reviewer’s remarks: threads under their line, dropped, published one by one or kept and confirmed, read again after a commit', async ({
  app,
}) => {
  test.setTimeout(420_000);
  const { page } = app;
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  git(app.repo, 'checkout', '-qb', 'feat/lien');
  const first = commit(app.repo, 'feat: lien magique', 'export const ttl = 0;\nexport const lien = 1;\n');

  const opened = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title: 'Connexion par lien magique', body: '', head: 'feat/lien', base: 'main' }),
  });
  const { number: n } = (await opened.json()) as { number: number };
  const push = () => git(app.repo, 'push', '-qf', 'origin', 'feat/lien', `feat/lien:refs/pull/${n}/head`);
  push();
  await pull(n, { author: 'bob', commits: [{ sha: first, message: 'feat: lien magique', author: 'bob' }] });

  const seeded = (await writes()).length;
  git(app.repo, 'checkout', '-q', 'main');
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  git(app.repo, 'checkout', '-q', 'feat/lien');
  await connectGithub(page);

  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  const row = page
    .locator('main.prs')
    .getByRole('listitem')
    .filter({ hasText: `#${n}` });
  await row.hover();
  await row.getByRole('button', { name: 'Faire relire' }).click();

  // The view of the pull request, on its tab « Fichiers »: the agent's remarks, threads under their line.
  const view = page.getByRole('main', { name: `La PR #${n}` });
  await expect(view.getByRole('tab', { name: /^Fichiers/ })).toHaveAttribute('aria-selected', 'true');
  const file = view.getByRole('region', { name: 'lien.ts' });
  const bug = file.getByRole('article', { name: 'Remarque de l’agent sur lien.ts:1' });
  await expect(bug, 'the review is written').toContainText('Cette ligne mérite un test.', { timeout: 90_000 });
  await expect(file.locator('.row[data-new="1"] + .under')).toContainText('Cette ligne mérite un test.');
  await expect(bug).toContainText('agent, pas publié');
  await expect(bug.getByLabel('Sévérité : Bogue')).toBeVisible();
  await expect(bug.getByRole('group', { name: 'Suggestion' })).toContainText('export const ttl = 0; // testé');
  const named = file.getByRole('article', { name: 'Remarque de l’agent sur lien.ts:2' });
  await expect(named).toContainText('Ce nom mérite mieux.');
  // Out of the diff: apart, in its file; its remote image not loaded.
  const outside = file.getByRole('article', { name: 'Remarque de l’agent sur lien.ts:9999' });
  await expect(file.getByRole('group', { name: /hors du diff/ })).toContainText('Hors du diff.');
  await expect(outside.locator('img')).toHaveCount(0);
  // On the whole pull request: above the files.
  const whole = view.getByRole('group', { name: '1 remarque sur la PR entière' }).getByRole('article');
  await expect(whole).toContainText('La description ne dit pas comment tester.');
  // Their count by severity in the head of the tab.
  const bar = view.getByRole('region', { name: 'Relecture de l’agent' });
  await expect(bar.getByRole('button', { name: /^Bogue/ })).toContainText('1');
  await expect(bar.getByRole('button', { name: /^Spec/ })).toContainText('1');
  await expect(bar.getByRole('button', { name: /^Détail/ })).toContainText('2');
  await bug.scrollIntoViewIfNeeded();
  await shots(page, 'pr-review');
  expect((await writes()).slice(seeded), 'nothing reaches the host before a click').toEqual([]);

  // Filtered by severity, then all again.
  await bar.getByRole('button', { name: /^Bogue/ }).click();
  await expect(named).toHaveCount(0);
  await bar.getByRole('button', { name: /^Bogue/ }).click();
  await expect(named).toBeVisible();

  // Dropped: its thread resolved here, folded, nothing on the host.
  await outside.getByRole('button', { name: /^Écarter/ }).click();
  const dropped = file.locator('.thread.agent.folded');
  await expect(dropped).toContainText('Écartée');
  expect((await writes()).slice(seeded)).toEqual([]);

  // Published one by one: its text read and corrected, then on the click, on its line, in the user's name.
  await bug.getByRole('button', { name: /^Publier/ }).click();
  await bug.getByRole('textbox', { name: 'Texte publié sur GitHub' }).fill('Ajoute un test du délai.');
  await shots(page, 'pr-review-publish');
  expect((await writes()).slice(seeded)).toEqual([]);
  await bug.getByRole('button', { name: /Publier sur GitHub/ }).click();
  const hostThread = file.getByRole('article', { name: 'Fil sur lien.ts:1' });
  await expect(hostThread).toContainText('Ajoute un test du délai.', { timeout: 30_000 });
  await expect(hostThread).toContainText('ada');
  await expect(bug, 'no double: the thread is the host’s now').toHaveCount(0);
  let sent = (await writes()).slice(seeded);
  expect(sent.map((w) => w.path)).toEqual([`/github/repos/acme/demo/pulls/${n}/comments`]);
  expect(sent[0].body).toMatchObject({ path: 'lien.ts', line: 1, side: 'RIGHT' });
  expect(String(sent[0].body.body)).toBe('Ajoute un test du délai.\n\n```suggestion\nexport const ttl = 0; // testé\n```');

  // Kept, then published with the others kept after a confirmation that lists them.
  await whole.getByRole('button', { name: /^Garder/ }).click();
  await expect(whole).toContainText('Gardée');
  await bar.getByRole('button', { name: 'Publier la remarque gardée' }).click();
  const confirm = view.getByRole('dialog', { name: 'Publier la remarque gardée ?' });
  await expect(confirm).toContainText('La description ne dit pas comment tester.');
  await shots(page, 'pr-review-batch');
  expect((await writes()).slice(seeded)).toHaveLength(1);
  await confirm.getByRole('button', { name: 'Publier', exact: true }).click();
  await expect(confirm).toHaveCount(0, { timeout: 30_000 });
  sent = (await writes()).slice(seeded);
  expect(sent.map((w) => w.path)).toEqual([`/github/repos/acme/demo/pulls/${n}/comments`, `/github/repos/acme/demo/issues/${n}/comments`]);
  expect(sent[1].body.body).toBe('La description ne dit pas comment tester.');

  // The summary in the Conversation: what it read, at which commit, its remarks by severity, a link to each.
  await view.getByRole('tab', { name: /^Conversation/ }).click();
  const summary = view.getByRole('region', { name: 'Résumé de la relecture de l’agent' });
  await expect(summary).toContainText(`a relu la PR au commit ${first.slice(0, 7)}`);
  await expect(summary).toContainText('4 remarques · 1 Détail');
  await expect(summary).toContainText('Relu par le faux claude (dossier vide)');
  await shots(page, 'pr-review-summary');
  await summary.getByRole('button', { name: 'Voir la remarque sur lien.ts:2' }).click();
  await expect(view.getByRole('tab', { name: /^Fichiers/ })).toHaveAttribute('aria-selected', 'true');
  await expect(named).toHaveClass(/current/);

  // A commit changes line 2: read again, the remark on it is stale (apart), a new one comes under its line; the dropped
  // one stays dropped, the published one stays the host's.
  const second = commit(app.repo, 'fix: nom', 'export const ttl = 0;\nexport const lienMagique = 1;\n');
  push();
  await pull(n, {
    updatedAt: '2026-11-16T12:00:00Z',
    commits: [
      { sha: first, message: 'feat: lien magique', author: 'bob' },
      { sha: second, message: 'fix: nom', author: 'bob' },
    ],
  });
  // A read is kept 20 s (PR-2): read again until the new diff is there.
  await expect(async () => {
    await view.getByRole('button', { name: 'Relire la PR' }).click();
    await expect(file.getByRole('table')).toContainText('lienMagique', { timeout: 5_000 });
  }).toPass({ timeout: 120_000 });
  await view.getByRole('button', { name: /Relire à nouveau/ }).click();
  const apart = file.getByRole('group', { name: /hors du diff/ });
  await expect(apart.getByRole('article', { name: 'Remarque de l’agent sur lien.ts:2' })).toContainText('périmée', {
    timeout: 90_000,
  });
  await expect(file.locator('.row[data-new="2"] + .under').getByRole('article')).toContainText('Ce nom mérite mieux.');
  await expect(file.locator('.thread.agent.folded')).toContainText('Écartée');
  await expect(file.getByRole('article', { name: 'Remarque de l’agent sur lien.ts:1' })).toHaveCount(0);
  await apart.scrollIntoViewIfNeeded();
  await shots(page, 'pr-review-again');
  expect((await writes()).slice(seeded), 'a reading again writes nothing').toHaveLength(2);

  // No branch moved, no reference left: the reviewer wrote nothing in the repository.
  expect(git(app.repo, 'for-each-ref', 'refs/escouade')).toBe('');
});
