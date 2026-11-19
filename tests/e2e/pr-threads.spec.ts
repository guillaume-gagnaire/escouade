// The threads of a pull request (ESC-474, PR-4 of the PR in Escouade) on the real app, against the fake GitHub of
// tests/fixtures/fake-trackers.mjs: the host's threads under their line in the tab « Fichiers », the resolved folded,
// a line commented, an answer, a thread resolved then opened again, each on a click and in the user's name on the host
// (the fake records it), a refusal said in the thread with the draft kept, « Suivante » by the keyboard, and a thread
// outdated once a commit is pushed, shown apart. ESCOUADE_E2E_SHOTS=<folder>: its screenshots, in both themes.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

type Asked = { method: string; path: string; body: Record<string, unknown> | null };
// What was written on GitHub: its POSTs but GraphQL's reads (a resolution and a reopening are GraphQL's writes).
const writes = async () =>
  ((await (await call(`${github.url}/__writes`)).json()) as Asked[]).filter(
    (w) => w.method !== 'GET' && (!w.path.endsWith('/graphql') || /resolveThread/i.test(String(w.body?.operationName ?? ''))),
  );
type FakeThread = {
  id: string;
  path: string;
  line: number;
  resolved?: boolean;
  outdated?: boolean;
  comments: { id: number; author: string; body: string }[];
};
const pull = async (n: number, set: Record<string, unknown> = {}) =>
  (await (await call(`${github.url}/__github/pulls/${n}`, { method: 'POST', body: JSON.stringify(set) })).json()) as {
    threadsWhole: FakeThread[];
  };

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 7 : 0);
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

const commit = (repo: string, message: string, files: Record<string, string>) => {
  for (const [p, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(repo, p)), { recursive: true });
    fs.writeFileSync(path.join(repo, p), text);
  }
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', message);
  return git(repo, 'rev-parse', 'HEAD');
};

const LINK = [
  'import { randomBytes } from "node:crypto";',
  '',
  'export async function sendMagicLink(email: string) {',
  '  const token = randomBytes(32).toString("base64url");',
  '  return mailer.send(email, link(token));',
  '}',
  '',
  'export const TTL_MS = 10 * 60 * 1000;',
].join('\n');

