// The external ticket systems on the real app, against the fake Jira, Trello, GitHub, ClickUp and GitLab of
// tests/fixtures/fake-trackers.mjs: an account connected, a source linked, tickets imported, and
// their external ones moved and commented while the fake `claude` works them.

import { execFileSync } from 'node:child_process';
import { addProject, expect, test, openSettingsPage, closeSettings, settingsEntry } from './fixture';
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
  process.env.ESCOUADE_CLICKUP_API = `${trackers.url}/clickup/api/v2`;
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
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
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

  // Sources, on the project's page: each one's default states, Trello's « Terminé » set by hand.
  await openSettingsPage(settings, 'Tickets importés');
  await settings.getByRole('combobox', { name: 'Projet Jira' }).selectOption('ATL');
  await expect(settings.getByRole('combobox', { name: 'Jira — En cours' })).toHaveValue('3');
  await expect(settings.getByRole('combobox', { name: 'Jira — Terminé' })).toHaveValue('10');
  await settings.getByRole('combobox', { name: 'Tableau Trello' }).selectOption('b1');
  await expect(settings.getByRole('combobox', { name: 'Trello — En cours' })).toHaveValue('l2');
  await expect(settings.getByRole('combobox', { name: 'Trello — Terminé' })).toHaveValue('l3');
  await settings.getByRole('switch', { name: "Commenter à l'arrivée dans « En cours »" }).click();
  await closeSettings(settings);
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

test('a ClickUp task is imported with its checklist, and its status follows its ticket (1024 × 728)', async ({ app }) => {
  const { page } = app;
  // The runner's screen is 1024 × 768.
  await page.setViewportSize({ width: 1024, height: 728 });
  await addProject(page, app.repo, { firstAgent: false });

  // The account: a key refused, then the right one; the user's name said.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await settings.getByRole('button', { name: 'Connecter…' }).nth(4).click();
  const form = settings.getByRole('form', { name: 'Connexion à ClickUp' });
  await form.getByLabel("Clé d'API").fill('bad');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(form.getByRole('alert')).toHaveText('ClickUp refuse ces identifiants (401) — Token invalid');
  await form.getByLabel("Clé d'API").fill('pk_183_jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · Ada Lovelace')).toBeVisible();

  // The list, on the project's page, by space and folder; the defaults guessed from the list's own statuses.
  await openSettingsPage(settings, 'Tickets importés');
  const source = settings.getByRole('combobox', { name: 'Liste ClickUp' });
  await expect(source.locator('optgroup[label="Produit › Sprints"] option')).toHaveText(['Sprint 12']);
  await source.selectOption('l1');
  await expect(settings.getByRole('combobox', { name: 'ClickUp — En cours' })).toHaveValue('in progress');
  await expect(settings.getByRole('combobox', { name: 'ClickUp — Terminé' })).toHaveValue('review');
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);

  // The import: the open tasks, the finished one left out; « Mes tâches » and a custom id.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Importer' }).click();
  const imp = page.getByRole('dialog', { name: 'Importer des tickets' });
  await expect(imp.getByRole('checkbox', { name: /ABC-12/ })).toContainText('✓ 2 critères détectés');
  await expect(imp.getByRole('checkbox', { name: /ABC-13/ })).toBeVisible();
  await expect(imp.getByRole('checkbox', { name: /ABC-9/ })).toHaveCount(0);
  await imp.getByRole('button', { name: 'Mes tâches' }).click();
  await expect(imp.getByRole('checkbox', { name: /ABC-13/ })).toHaveCount(0);
  await imp.getByRole('button', { name: 'Mes tâches' }).click();
  await imp.getByRole('textbox', { name: 'Rechercher' }).fill('ABC-12');
  await expect(imp.getByRole('checkbox', { name: /ABC-13/ })).toHaveCount(0);
  await imp.getByRole('checkbox', { name: /ABC-12/ }).click();
  await imp.getByRole('button', { name: '3', exact: true }).click();
  await imp.getByRole('button', { name: 'Importer 1 ticket' }).click();
  await expect(imp).toHaveCount(0);
  await expect(page.getByText('1 ticket importé depuis ClickUp')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Ouvrir ABC-12 dans ClickUp' })).toBeVisible();

  // The fake claude meets criterion n at loop n: « À tester » after its second loop.
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByRole('button', { name: /Ouvrir ABC-12/ })).toBeVisible({ timeout: 60_000 });
  await expect.poll(async () => (await writes()).filter((w) => w.path.endsWith('/comment')).length).toBe(1);
  const all = await writes();
  // « in progress » when it starts and when it goes to test (its default for « À tester »).
  expect(all.filter((w) => w.method === 'PUT' && w.path === '/clickup/api/v2/task/86abx3').map((w) => w.body.status)).toEqual([
    'in progress',
    'in progress',
  ]);
  const comment = all.find((w) => w.path === '/clickup/api/v2/task/86abx3/comment')!.body;
  expect(comment.comment_text).toContain('est prêt à tester.');
  expect(comment.comment_text).toContain('✓ Filtre par date');
  // The task's status has changed in ClickUp.
  const task = await (await fetch(`${trackers.url}/clickup/api/v2/task/86abx3`)).json();
  expect(task.status.status).toBe('in progress');
});

