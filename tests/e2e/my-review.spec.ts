// « Ta relecture » (ESC-494, PR-7 of the PR in Escouade) on the real app, against the fake GitHub of
// tests/fixtures/fake-trackers.mjs: two remarks kept waiting under their line of a ticket's pull request, « Demander des
// changements » published in one review on GitHub (the fake records it), then the conversation, the list « Pull requests »
// (its « Ta relecture » chip gone) and the card « En revue » (« changements demandés ») read again, and the ticket's agent
// gets the changes asked (it commits « fix: retours de … »). And a review waiting through a restart of the app, its
// remark stale once a commit takes its line away, published as a comment of the pull request. ESCOUADE_E2E_SHOTS=<folder>:
// its screenshots, in both themes, at the size of ESCOUADE_E2E_VIEWPORT.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
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

type Asked = { method: string; path: string; body: Record<string, any> | null };
/** The reviews published on GitHub. */
const reviews = async () =>
  ((await (await call(`${github.url}/__writes`)).json()) as Asked[]).filter(
    (w) => w.method === 'POST' && /\/pulls\/\d+\/reviews$/.test(w.path),
  );
const setPull = (n: number, over: Record<string, unknown>) =>
  call(`${github.url}/__github/pulls/${n}`, { method: 'POST', body: JSON.stringify(over) });

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 9 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
});

function remote(app: { root: string; repo: string }) {
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  return bare;
}

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

/** The header of the view keeps its buttons on its line: none spills out of it (ESC-481). */
async function headerFits(view: Locator) {
  const spilled = await view.locator('.hd').evaluate((hd) => {
    const box = hd.getBoundingClientRect();
    return [...hd.querySelectorAll('.right > *')]
      .filter((el) => {
        const r = el.getBoundingClientRect();
        // Hidden at this width: nothing to spill.
        if (!r.width) return false;
        return r.right > box.right + 1 || r.left < box.left - 1;
      })
      .map(
        (el) =>
          `${el.getAttribute('aria-label') ?? el.textContent} (${Math.round(el.getBoundingClientRect().right)} > ${Math.round(box.right)})`,
      );
  });
  expect(spilled).toEqual([]);
}

/** « + » on line `n` of the tip (shown when the line is hovered), a remark written, « Ajouter à ma relecture ». */
async function addRemark(view: Locator, n: number, text: string) {
  await view.locator(`.row[data-new="${n}"]`).first().hover();
  await view
    .getByRole('button', { name: `Commenter la ligne ${n}` })
    .first()
    .click();
  const box = view.getByRole('textbox', { name: /^Commentaire sur/ });
  await box.fill(text);
  await view.getByRole('button', { name: 'Ajouter à ma relecture' }).click();
  await expect(box).toHaveCount(0, { timeout: 30_000 });
}

async function openList(page: Page) {
  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  const list = page.locator('main.prs');
  await expect(list.getByRole('heading', { name: 'Pull requests' })).toBeVisible();
  return list;
}

