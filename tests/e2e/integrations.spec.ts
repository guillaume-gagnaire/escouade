// The external ticket systems on the real app, against the fake Jira, Trello and GitHub of
// tests/fixtures/fake-trackers.mjs: an account connected, a source linked, tickets imported, and
// their external ones moved and commented while the fake `claude` works them.

import { addProject, expect, test } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let trackers: { url: string; close: () => Promise<void> };

interface Write {
  method: string;
  path: string;
  query: Record<string, string>;
  body: any;
}

const writes = async (): Promise<Write[]> => (await fetch(`${trackers.url}/__writes`)).json();

test.beforeAll(async () => {
  trackers = await startFakeTrackers(0);
  // Read by the app the fixture starts next.
  process.env.ESCOUADE_TRELLO_API = `${trackers.url}/1`;
  process.env.ESCOUADE_GITHUB_API = `${trackers.url}/github`;
});

test.beforeEach(async () => {
  await fetch(`${trackers.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await trackers?.close();
});

test('a Jira issue and a Trello card are imported, and their states follow their tickets', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });

  // Accounts: refused, then accepted.
  await page.getByTitle('Réglages (Ctrl+,)').click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Intégrations' }).click();
  await settings.getByRole('button', { name: 'Connecter…' }).first().click();
  const jira = settings.getByRole('form', { name: 'Connexion à Jira' });
  await jira.getByLabel('Site').fill(trackers.url);
  await jira.getByLabel('E-mail').fill('ada@atlas.dev');
  await jira.getByLabel("Jeton d'API").fill('bad');
  await jira.getByRole('button', { name: 'Connecter' }).click();
  await expect(jira.getByRole('alert')).toHaveText('Jira refuse ces identifiants (401) — Bad credentials');
  await jira.getByLabel("Jeton d'API").fill('jeton');
  await jira.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText(/Connecté · ada@atlas\.dev · 127\.0\.0\.1:/)).toBeVisible();
  await settings.getByRole('button', { name: 'Connecter…' }).first().click();
  const trello = settings.getByRole('form', { name: 'Connexion à Trello' });
  await trello.getByLabel("Clé d'API").fill('cle');
  await trello.getByLabel('Jeton').fill('jeton');
  await trello.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada')).toBeVisible();

  // Sources: each one's default states, Trello's « Terminé » set by hand.
  await settings.getByRole('combobox', { name: 'Projet Jira' }).selectOption('ATL');
  await expect(settings.getByRole('combobox', { name: 'Jira — En cours' })).toHaveValue('3');
  await expect(settings.getByRole('combobox', { name: 'Jira — Terminé' })).toHaveValue('10');
  await settings.getByRole('combobox', { name: 'Tableau Trello' }).selectOption('b1');
  await expect(settings.getByRole('combobox', { name: 'Trello — En cours' })).toHaveValue('l2');
  await expect(settings.getByRole('combobox', { name: 'Trello — Terminé' })).toHaveValue('l3');
  await settings.getByRole('switch', { name: "Commenter à l'arrivée dans « En cours »" }).click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);

  // The import: one ticket of each source.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Importer' }).click();
  const imp = page.getByRole('dialog', { name: 'Importer des tickets' });
  const issue = imp.getByRole('checkbox', { name: /ATL-1287/ });
  await expect(issue).toContainText('✓ 2 critères détectés');
  await issue.click();
  await imp.getByRole('tab', { name: /Trello/ }).click();
  await imp.getByRole('checkbox', { name: /#151/ }).click();
  await imp.getByRole('button', { name: '3', exact: true }).click();
  await imp.getByRole('button', { name: 'Importer 2 tickets' }).click();
  await expect(imp).toHaveCount(0);
  await expect(page.getByText('2 tickets importés depuis Jira et Trello')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Ouvrir ATL-1287 dans Jira' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Ouvrir #151 dans Trello' })).toBeVisible();

  // The fake claude meets criterion n at loop n: both go « À tester » after their second loop.
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByRole('button', { name: /Ouvrir ATL-1287/ })).toBeVisible({ timeout: 60_000 });
  await expect(review.getByRole('button', { name: /Ouvrir #151/ })).toBeVisible({ timeout: 60_000 });
  await expect.poll(async () => (await writes()).filter((w) => w.path.endsWith('/comment') || w.path.endsWith('/comments')).length).toBe(4);
  const all = await writes();
  // Jira: « In Progress » when it starts (once: it is there when it goes to test).
  expect(all.filter((w) => w.path === '/rest/api/3/issue/ATL-1287/transitions').map((w) => w.body.transition.id)).toEqual(['t3']);
  // Trello: the card in « En cours ».
  expect(all.filter((w) => w.path === '/1/cards/c151').map((w) => w.query.idList)).toEqual(['l2', 'l2']);
  const jiraComments = all.filter((w) => w.path === '/rest/api/3/issue/ATL-1287/comment').map((w) => JSON.stringify(w.body.body));
  expect(jiraComments[0]).toContain('est pris par un agent (branche ticket/');
  expect(jiraComments[1]).toContain('est prêt à tester.');
  expect(jiraComments[1]).toContain('✓ Refresh avant expiration');
  const trelloComments = all.filter((w) => w.path === '/1/cards/c151/actions/comments').map((w) => w.body.text);
  expect(trelloComments[1]).toContain('✓ Colonnes horodatage, IP, résultat');
});

test('the automatic import brings the GitHub issues that carry its label', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByTitle('Réglages (Ctrl+,)').click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Intégrations' }).click();
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const github = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await github.getByLabel(/Jeton/).fill('ghp_jeton');
  await github.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada')).toBeVisible();
  await settings.getByRole('combobox', { name: 'Dépôt GitHub Issues' }).selectOption('acme/demo');
  await expect(settings.getByRole('combobox', { name: 'GitHub Issues — En cours' })).toHaveValue('label:in-progress');
  await expect(settings.getByRole('combobox', { name: 'GitHub Issues — Terminé' })).toHaveValue('label:à tester');
  await settings.getByRole('switch', { name: 'Importer les tickets étiquetés' }).click();
  await settings.getByRole('button', { name: '5 min', exact: true }).click();
  // Kept « À faire »: the autopilot is turned off first.
  await settings.getByRole('tab', { name: 'Kanban' }).click();
  await settings.getByRole('switch', { name: 'Attribuer les tickets automatiquement' }).click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  // The first pass comes 30 s after the start.
  const todo = page.getByRole('region', { name: 'À faire' });
  await expect(todo.getByRole('button', { name: 'Ouvrir #42 dans GitHub Issues' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByText(/1 ticket importé depuis GitHub dans demo-api/)).toBeVisible();
  await expect(todo.getByRole('button', { name: /#43/ })).toHaveCount(0);
});