test('the automatic import brings the GitHub issues that carry its label', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const github = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await github.getByLabel(/Jeton/).fill('ghp_jeton');
  await github.getByRole('button', { name: 'Connecter' }).click();
  // Issues' account, and github.com's among the code hosts: the same one.
  await expect(settings.getByText('Connecté · @ada')).toHaveCount(2);
  await expect(settings.getByRole('group', { name: 'github.com' })).toContainText('Sert aussi à GitHub Issues');
  // The automatic import is Escouade's, the source the project's.
  await settings.getByRole('switch', { name: 'Importer les tickets étiquetés' }).click();
  await settings.getByRole('button', { name: '5 min', exact: true }).click();
  await openSettingsPage(settings, 'Tickets importés');
  await settings.getByRole('combobox', { name: 'Dépôt GitHub Issues' }).selectOption('acme/demo');
  await expect(settings.getByRole('combobox', { name: 'GitHub Issues — En cours' })).toHaveValue('label:in-progress');
  await expect(settings.getByRole('combobox', { name: 'GitHub Issues — Terminé' })).toHaveValue('label:à tester');
  // Kept « À faire »: the autopilot is turned off first.
  await openSettingsPage(settings, 'Kanban');
  await settings.getByRole('switch', { name: 'Attribuer les tickets automatiquement' }).click();
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  // The first pass comes 30 s after the start.
  const todo = page.getByRole('region', { name: 'À faire' });
  await expect(todo.getByRole('button', { name: 'Ouvrir #42 dans GitHub Issues' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByText(/1 ticket importé depuis GitHub dans demo-api/)).toBeVisible();
  await expect(todo.getByRole('button', { name: /#43/ })).toHaveCount(0);
});

test('the self-hosted GitLab of a project is connected from its remote, and the rights its token lacks are listed', async ({ app }) => {
  const { page } = app;
  // Its remote is on an instance no account is connected for (the fake GitLab).
  execFileSync('git', ['remote', 'add', 'origin', `${trackers.url}/acme/web/app.git`], { cwd: app.repo, stdio: 'pipe' });
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await expect(settings.getByRole('heading', { name: 'Hébergeurs de code' })).toBeVisible();
  await expect(settings.getByRole('group', { name: '127.0.0.1' })).toContainText('Aucun compte, pour le projet demo-api');

  // « Connecter <hôte> »: its address from the remote, its kind asked of it.
  await settings.getByRole('button', { name: 'Connecter 127.0.0.1', exact: true }).click();
  const form = settings.getByRole('form', { name: 'Connexion à un hébergeur' });
  await expect(form.getByLabel('Hébergeur')).toHaveValue('gitlab');
  await expect(form.getByLabel('Adresse')).toHaveValue(trackers.url);
  await expect(form.getByText('Droits à donner au jeton : api')).toBeVisible();
  await form.getByLabel(/^Jeton/).fill('bad');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(form.getByRole('alert')).toHaveText('GitLab refuse ces identifiants (401) — Bad credentials');
  await form.getByLabel(/^Jeton/).fill('glpat-jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();

  // Connected, the right it lacks said, the project on it.
  const host = new URL(trackers.url).host;
  const account = settings.getByRole('group', { name: host });
  await expect(account).toContainText('GitLab');
  await expect(account).toContainText('Droits manquants : api');
  await expect(account).toContainText('Projet : demo-api');
  await expect(settings.getByRole('button', { name: 'Connecter 127.0.0.1', exact: true })).toHaveCount(0);

  // Disconnected: the host is back without an account.
  await settings.getByRole('button', { name: `Déconnecter ${host}` }).click();
  await expect(settings.getByRole('button', { name: 'Connecter 127.0.0.1', exact: true })).toBeVisible();
});

test('GitLab Issues takes the GitLab code host’s account, its project offered by the import, and its issue follows its ticket (1024 × 728)', async ({
  app,
}) => {
  const { page } = app;
  // The runner's screen is 1024 × 768.
  await page.setViewportSize({ width: 1024, height: 728 });
  // Its remote is on the self-hosted GitLab (the fake one).
  execFileSync('git', ['remote', 'add', 'origin', `${trackers.url}/acme/api.git`], { cwd: app.repo, stdio: 'pipe' });
  await addProject(page, app.repo, { firstAgent: false });

  // No source yet: the import names the seven, and leads to the project's page « Tickets importés »; with no account,
  // that page leads to « Services connectés ».
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Importer' }).click();
  const imp = page.getByRole('dialog', { name: 'Importer des tickets' });
  await expect(imp.getByText('Aucune source liée à ce projet')).toBeVisible();
  await expect(
    imp.getByText(/^Connecte Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards, puis choisis/),
  ).toBeVisible();
  await imp.getByRole('button', { name: 'Lier une source' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await expect(settingsEntry(settings, 'Tickets importés')).toHaveAttribute('aria-current', 'page');
  await expect(settings.getByRole('heading', { name: 'Sources liées à demo-api' })).toBeVisible();
  await settings.getByRole('button', { name: 'Ouvrir Services connectés' }).click();
  await expect(settingsEntry(settings, 'Services connectés')).toHaveAttribute('aria-current', 'page');
  await expect(settings.getByRole('heading', { name: 'Hébergeurs de code' })).toBeVisible();

  // GitLab Issues is connected through the code host's own form: one account for both uses.
  await settings.getByRole('button', { name: 'Connecter GitLab…' }).click();
  const form = settings.getByRole('form', { name: 'Connexion à un hébergeur' });
  await expect(form.getByLabel('Hébergeur')).toHaveValue('gitlab');
  await form.getByLabel('Adresse').fill(trackers.url);
  await form.getByLabel(/^Jeton/).fill('glpat-e2e');
  await form.getByRole('button', { name: 'Connecter' }).click();
  const host = new URL(trackers.url).host;
  await expect(settings.getByRole('group', { name: host })).toContainText('Sert aussi à GitLab Issues');
  await expect(settings.getByText(`Connecté · @ada · ${host}`)).toBeVisible();
  await closeSettings(settings);

  // The import offers the project's own GitLab project, linked in one click.
  await page.getByRole('button', { name: 'Importer' }).click();
  await imp.getByRole('button', { name: 'Lier acme/api (GitLab Issues)' }).click();
  await expect(imp.getByRole('tab', { name: /GitLab Issues/ })).toHaveAttribute('aria-selected', 'true');
  await expect(imp.getByRole('checkbox', { name: /#7/ })).toContainText('✓ 2 critères détectés');
  await expect(imp.getByRole('checkbox', { name: /#8/ })).toBeVisible();
  // The closed one is not offered; « Assignées à moi » keeps Ada's.
  await expect(imp.getByRole('checkbox', { name: /#3/ })).toHaveCount(0);
  await imp.getByRole('button', { name: 'Assignées à moi' }).click();
  await expect(imp.getByRole('checkbox', { name: /#7/ })).toHaveCount(0);
  await imp.getByRole('button', { name: 'Assignées à moi' }).click();
  await imp.getByRole('textbox', { name: 'Rechercher' }).fill('factures');
  await imp.getByRole('checkbox', { name: /#7/ }).click();
  await imp.getByRole('button', { name: '3', exact: true }).click();
  await imp.getByRole('button', { name: 'Importer 1 ticket' }).click();
  await expect(imp).toHaveCount(0);
  await expect(page.getByText('1 ticket importé depuis GitLab')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Ouvrir #7 dans GitLab Issues' })).toBeVisible();

  // The fake claude meets criterion n at loop n: « À tester » after its second loop.
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByRole('button', { name: /Ouvrir #7/ })).toBeVisible({ timeout: 60_000 });
  const issue = '/api/v4/projects/acme%2Fapi/issues/7';
  await expect.poll(async () => (await writes()).filter((w) => w.path === `${issue}/notes`).length).toBe(1);
  const all = await writes();
  // « Doing », a list of its board, when it starts; already there when it goes to test.
  expect(all.filter((w) => w.method === 'PUT' && w.path === issue).map((w) => w.body)).toEqual([{ add_labels: 'Doing' }]);
  const note = all.find((w) => w.path === `${issue}/notes`)!.body;
  expect(note.body).toContain('est prêt à tester.');
  expect(note.body).toContain('✓ 50 factures par page');
  // The issue carries it in GitLab.
  const now = await (await fetch(`${trackers.url}${issue}`, { headers: { 'PRIVATE-TOKEN': 'x' } })).json();
  expect(now.labels).toEqual(['claude-ready', 'Doing']);
});