test('two remarks waiting, « Demander des changements » published in one review: the list, the card and the ticket’s agent follow', async ({
  app,
}) => {
  test.setTimeout(480_000);
  const { page } = app;
  const bare = remote(app);
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Ouvrir une pull request/ }).click();
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const card = col('En revue').locator('.card');
  await expect(card.locator('[data-ticket-pr="101"]')).toContainText('PR #101', { timeout: 60_000 });
  const log = () => git(bare, 'log', '--format=%s', 'ticket/dem-1');

  // Opened by marta's account on GitHub (not the user's own), a review asked of the user (ada); its head where GitHub keeps it.
  git(bare, 'update-ref', 'refs/pull/101/head', 'refs/heads/ticket/dem-1');
  await setPull(101, { author: 'marta', reviewers: ['ada'] });
  let list = await openList(page);
  const row = list.getByRole('listitem').filter({ hasText: '#101' });
  await expect(row).toContainText('Ta relecture', { timeout: 30_000 });
  await row.locator('button.main').click();
  const view = page.getByRole('main', { name: 'La PR #101' });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });

  // Two remarks under their line, kept waiting: nothing written on GitHub.
  await view.getByRole('tab', { name: /^Fichiers/ }).click();
  await expect(view.locator('.row[data-new="1"]').first()).toBeVisible({ timeout: 30_000 });
  await addRemark(view, 1, 'Ce **nom** est peu clair.');
  await addRemark(view, 1, 'Un test pour ce cas ?');
  const pending = view.locator('article.mine');
  await expect(pending).toHaveCount(2);
  await expect(pending.first()).toContainText('en attente · pas publiée');
  const button = view.getByRole('button', { name: 'Ta relecture, 2 remarques en attente' });
  await expect(button).toBeVisible();
  await headerFits(view);
  expect(await reviews()).toEqual([]);
  await shots(page, 'my-review-pending');

  // The panel: « Demander des changements », a text, the case for the ticket's agent checked.
  await button.click();
  const panel = view.getByRole('dialog', { name: 'Ta relecture de la PR #101' });
  await expect(panel).toContainText('2 remarques en attente');
  await panel.getByRole('radio', { name: /Demander des changements/ }).check();
  await expect(panel.getByRole('button', { name: 'Publier sur GitHub' })).toBeDisabled();
  await panel.getByRole('textbox', { name: 'Texte de ta relecture' }).fill('Deux points avant le merge.');
  await expect(panel.getByRole('checkbox', { name: 'Renvoyer aussi à l’agent du ticket' })).toBeChecked();
  await shots(page, 'my-review-panel');
  expect(await reviews()).toEqual([]);

  await panel.getByRole('button', { name: 'Publier sur GitHub' }).click();
  await expect(panel).toHaveCount(0, { timeout: 60_000 });
  const sent = await reviews();
  expect(sent).toHaveLength(1);
  expect(sent[0].path).toMatch(/\/repos\/acme\/demo\/pulls\/101\/reviews$/);
  expect(sent[0].body?.event).toBe('REQUEST_CHANGES');
  expect(sent[0].body?.body).toBe('Deux points avant le merge.');
  expect((sent[0].body?.comments as { body: string }[]).map((c) => c.body)).toEqual(['Ce **nom** est peu clair.', 'Un test pour ce cas ?']);
  expect(sent[0].body?.commit_id).toMatch(/^[0-9a-f]{40}$/);

  // The view read again: no remark waiting, the host's threads in their place, the review in the conversation.
  await expect(view.locator('article.mine')).toHaveCount(0);
  await expect(view.getByRole('button', { name: /remarques? en attente/ })).toHaveCount(0);
  await view.getByRole('tab', { name: /^Conversation/ }).click();
  await expect(view.getByText('a demandé des changements')).toBeVisible({ timeout: 30_000 });
  await shots(page, 'my-review-published');

  // The ticket's agent got the changes asked, by ESC-61's way: it commits and Escouade pushes.
  await expect.poll(log, { timeout: 90_000 }).toContain('fix: retours de DEM-1');

  // The list: « Ta relecture » off, the changes asked said.
  await view.getByRole('button', { name: 'Retour aux PR' }).click();
  list = page.locator('main.prs');
  await list.getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(row).not.toContainText('Ta relecture', { timeout: 60_000 });
  await expect(row).toContainText(/changements demandés/i);

  // The card « En revue »: changes asked.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card).toContainText(/changements demandés/i, { timeout: 60_000 });
});

const commit = (repo: string, message: string, files: Record<string, string>) => {
  for (const [p, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(repo, p)), { recursive: true });
    fs.writeFileSync(path.join(repo, p), text);
  }
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', message);
};

