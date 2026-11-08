// Azure DevOps as a code host (ESC-241) on the real app, in the window of the CI runner (1024 × 728),
// against the fake Azure DevOps of tests/fixtures/fake-trackers.mjs: its organization connected with
// a PAT in Réglages › Services connectés, then an ordinary agent's pull request opened from the header of
// its conversation (pushed to a local remote that stands for dev.azure.com/acme/demo/_git/demo), its
// build policy and its reviewer read on its card and in the status bar, and the two corrections
// handed to the agent on a click.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings, settingsEntry } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let fake: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const writes = async (): Promise<{ method: string; path: string; body: any }[]> => (await fetch(`${fake.url}/__writes`)).json();
const PULLS = '/azure/acme/demo/_apis/git/repositories/demo/pullrequests';

test.beforeAll(async () => {
  fake = await startFakeTrackers(0);
  // Read by the app the fixture starts next: dev.azure.com's API is the fake one, and the pull
  // requests are read again at each selection of the agent.
  process.env.ESCOUADE_AZURE_DEVOPS_API = `${fake.url}/azure`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
});

test.beforeEach(async () => {
  await fetch(`${fake.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_AZURE_DEVOPS_API;
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
});

/** Connects the organization acme of dev.azure.com (the fake one's) with a PAT, in Réglages › Services connectés. */
async function connectAzure(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await settings.getByRole('button', { name: 'Connecter un hébergeur…' }).click();
  const form = settings.getByRole('form', { name: 'Connexion à un hébergeur' });
  await form.getByLabel('Hébergeur').selectOption('azure');
  await expect(form.getByLabel('Adresse')).toHaveAttribute('placeholder', 'dev.azure.com/acme');
  await form.getByLabel('Adresse').fill('dev.azure.com/acme');
  await form.getByLabel(/^Jeton/).fill('pat-azure');
  await form.getByRole('button', { name: 'Connecter' }).click();
  const row = settings.getByRole('group', { name: 'dev.azure.com' });
  await expect(row).toContainText('Azure DevOps · dev.azure.com/acme');
  await expect(row).toContainText('Connecté · ada@acme.dev');
  await expect(row).toContainText('Projet : ');
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);
}

/** The Kanban, then the agents again: the agent's conversation is shown anew, and its pull requests read again. */
async function showAgain(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await page.locator('.side .card').first().click();
}

test('an agent opens its pull request on Azure DevOps, and its card follows its build and its reviewer', async ({ app }) => {
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  // dev.azure.com/acme/demo/_git/demo, whose pushes land in a bare repository of the test.
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://dev.azure.com/acme/demo/_git/demo';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  const cards = page.locator('.side .card');
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  // No account yet: nothing to open.
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toHaveCount(0);
  await connectAzure(page);
  await showAgain(page);

  const worktrees = () =>
    git(app.repo, 'worktree', 'list', '--porcelain')
      .split('\n')
      .filter((l) => l.startsWith('worktree '))
      .map((l) => l.slice('worktree '.length))
      .slice(1);
  await expect.poll(() => worktrees().length).toBe(1);
  const wt = worktrees()[0];
  const branch = git(wt, 'branch', '--show-current');
  fs.writeFileSync(path.join(wt, 'login.ts'), 'export const login = 1;\n');
  git(wt, 'add', '-A');
  git(wt, 'commit', '-qm', 'feat: connexion par jeton');

  // « Ouvrir une PR » from the header: proposed, read, then pushed and opened as a draft.
  await page.getByRole('button', { name: 'Ouvrir une PR' }).click();
  const dialog = page.getByRole('dialog', { name: 'Ouvrir une PR sur Azure DevOps' });
  await expect(dialog.getByRole('textbox', { name: 'Titre' })).toHaveValue('Le travail de l’agent, proposé par le faux claude');
  await expect(dialog.getByRole('textbox', { name: 'Description' })).toHaveValue('- feat: connexion par jeton');
  await expect(dialog.getByText(`· ${branch} → main`)).toBeVisible();
  await dialog.getByRole('checkbox', { name: 'En brouillon' }).check();
  await dialog.getByRole('button', { name: 'Pousser et ouvrir' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText('PR #101 ouverte.')).toBeVisible();
  const created = (await writes()).filter((w) => w.path === PULLS);
  expect(created.map((w) => w.body)).toEqual([
    {
      sourceRefName: `refs/heads/${branch}`,
      targetRefName: 'refs/heads/main',
      title: 'Le travail de l’agent, proposé par le faux claude',
      description: '- feat: connexion par jeton',
      isDraft: true,
    },
  ]);
  expect(git(bare, 'log', '-1', '--format=%s', branch)).toBe('feat: connexion par jeton');
  // Its number in the header, under the card and beside its branch in the status bar.
  await expect(page.getByRole('button', { name: 'Ouvrir la PR #101 dans le navigateur (brouillon)' })).toHaveCount(3);
  await expect(page.locator('footer').getByRole('button', { name: 'Ouvrir la PR #101 dans le navigateur (brouillon)' })).toBeVisible();
  await expect(page.locator('footer .sync')).toContainText(branch);
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toHaveCount(0);

  // Its build policy fails and a reviewer waits for the author: read again when the agent is shown.
  await fetch(`${fake.url}/__azure/pulls/101`, {
    method: 'POST',
    body: JSON.stringify({ checks: 'FAILURE', changesRequested: true, draft: false }),
  });
  await showAgain(page);
  const prs = page.getByRole('group', { name: /^Pull requests de / });
  const num = prs.getByRole('button', { name: 'Ouvrir la PR #101 dans le navigateur (ouverte)' });
  await expect(num).toHaveText('#101');
  await expect(num.locator('[data-icon="close"]')).toHaveCount(1);
  await expect(page.locator('footer').getByRole('button', { name: 'Ouvrir la PR #101 dans le navigateur (ouverte)' })).toBeVisible();
  await expect(prs.getByRole('button', { name: 'L’agent corrige', exact: true })).toBeVisible();
  await expect(prs.getByRole('button', { name: 'L’agent corrige les retours' })).toBeVisible();

  // The failed build's log goes to the agent, on the click.
  await prs.getByRole('button', { name: 'L’agent corrige', exact: true }).click();
  await expect(
    page
      .locator('.conv')
      .getByText(/La CI de la PR #101 \(https:\/\/dev\.azure\.com\/acme\/demo\/_git\/demo\/pullrequest\/101\) échoue/)
      .first(),
  ).toBeVisible();
  await expect(
    page
      .locator('.conv')
      .getByText(/FAIL src\/app\.test\.ts/)
      .first(),
  ).toBeVisible();
  await expect(
    page
      .locator('.conv')
      .getByText(/Bonjour, tu as dit : La CI de la PR #101/)
      .first(),
  ).toBeVisible({ timeout: 30_000 });
  // Then the reviewer's thread.
  await prs.getByRole('button', { name: 'L’agent corrige les retours' }).click();
  await expect(
    page
      .locator('.conv')
      .getByText(/a des retours de relecture/)
      .first(),
  ).toBeVisible();
  await expect(
    page
      .locator('.conv')
      .getByText(/Ce nom est peu clair\./)
      .first(),
  ).toBeVisible();
});

// Azure Boards (ESC-242): the Kanban's source on the same Azure DevOps account, its project offered by the import,
// a work item imported with its Acceptance Criteria, then its state pushed and a comment written as its ticket moves.
test('Azure Boards takes the Azure DevOps account, its project offered by the import, and its work item follows its ticket (1024 × 728)', async ({
  app,
}) => {
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  git(app.repo, 'remote', 'add', 'origin', 'https://dev.azure.com/acme/demo/_git/demo');
  await addProject(page, app.repo, { firstAgent: false });

  // No source yet: the import names the seven, and leads to the project's page « Tickets importés »; with no account,
  // that page leads to « Services connectés ».
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Importer' }).click();
  const imp = page.getByRole('dialog', { name: 'Importer des tickets' });
  await expect(imp.getByText(/^Connecte Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards, puis/)).toBeVisible();
  await imp.getByRole('button', { name: 'Lier une source' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await expect(settingsEntry(settings, 'Tickets importés')).toHaveAttribute('aria-current', 'page');
  await settings.getByRole('button', { name: 'Ouvrir Services connectés' }).click();
  await expect(settings.getByRole('heading', { name: 'Sources de tickets' })).toBeVisible();

  // Azure Boards is connected through the code host's own form: one account for the PRs and the work items.
  const row = settings.locator('.acct').filter({ hasText: 'droit Work Items' });
  await expect(row).toContainText('Son jeton doit avoir le droit Work Items (lecture et écriture).');
  await row.getByRole('button', { name: 'Connecter Azure DevOps…' }).click();
  const form = settings.getByRole('form', { name: 'Connexion à un hébergeur' });
  await expect(form.getByLabel('Hébergeur')).toHaveValue('azure');
  await form.getByLabel('Adresse').fill('dev.azure.com/acme');
  await form.getByLabel(/^Jeton/).fill('pat-azure');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByRole('group', { name: 'dev.azure.com' })).toContainText('Sert aussi à Azure Boards');
  await expect(row).toContainText('Connecté · ada@acme.dev · acme');
  await closeSettings(settings);

  // The import offers the project's own Azure DevOps project, linked in one click.
  await page.getByRole('button', { name: 'Importer' }).click();
  await imp.getByRole('button', { name: 'Lier demo (Azure Boards)' }).click();
  await expect(imp.getByRole('tab', { name: /Azure Boards/ })).toHaveAttribute('aria-selected', 'true');
  await expect(imp.getByRole('checkbox', { name: /#101/ })).toContainText('✓ 2 critères détectés');
  await expect(imp.getByRole('checkbox', { name: /#102/ })).toBeVisible();
  // The closed one is not offered; « Assignés à moi » keeps Ada's, a type's chip its type's.
  await expect(imp.getByRole('checkbox', { name: /#103/ })).toHaveCount(0);
  await imp.getByRole('button', { name: 'Assignés à moi' }).click();
  await expect(imp.getByRole('checkbox', { name: /#102/ })).toHaveCount(0);
  await imp.getByRole('button', { name: 'Assignés à moi' }).click();
  await imp.getByRole('button', { name: 'Bug', exact: true }).click();
  await expect(imp.getByRole('checkbox', { name: /#101/ })).toHaveCount(0);
  await expect(imp.getByRole('checkbox', { name: /#102/ })).toBeVisible();
  await imp.getByRole('button', { name: 'Bug', exact: true }).click();
  await imp.getByRole('textbox', { name: 'Rechercher' }).fill('factures');
  await imp.getByRole('checkbox', { name: /#101/ }).click();
  await imp.getByRole('button', { name: '3', exact: true }).click();
  await imp.getByRole('button', { name: 'Importer 1 ticket' }).click();
  await expect(imp).toHaveCount(0);
  await expect(page.getByText('1 ticket importé depuis Azure Boards')).toBeVisible();

  // The fake claude meets criterion n at loop n: « À tester » after its second loop.
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByRole('button', { name: /Ouvrir #101/ })).toBeVisible({ timeout: 60_000 });
  const item = '/azure/acme/demo/_apis/wit/workitems/101';
  await expect.poll(async () => (await writes()).filter((w) => w.path === `${item}/comments`).length).toBe(1);
  const all = await writes();
  // « Active » when it starts (in JSON Patch); already so when it goes to test.
  expect(all.filter((w) => w.method === 'PATCH' && w.path === item).map((w) => w.body)).toEqual([
    [{ op: 'add', path: '/fields/System.State', value: 'Active' }],
  ]);
  const comment = all.find((w) => w.path === `${item}/comments`)!;
  expect(comment.body.text).toContain('est prêt à tester.');
  expect(comment.body.text).toContain('✓ 50 factures par page');
  // The work item is Active on Azure Boards.
  const now = await (await fetch(`${fake.url}${item}`)).json();
  expect(now.fields['System.State']).toBe('Active');
});
