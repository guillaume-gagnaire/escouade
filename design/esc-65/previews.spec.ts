// The previews of ESC-65 (README.md here): the real app on the fake GitHub of the end-to-end tests, 30 open pull requests
// on acme/demo, read by the app (`project_prs`, through the GitHub plugin); then the window drawn with each direction
// (directions.js) and captured. `npx playwright test -c design/esc-65/playwright.config.ts`.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from '../../tests/e2e/fixture';
import { startFakeTrackers } from '../../tests/fixtures/fake-trackers.mjs';

const HERE = import.meta.dirname;
let github: { url: string; close: () => Promise<void> };
const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

/** The pull requests of the demo: who opened them, when, their CI, reviews, conflict, fork. */
type Seed = {
  title: string;
  head: string;
  author: string;
  hours: number;
  checks?: 'SUCCESS' | 'FAILURE' | 'PENDING' | null;
  draft?: boolean;
  changesRequested?: boolean;
  approvals?: number;
  threads?: number;
  conflict?: boolean;
  reviewers?: string[];
  fork?: string;
};
const SEEDS: Seed[] = [
  { title: 'Pagination par curseur sur /v1/tickets', head: 'feat/cursor-pagination', author: 'ada', hours: 3, checks: 'PENDING', reviewers: ['bob'] },
  { title: 'Jeton d’accès : rotation sans coupure', head: 'feat/token-rotation', author: 'bob', hours: 5, checks: 'SUCCESS', approvals: 1, reviewers: ['ada'] },
  { title: 'Corrige le fuseau des dates d’échéance', head: 'fix/due-date-tz', author: 'chloe', hours: 8, checks: 'FAILURE' },
  { title: 'Traductions espagnoles de l’onboarding', head: 'i18n-es', author: 'marta', hours: 20, checks: 'SUCCESS', fork: 'marta', reviewers: ['ada'] },
  { title: 'Limite de débit par clé d’API', head: 'feat/rate-limit', author: 'ada', hours: 26, checks: 'SUCCESS', changesRequested: true, threads: 3 },
  { title: 'Migre les webhooks vers la file de travail', head: 'refactor/webhooks-queue', author: 'dmitri', hours: 30, checks: 'SUCCESS', approvals: 2 },
  { title: 'Export CSV des tickets filtrés', head: 'feat/csv-export', author: 'ada', hours: 44, checks: 'FAILURE', threads: 1 },
  { title: 'Index manquant sur tickets.project_id', head: 'perf/ticket-index', author: 'bob', hours: 50, checks: 'SUCCESS', approvals: 1, conflict: true },
  { title: 'Brouillon : recherche plein texte', head: 'spike/full-text', author: 'ada', hours: 60, checks: null, draft: true },
  { title: 'Corrige la double notification à l’assignation', head: 'fix/double-notify', author: 'eve', hours: 70, checks: 'SUCCESS', fork: 'eve', reviewers: ['ada', 'bob'] },
  { title: 'Met à jour axum en 0.8', head: 'deps/axum-0.8', author: 'renovate', hours: 80, checks: 'FAILURE' },
  { title: 'Champs personnalisés : type date', head: 'feat/custom-date-field', author: 'chloe', hours: 96, checks: 'SUCCESS', approvals: 2 },
  { title: 'Supprime l’ancien endpoint /v0/search', head: 'chore/drop-v0-search', author: 'dmitri', hours: 110, checks: 'SUCCESS', changesRequested: true, threads: 2 },
  { title: 'Journal d’audit des droits', head: 'feat/audit-log', author: 'ada', hours: 130, checks: 'SUCCESS', approvals: 1 },
  { title: 'Documentation OpenAPI des pièces jointes', head: 'docs/openapi-attachments', author: 'marta', hours: 150, checks: 'SUCCESS', fork: 'marta' },
  { title: 'Tests de charge du tableau Kanban', head: 'test/board-load', author: 'bob', hours: 170, checks: 'PENDING', reviewers: ['ada'] },
  { title: 'SSO SAML pour les comptes équipe', head: 'feat/saml-sso', author: 'chloe', hours: 200, checks: 'FAILURE', conflict: true, threads: 4 },
  { title: 'Archive les projets inactifs', head: 'feat/archive-projects', author: 'dmitri', hours: 230, checks: 'SUCCESS' },
  { title: 'Brouillon : API GraphQL', head: 'spike/graphql', author: 'bob', hours: 260, checks: null, draft: true },
  { title: 'Corrige le tri des colonnes personnalisées', head: 'fix/custom-sort', author: 'eve', hours: 300, checks: 'SUCCESS', approvals: 1 },
  { title: 'Met à jour sqlx en 0.9', head: 'deps/sqlx-0.9', author: 'renovate', hours: 330, checks: 'SUCCESS' },
  { title: 'Réponses par e-mail sur les tickets', head: 'feat/email-replies', author: 'ada', hours: 380, checks: 'SUCCESS', conflict: true },
  { title: 'Clés étrangères en cascade sur les étiquettes', head: 'fix/label-cascade', author: 'chloe', hours: 420, checks: 'SUCCESS' },
  { title: 'Cache des permissions par requête', head: 'perf/perm-cache', author: 'dmitri', hours: 480, checks: 'SUCCESS', changesRequested: true },
  { title: 'Thème clair du portail public', head: 'feat/portal-light', author: 'marta', hours: 540, checks: 'SUCCESS', fork: 'marta' },
  { title: 'Brouillon : import depuis Jira', head: 'feat/jira-import', author: 'chloe', hours: 600, checks: null, draft: true },
  { title: 'Retire la dépendance à moment.js', head: 'chore/drop-moment', author: 'bob', hours: 700, checks: 'SUCCESS', approvals: 1 },
  { title: 'Compteurs du tableau de bord en temps réel', head: 'feat/live-counters', author: 'eve', hours: 800, checks: 'FAILURE' },
  { title: 'Sauvegarde chiffrée des pièces jointes', head: 'feat/encrypted-backup', author: 'dmitri', hours: 900, checks: 'SUCCESS' },
  { title: 'Lien de partage public d’un ticket', head: 'feat/share-link', author: 'chloe', hours: 1100, checks: 'SUCCESS', approvals: 2 },
];

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 1 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
});
test.afterAll(async () => {
  await github?.close();
});