test('a review waits through a restart; a commit that takes its line away makes its remark stale, published in the text', async ({
  app,
}) => {
  test.setTimeout(420_000);
  let page = app.page;
  remote(app);
  git(app.repo, 'checkout', '-qb', 'feat/lien');
  commit(app.repo, 'feat: lien magique', {
    'src/lien.ts': 'export const ttl = 10;\nexport const lien = () => "x";\nexport const fin = 1;\n',
  });
  const push = () => git(app.repo, 'push', '-qf', 'origin', 'feat/lien', 'feat/lien:refs/pull/101/head');
  push();
  git(app.repo, 'checkout', '-q', 'main');
  const created = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title: 'Connexion par lien magique', body: 'Le lien.', head: 'feat/lien', base: 'main' }),
  });
  const { number: n } = (await created.json()) as { number: number };
  await setPull(n, { author: 'marta' });
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);

  const open = async (p: Page) => {
    const list = await openList(p);
    await list
      .getByRole('listitem')
      .filter({ hasText: `#${n}` })
      .locator('button.main')
      .click();
    const v = p.getByRole('main', { name: `La PR #${n}` });
    await expect(v.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });
    await v.getByRole('tab', { name: /^Fichiers/ }).click();
    return v;
  };
  let view = await open(page);
  await addRemark(view, 2, 'Pourquoi une fonction ?');
  await view.getByRole('button', { name: 'Ta relecture, 1 remarque en attente' }).click();
  await view.getByRole('textbox', { name: 'Texte de ta relecture' }).fill('Presque.');
  await page.waitForTimeout(1_000);
  await view.getByRole('button', { name: 'Fermer' }).last().click();

  // The app starts again: the review waits still, its text and its remark under its line.
  page = await app.restart();
  view = await open(page);
  const mine = view.locator('article.mine');
  await expect(mine).toHaveCount(1, { timeout: 30_000 });
  await expect(mine).toContainText('Pourquoi une fonction ?');
  await expect(mine).not.toContainText('dépassée');
  await view.getByRole('button', { name: 'Ta relecture, 1 remarque en attente' }).click();
  await expect(view.getByRole('textbox', { name: 'Texte de ta relecture' })).toHaveValue('Presque.');
  await view.getByRole('button', { name: 'Fermer' }).last().click();
  expect(await reviews()).toEqual([]);

  // A commit takes line 2 away: read again (past the 20 s the reading is kept), its remark is stale, apart in its file.
  git(app.repo, 'checkout', '-q', 'feat/lien');
  commit(app.repo, 'refactor: lien', { 'src/lien.ts': 'export const ttl = 10;\nexport const fin = 1;\n' });
  push();
  git(app.repo, 'checkout', '-q', 'main');
  // GitHub says it changed (a push moves its `updated_at`).
  await setPull(n, { updatedAt: '2026-11-17T12:00:00Z' });
  await page.waitForTimeout(21_000);
  await view.getByRole('button', { name: 'Relire la PR' }).click();
  await expect(mine).toContainText('dépassée', { timeout: 30_000 });
  await expect(view.getByRole('group', { name: 'Ta remarque dépassée : sa ligne n’est plus dans le diff' })).toBeVisible();
  await shots(page, 'my-review-stale');

  // Published as a comment: the stale remark goes in the review's text, with its place.
  await view.getByRole('button', { name: 'Ta relecture, 1 remarque en attente' }).click();
  const panel = view.getByRole('dialog', { name: `Ta relecture de la PR #${n}` });
  await expect(panel).toContainText('dépassée');
  await panel.getByRole('button', { name: 'Publier sur GitHub' }).click();
  await expect(panel).toHaveCount(0, { timeout: 60_000 });
  const sent = await reviews();
  expect(sent).toHaveLength(1);
  expect(sent[0].body?.event).toBe('COMMENT');
  expect(sent[0].body?.body).toBe('Presque.\n\n`src/lien.ts:2` : Pourquoi une fonction ?');
  expect(sent[0].body?.comments).toEqual([]);
});