test('the threads: under their line, a line commented, an answer, resolved and reopened on the host, a refusal, « Suivante », outdated after a commit', async ({
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
  const first = commit(app.repo, 'feat: lien magique', {
    'src/auth/magic-link.ts': LINK + '\n',
    'src/auth/session.ts': 'export const session = { ttl: 600 };\n',
  });
  const push = () => git(app.repo, 'push', '-qf', 'origin', 'feat/lien', 'feat/lien:refs/pull/101/head');
  push();
  const created = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title: 'Connexion par lien magique', body: 'Le lien.', head: 'feat/lien', base: 'main' }),
  });
  const { number: n } = (await created.json()) as { number: number };
  // On GitHub: marta asked about the token (open), the TTL's thread is resolved, and one of the conversation.
  await pull(n, {
    author: 'bob',
    commits: [{ sha: first, message: 'feat: lien magique', author: 'bob' }],
    threads: 1,
    threadsWhole: [
      {
        id: 'PRRT_token',
        path: 'src/auth/magic-link.ts',
        line: 4,
        comments: [{ id: 501, author: 'marta', body: 'Le **jeton** se compare-t-il en temps constant ?' }],
      },
      {
        id: 'PRRT_ttl',
        path: 'src/auth/magic-link.ts',
        line: 8,
        resolved: true,
        comments: [
          { id: 502, author: 'marta', body: 'La spec dit 10 min.' },
          { id: 503, author: 'bob', body: 'Corrigé.' },
        ],
      },
    ],
    conversation: [{ id: 601, author: 'marta', body: 'Merci pour la PR !' }],
  });

  const before = (await writes()).length;
  git(app.repo, 'checkout', '-q', 'main');
  await addProject(page, app.repo, { firstAgent: false });
  git(app.repo, 'checkout', '-q', 'feat/lien');
  await connectGithub(page);
  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  await page
    .locator('main.prs')
    .getByRole('listitem')
    .filter({ hasText: `#${n}` })
    .getByRole('button', { name: new RegExp(`PR #${n}`) })
    .click();
  const view = page.getByRole('main', { name: `La PR #${n}` });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });
  await expect(view).toContainText('1 fil ouvert · 1 résolu', { timeout: 30_000 });
  await view.getByRole('tab', { name: /^Fichiers/ }).click();

  // Under its line: marta's thread after line 4; the TTL's folded, with its count.
  const link = view.getByRole('region', { name: 'src/auth/magic-link.ts' });
  const token = link.getByRole('article', { name: 'Fil sur src/auth/magic-link.ts:4' });
  await expect(token).toContainText('se compare-t-il en temps constant', { timeout: 30_000 });
  await expect(link.locator('.row[data-new="4"] + .under')).toContainText('marta');
  const ttl = link.locator('[data-thread-key="PRRT_ttl"]');
  await expect(ttl).toContainText('résolu');
  await expect(ttl).toContainText('2 messages');
  await expect(ttl).not.toContainText('Corrigé.');
  await shots(page, 'pr-threads');

  // A comment on line 3, its draft kept across tabs, sent on the click.
  await link.locator('.row[data-new="3"]').hover();
  await link.getByRole('button', { name: 'Commenter la ligne 3' }).click();
  await link.getByRole('textbox', { name: /ligne 3/ }).fill('Pourquoi **async** ici ?');
  await view.getByRole('tab', { name: /^Conversation/ }).click();
  await view.getByRole('tab', { name: /^Fichiers/ }).click();
  await link.locator('.row[data-new="3"]').hover();
  await link.getByRole('button', { name: 'Commenter la ligne 3' }).click();
  await expect(link.getByRole('textbox', { name: /ligne 3/ })).toHaveValue('Pourquoi **async** ici ?');
  await shots(page, 'pr-threads-compose');
  expect((await writes()).length).toBe(before);
  await link.getByRole('button', { name: 'Commenter maintenant', exact: true }).click();
  const mine = link.locator('.row[data-new="3"] + .under').getByRole('article');
  await expect(mine).toContainText('ada', { timeout: 30_000 });
  await expect(mine.locator('strong')).toHaveText('async');
  const sent = (await writes()).slice(before);
  expect(sent.map((w) => w.path)).toEqual([`/github/repos/acme/demo/pulls/${n}/comments`]);
  expect(sent[0].body).toMatchObject({ path: 'src/auth/magic-link.ts', line: 3, side: 'RIGHT', commit_id: first });

  // An answer in marta's thread, then resolved, then opened again: each a click, each on the host.
  await token.getByRole('button', { name: 'Répondre' }).click();
  await token.getByRole('textbox', { name: 'Répondre' }).fill('Oui : `timingSafeEqual`.');
  await token.getByRole('button', { name: 'Répondre' }).click();
  await expect(token).toContainText('timingSafeEqual', { timeout: 30_000 });
  await token.getByRole('button', { name: 'Résoudre' }).click();
  const folded = link.locator('[data-thread-key="PRRT_token"]');
  await expect(folded).toContainText('résolu', { timeout: 30_000 });
  await expect(folded.getByRole('button', { name: 'Rouvrir' })).toBeVisible();
  expect((await pull(n)).threadsWhole.find((t) => t.id === 'PRRT_token')?.resolved).toBe(true);
  await folded.getByRole('button', { name: 'Rouvrir' }).click();
  await expect(link.getByRole('article', { name: 'Fil sur src/auth/magic-link.ts:4' })).toContainText('timingSafeEqual', {
    timeout: 30_000,
  });
  expect((await pull(n)).threadsWhole.find((t) => t.id === 'PRRT_token')?.resolved).toBe(false);

  // A token without the right to write: said in the thread, the draft kept, nothing written.
  await pull(n, { writeStatus: 403 });
  const again = link.getByRole('article', { name: 'Fil sur src/auth/magic-link.ts:4' });
  await again.getByRole('button', { name: 'Répondre' }).click();
  await again.getByRole('textbox', { name: 'Répondre' }).fill('Encore une chose.');
  await again.getByRole('button', { name: 'Répondre' }).click();
  await expect(again.getByRole('alert')).toContainText('n’a pas le droit d’écrire sur ce dépôt', { timeout: 30_000 });
  await expect(again.getByRole('textbox', { name: 'Répondre' })).toHaveValue('Encore une chose.');
  await shots(page, 'pr-threads-error');
  await pull(n, { writeStatus: 0 });
  await again.getByRole('button', { name: 'Annuler' }).click();

  // « Suivante » by the keyboard: N from the first open thread to the next.
  await view.getByRole('tab', { name: /^Fichiers/ }).focus();
  await page.keyboard.press('n');
  const current = () => page.evaluate(() => document.activeElement?.closest('[data-thread-key]')?.getAttribute('data-thread-key') ?? null);
  // Line 3 (the comment written above) comes before line 4.
  await expect.poll(current).toMatch(/^PRRT_\d+$/);
  await page.keyboard.press('n');
  await expect.poll(current).toBe('PRRT_token');

  // A commit pushed changes line 3: GitHub says the thread outdated, and it goes apart, as such.
  const second = commit(app.repo, 'fix: sans async', {
    'src/auth/magic-link.ts': LINK.replace('export async function', 'export function') + '\n',
  });
  push();
  const now = await pull(n);
  await pull(n, {
    updatedAt: '2026-11-15T12:00:00Z',
    threads: 2,
    commits: [
      { sha: first, message: 'feat: lien magique', author: 'bob' },
      { sha: second, message: 'fix: sans async', author: 'bob' },
    ],
    threadsWhole: now.threadsWhole.map((t) => (t.line === 3 ? { ...t, outdated: true } : t)),
  });
  const apart = link.getByRole('group', { name: /^Périmé/ });
  await expect(apart).toContainText('Pourquoi', { timeout: 120_000 });
  await expect(apart).toContainText('src/auth/magic-link.ts:3');
  await expect(link.locator('.row[data-new="3"] + .under')).toHaveCount(0);
  await expect(link.getByRole('table')).not.toContainText('export async function', { timeout: 30_000 });
  await apart.scrollIntoViewIfNeeded();
  await shots(page, 'pr-threads-outdated');

  // What was asked of GitHub: the comment, the answer, the resolution, the reopening, and the answer it refused.
  const all = (await writes())
    .slice(before)
    .map((w) => (w.path.endsWith('/graphql') ? `graphql#${(w.body as { operationName?: string })?.operationName}` : w.path));
  expect(all.filter((w) => w.includes('/comments') || /^graphql#(Un)?[Rr]esolveThread$/.test(w))).toEqual([
    `/github/repos/acme/demo/pulls/${n}/comments`,
    `/github/repos/acme/demo/pulls/${n}/comments/501/replies`,
    'graphql#ResolveThread',
    'graphql#UnresolveThread',
    `/github/repos/acme/demo/pulls/${n}/comments/501/replies`,
  ]);
});