async function connectGithub(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Intégrations' }).click();
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await form.getByLabel(/Jeton/).fill('ghp_jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
}

test('the three directions of the « Pull requests » view', async ({ app }) => {
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 });
  // The seeds, oldest first: the fake gives them the most recent first.
  for (const s of [...SEEDS].reverse()) {
    const r = await fetch(`${github.url}/github/repos/acme/demo/pulls`, {
      method: 'POST',
      body: JSON.stringify({ title: s.title, body: '', head: s.head, base: 'main', draft: s.draft }),
    });
    const { number } = await r.json();
    await fetch(`${github.url}/__github/pulls/${number}`, {
      method: 'POST',
      body: JSON.stringify({
        author: s.author,
        createdAt: new Date(Date.now() - s.hours * 3600_000).toISOString(),
        checks: s.checks,
        changesRequested: !!s.changesRequested,
        approvals: s.approvals ?? 0,
        threads: s.threads,
        mergeable: s.conflict ? 'CONFLICTING' : 'MERGEABLE',
        reviewers: s.reviewers ?? [],
        headRepo: s.fork ? `${s.fork}/demo` : undefined,
      }),
    });
  }

  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  git(app.repo, 'remote', 'add', 'origin', 'https://github.com/acme/demo.git');
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, 'https://github.com/acme/demo.git');
  git(app.repo, 'push', '-q', 'origin', 'main');
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);
  await page.locator('.side .seg button', { hasText: 'Kanban' }).click();

  // What the app reads: the GitHub plugin's open pull requests of acme/demo.
  const id = JSON.parse(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).projects[0].id as string;
  const read = await page.evaluate(
    (projectId) =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<any> } }).__TAURI_INTERNALS__.invoke(
        'project_prs',
        { projectId },
      ),
    id,
  );
  const prs = read.data.prs as any[];
  expect(prs.length).toBe(30);
  // What the app's PullRequest does not carry yet (phase 2): whose review is asked; and an agent of the project on one.
  const byTitle = new Map(SEEDS.map((s) => [s.title, s]));
  for (const p of prs) p.reviewers = byTitle.get(p.title)?.reviewers ?? [];
  prs.find((p) => p.title.startsWith('Limite de débit')).agent = 'rate-limit';
  fs.writeFileSync(path.join(HERE, 'data.json'), JSON.stringify(read, null, 2));

  // A string evaluated through the DevTools protocol: the window's CSP refuses an inline script.
  await page.evaluate(fs.readFileSync(path.join(HERE, 'directions.js'), 'utf8'));
  const fork = prs.find((p) => p.fork)!.number;
  const shot = async (name: string, o: Record<string, unknown>, theme: 'dark' | 'light' = 'dark') => {
    await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
    await page.evaluate(([list, opts]) => (window as any).__esc65(list, opts), [prs, o] as const);
    await page.mouse.move(1438, 450);
    await page.waitForTimeout(400);
    await page.screenshot({ path: path.join(HERE, `${name}.png`), animations: 'disabled', caret: 'hide' });
  };

  for (const dir of ['a', 'b', 'c']) {
    await shot(`${dir}-toutes`, { dir, filter: 'all', selected: fork });
    await shot(`${dir}-toutes-clair`, { dir, filter: 'all', selected: fork }, 'light');
    await shot(`${dir}-miennes`, { dir, filter: 'mine', selected: prs.find((p) => p.author === 'ada').number });
    await shot(`${dir}-relecture`, { dir, filter: 'review', selected: fork });
  }
  // The states, the same in the three directions (header and message): drawn in A, one pull request in each.
  await shot('etat-chargement', { dir: 'a', filter: 'all', state: 'loading' });
  await shot('etat-erreur', { dir: 'a', filter: 'all', state: 'error' });
  await shot('etat-vide', { dir: 'a', filter: 'all', state: 'empty' });
  await shot('etat-vide-relecture', { dir: 'a', filter: 'review', state: 'one' });
  for (const dir of ['a', 'b', 'c']) await shot(`${dir}-une`, { dir, filter: 'all', state: 'one', selected: prs[0].number });
});
