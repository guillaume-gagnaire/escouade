import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, send, test } from './fixture';

test('a new project opens an agent that streams Claude’s reply', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await expect(page.getByRole('button', { name: /demo-api/ }).first()).toBeVisible();
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await expect(page.getByText('Tâche terminée')).toBeVisible();
  await expect(page.locator('footer')).toContainText('1 terminé');
  // Named from the first request (the fake CLI answers the naming call).
  await expect(page.locator('.card .name').first()).toHaveText('bonjour-fake');
  // The quota of the 5-hour window, as the fake CLI reports it: a meter, whose percentage is in its tooltip.
  await expect(page.locator('footer')).toContainText('5h');
  await expect(page.getByRole('meter', { name: 'Quota sur 5 heures' })).toHaveAttribute('aria-valuenow', '12');
});

test('the model picker names the version Claude Code runs for each model', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  // The fake CLI reports Sonnet by its alias, Fable only by its full id, as Claude Code 2.1.284.
  await page.getByRole('button', { name: 'Modèle : Sonnet 5.5' }).click();
  await expect(page.getByRole('menuitemradio', { name: /Fable 5\.1/ })).toBeVisible();
});

test('a question from Claude is signalled and answered in one click', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'question');
  const card = page.getByTestId('question-pending');
  await expect(card).toContainText('Quelle base de données ?');
  await expect(page.locator('footer')).toContainText('1 en attente');
  await expect(page.locator('.tab .pill')).toHaveText('1');
  await card.getByRole('button', { name: 'SQLite' }).click();
  await expect(page.getByText('Choix retenu : SQLite')).toBeVisible();
  await expect(page.locator('footer')).toContainText('0 en attente');
  await expect(page.getByText('→ SQLite')).toBeVisible();
});

test('a permission can be refused with an explanation typed in the composer', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'permission');
  await expect(page.getByTestId('permission-pending')).toContainText('rm -rf build');
  // For 500 ms after a request appears, Entrée in the composer answers nothing (HOLD_MS): a press
  // meant for a message would answer a request not read yet.
  await page.waitForTimeout(600);
  await send(page, 'utilise npm run clean');
  await expect(page.getByText('Compris : utilise npm run clean')).toBeVisible();
  await expect(page.getByText('✕ Refusé · Bash rm -rf build')).toBeVisible();
});

test('uncommitted changes show up in the tab counter, the files panel and the diff', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'const a = 2;\nconst b = 3;\n');
  await expect(page.locator('.tab .delta')).toHaveText('Δ 1');
  await page.getByRole('button', { name: /Fichiers/ }).click();
  await page.getByRole('button', { name: 'Tout le projet' }).click();
  await expect(page.locator('.panel')).toContainText('app.ts');
  // The row's own button, not its `</>` editor button (whose name also holds the file name).
  await page.locator('.filerow .file').filter({ hasText: 'app.ts' }).click();
  const diff = page.getByRole('dialog', { name: 'src/app.ts' });
  await expect(diff).toContainText('const b = 3;');
  await page.keyboard.press('Escape');
  await expect(diff).toBeHidden();
});

test('a changed file is reverted or deleted from its context menu', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'const a = 2;\n');
  fs.writeFileSync(path.join(app.repo, 'notes.md'), 'brouillon\n');
  await page.getByRole('button', { name: /Fichiers/ }).click();
  await page.getByRole('button', { name: 'Tout le projet' }).click();
  const panel = page.locator('.panel');
  // The row's own button, not its `</>` editor button (whose name also holds the file name).
  const file = (name: RegExp) => panel.locator('.filerow .file').filter({ hasText: name });

  await file(/app\.ts/).click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Abandonner les modifications…' }).click();
  await page
    .getByRole('dialog', { name: 'Abandonner les modifications de « app.ts » ?' })
    .getByRole('button', { name: 'Abandonner les modifications' })
    .click();
  await expect(file(/app\.ts/)).toBeHidden();
  expect(fs.readFileSync(path.join(app.repo, 'src', 'app.ts'), 'utf8')).toBe('const a = 1;\n');

  await file(/notes\.md/).click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Supprimer le fichier…' }).click();
  await page.getByRole('dialog', { name: 'Supprimer « notes.md » ?' }).getByRole('button', { name: 'Supprimer' }).click();
  await expect(panel).toContainText('Aucune modification non commitée.');
  expect(fs.existsSync(path.join(app.repo, 'notes.md'))).toBe(false);
});

test('the status bar shows what there is to pull from the remote, fetched and pulled from the branch picker', async ({ app }) => {
  const { page } = app;
  const git = (cwd: string, ...args: string[]) =>
    execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', ...args], { cwd, stdio: 'pipe' });
  const root = path.dirname(app.repo);
  const remote = path.join(root, 'remote.git');
  git(root, 'init', '-q', '--bare', '-b', 'main', remote);
  git(app.repo, 'remote', 'add', 'origin', remote);
  git(app.repo, 'push', '-q', '-u', 'origin', 'main');
  await addProject(page, app.repo);
  const sync = page.locator('footer .sync');
  await expect(sync).toContainText('⎇ main');
  await expect(sync).toContainText('↓0');

  // Someone else pushes a commit.
  const other = path.join(root, 'other');
  git(root, 'clone', '-q', remote, other);
  fs.writeFileSync(path.join(other, 'README.md'), 'depuis ailleurs\n');
  git(other, 'add', '-A');
  git(other, 'commit', '-qm', 'readme');
  git(other, 'push', '-q');

  await sync.click();
  await page.getByRole('dialog', { name: 'Branches' }).getByRole('button', { name: /Fetch/ }).click();
  await expect(sync).toContainText('↓1');
  await sync.click();
  await page
    .getByRole('dialog', { name: 'Branches' })
    .getByRole('button', { name: /Récupérer/ })
    .click();
  await expect(sync).toContainText('↓0');
  expect(fs.readFileSync(path.join(app.repo, 'README.md'), 'utf8')).toBe('depuis ailleurs\n');
});

test('a PDF attached to a message reaches Claude as a document', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const pdf = Buffer.from('%PDF-1.4 rapport');
  await page.locator('input[type=file]').setInputFiles({ name: 'rapport.pdf', mimeType: 'application/pdf', buffer: pdf });
  await expect(page.getByText('rapport.pdf')).toBeVisible();
  await send(page, 'Résume ce rapport');
  await expect(page.getByText('Bonjour, tu as dit : Résume ce rapport')).toBeVisible();
  // What the CLI read on its stdin.
  const stdin = path.join(path.dirname(app.data), 'fake-claude.stdin.jsonl');
  const sent = fs
    .readFileSync(stdin, 'utf8')
    .trim()
    .split('\n')
    .map((l) => JSON.parse(l))
    .find((m) => Array.isArray(m.message?.content));
  expect(sent.message.content).toEqual([
    { type: 'document', title: 'rapport.pdf', source: { type: 'base64', media_type: 'application/pdf', data: pdf.toString('base64') } },
    { type: 'text', text: 'Résume ce rapport' },
  ]);
});

test('the status bar counts the running Claude processes, with their memory and CPU', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  // The first agent's process starts with the project.
  const procs = page.locator('footer .it', { hasText: 'Claude ·' });
  await expect(procs).toHaveText(/^1 Claude · \d+ Mo · \d+ % CPU$/);
  // Measured: the fake CLI is cmd.exe running node.
  const memory = Number((await procs.textContent())!.match(/(\d+) Mo/)![1]);
  expect(memory).toBeGreaterThan(10);
  await expect(procs).toHaveAttribute('title', /: \d+ Mo · \d+ %/);
});

test('a subagent’s report and a background task are shown as such, not as the user’s messages', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'lance un sous-agent');
  await expect(page.getByText('Rapport reçu.')).toBeVisible();
  await expect(page.getByText('Rapport du sous-agent « Chercher la cause du bug »')).toBeVisible();
  await expect(page.getByText('Cause trouvée')).toBeVisible();
  await expect(page.getByText('Tâche de fond terminée')).toBeVisible();
  await expect(page.locator('.bubble')).toHaveCount(1);
  await expect(page.locator('.bubble')).toContainText('lance un sous-agent');
  // The context, out of the window the CLI told.
  await expect(page.locator('header.head .m', { hasText: 'Contexte' })).toContainText('/ 200 k');
});

test('a background task that ends while the agent waits is shown before what Claude says of it', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'lance la tâche de fond');
  await expect(page.getByText('Lancé en arrière-plan.')).toBeVisible();
  await expect(page.getByText('Les tests sont passés.')).toBeVisible();
  const done = page.getByText('Tâche de fond terminée');
  await expect(done).toHaveCount(1);
  // In the conversation's order: the end of the task, then Claude's answer to it.
  const order = await page.locator('.msgs').innerText();
  expect(order.indexOf('Tâche de fond terminée')).toBeGreaterThan(order.indexOf('Lancé en arrière-plan.'));
  expect(order.indexOf('Tâche de fond terminée')).toBeLessThan(order.indexOf('Les tests sont passés.'));
});

test('an agent stopped by the usage limit is to resume by itself once it resets, which can be cancelled', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'jusqu’à la limite');
  await expect(page.getByText("Le tour s'est terminé en erreur")).toBeVisible();
  // The fake CLI's quota resets in an hour.
  // At 23:00 or later, the reset falls the next day: its day is told too.
  const resume = page.getByText(/^Reprise automatique (le .+ )?à \d\d:\d\d$/);
  await expect(resume).toBeVisible();
  await expect(page.locator('.card .status', { hasText: /^Reprise (le .+ )?à \d\d:\d\d$/ })).toBeVisible();
  await page.getByRole('button', { name: 'Annuler la reprise' }).click();
  await expect(resume).toBeHidden();
  await expect(page.locator('.card .status', { hasText: 'Reprise' })).toBeHidden();
});

test('edits by Claude are attributed to the agent', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'edit');
  await expect(page.getByText('Fichier modifié.')).toBeVisible();
  await expect(page.locator('.tool').filter({ hasText: 'Edit' })).toContainText('+2');
});

test('the split layout shows the files the agent edits, with their diff, next to the conversation', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  const pane = page.locator('.panel.docked');
  await expect(pane).toContainText('Aucun fichier modifié par cet agent.');
  await send(page, 'edit');
  await expect(page.getByText('Fichier modifié.')).toBeVisible();
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'const a = 2;\nconst b = 3;\n');
  await expect(pane.getByRole('region', { name: 'Diff de src/app.ts' })).toContainText('const b = 3;');
  const conv = (await page.locator('main.conv').boundingBox())!;
  const side = (await pane.boundingBox())!;
  expect(Math.abs(conv.width - side.width)).toBeLessThan(2);
  await page.keyboard.press('Control+Shift+L');
  await expect(pane).toBeHidden();
});

test('the history shows the repository’s commits and opens a commit’s diff', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  await page.getByRole('tab', { name: 'Historique' }).click();
  const pane = page.locator('.panel.docked');
  await expect(pane).toContainText('init');
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'const a = 2;\n');
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qam', 'passe a à 2'], { cwd: app.repo });
  await expect(pane).toContainText('passe a à 2');
  await pane.getByText('passe a à 2').click();
  const diff = page.getByRole('dialog', { name: /passe a à 2/ });
  await expect(diff).toContainText('const a = 2;');
});

test('an agent in remote control shows what is sent to it from claude.ai', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const card = page.locator('.card').first();
  await card.click({ button: 'right' });
  await page.getByRole('menuitem', { name: /Activer le remote control/ }).click();
  await expect(card.getByTitle(/Remote control : connecté/)).toBeVisible();
  // The fake CLI then plays a message sent from the phone, and Claude answers it.
  await expect(page.locator('.bubble', { hasText: 'Message depuis le téléphone' })).toContainText('depuis claude.ai');
  await expect(page.getByText('Bonjour, tu as dit : Message depuis le téléphone')).toBeVisible();
  await card.click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: 'Ouvrir sur claude.ai' })).toBeVisible();
  await page.getByRole('menuitem', { name: 'Désactiver le remote control' }).click();
  await expect(card.getByTitle(/Remote control/)).toBeHidden();
});

test('scrolling up slowly from the bottom of a conversation is never pulled back down', async ({ app }) => {
  const { page } = app;
  // The window of the CI runners: the first reply just fits, and the message field shrinking back
  // once a message is sent moves the view up (which must not stop it following).
  await page.setViewportSize({ width: 1028, height: 779 });
  await addProject(page, app.repo);
  const scroll = page.locator('.scroll');
  // [scrollTop, scrollHeight, clientHeight] after each reply, for the failure message (CI).
  const followed: number[][] = [];
  for (let i = 1; i <= 10; i++) {
    await send(page, `message ${i} ` + 'du texte pour remplir la conversation '.repeat(12));
    await expect(page.getByText(`tu as dit : message ${i} `)).toBeVisible();
    followed.push(await scroll.evaluate((el) => [el.scrollTop, el.scrollHeight, el.clientHeight]));
  }
  const box = (await scroll.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  const bottom = await scroll.evaluate((el) => el.scrollHeight - el.clientHeight);
  // The conversation followed every reply down to the bottom.
  const rendering = await page.evaluate(
    () =>
      new Promise((resolve) => {
        const state = () => ({ visibility: document.visibilityState, focus: document.hasFocus(), size: [innerWidth, innerHeight] });
        const t = setTimeout(() => resolve({ animationFrame: false, ...state() }), 1000);
        requestAnimationFrame(() => (clearTimeout(t), resolve({ animationFrame: true, ...state() })));
      }),
  );
  const jump = await page.getByRole('button', { name: /Nouveaux messages/ }).isVisible();
  expect(await scroll.evaluate((el) => el.scrollTop), JSON.stringify({ rendering, jump, followed })).toBeGreaterThanOrEqual(bottom - 2);
  const tops: number[] = [];
  for (let i = 0; i < 40; i++) {
    await page.mouse.wheel(0, -12);
    await page.waitForTimeout(60);
    tops.push(await scroll.evaluate((el) => el.scrollTop));
  }
  for (let i = 1; i < tops.length; i++) expect(tops[i], JSON.stringify(tops)).toBeLessThanOrEqual(tops[i - 1]);
  expect(tops.at(-1)!, JSON.stringify({ bottom, tops })).toBeLessThan(bottom - 60);
});

test('a deleted agent leaves the list for good', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  await page.getByRole('button', { name: '+ Nouvel agent' }).click();
  await expect(page.locator('.card')).toHaveCount(2);
  await page.locator('.card').first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Supprimer…' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Supprimer' }).click();
  await expect(page.locator('.card')).toHaveCount(1);
  // Its process exits (and its naming may end) after the removal: it must not come back.
  await page.waitForTimeout(1500);
  await expect(page.locator('.card')).toHaveCount(1);
  await expect(page.locator('.card .name')).toHaveText('agent-2');
});

test('a new worktree is set up before its agent takes a message, and torn down before it goes', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await page.getByRole('button', { name: 'Configurer', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('button', { name: 'Ajouter une commande de préparation' }).click();
  await dialog
    .getByRole('group', { name: 'Commande de préparation 1' })
    .getByLabel('Commande', { exact: true })
    .fill('Start-Sleep 3; Set-Content prepare.txt $env:ESCOUADE_BRANCH');
  await dialog.getByRole('button', { name: 'Ajouter une commande de démontage' }).click();
  await dialog
    .getByRole('group', { name: 'Commande de démontage 1' })
    .getByLabel('Commande', { exact: true })
    .fill('Set-Content (Join-Path $env:ESCOUADE_PROJECT_DIR teardown.txt) (Test-Path prepare.txt)');
  await dialog.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(dialog).toBeHidden();

  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect(page.getByRole('status').filter({ hasText: 'Préparation du worktree' })).toContainText(
    'Préparation du worktree · 1/1 · Start-Sleep 3',
  );
  await expect(page.locator('.card')).toContainText('Préparation…');
  // Sent during the setup, the message goes once it is over.
  await send(page, 'Bonjour');
  await expect(page.getByText(/^Worktree préparé \(1 commande, \d+ s\)\.$/)).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  expect(fs.readFileSync(path.join(wt, 'prepare.txt'), 'utf8').trim()).toMatch(/^escouade\/agent-1/);

  await page.locator('.card').first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Supprimer…' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Supprimer' }).click();
  await expect(page.locator('.card')).toHaveCount(0);
  // Torn down in the worktree while it was there, then removed.
  await expect.poll(() => fs.existsSync(wt), { timeout: 30_000 }).toBe(false);
  expect(fs.readFileSync(path.join(app.repo, 'teardown.txt'), 'utf8').trim()).toBe('True');
});

test('a terminal runs commands in the project folder', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByTitle('Nouveau terminal').click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('.xterm')).toBeVisible();
  await page.waitForTimeout(2500); // PowerShell start-up
  await page.locator('.xterm').click();
  await page.keyboard.type('Set-Content e2e-terminal.txt ok');
  await page.keyboard.press('Enter');
  await expect.poll(() => fs.existsSync(path.join(app.repo, 'e2e-terminal.txt')), { timeout: 20_000 }).toBe(true);
});

test('launch commands run in their own terminals, with their status live', async ({ app }) => {
  const { page } = app;
  const marker = path.join(app.repo, 'web', 'serveur.txt');
  fs.mkdirSync(path.dirname(marker));
  await addProject(page, app.repo);

  // The project's settings, on its tab.
  await page.getByRole('button', { name: 'Configurer', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await expect(dialog.getByRole('tab', { name: 'Projets' })).toHaveAttribute('aria-selected', 'true');
  await dialog.getByRole('button', { name: '+ Ajouter une commande' }).click();
  const serveurCfg = dialog.getByRole('group', { name: 'Commande 1' });
  await serveurCfg.getByLabel('Nom').fill('Serveur');
  await serveurCfg.getByLabel('Commande').fill('Set-Content serveur.txt ok; Start-Sleep 600');
  await serveurCfg.getByLabel(/Sous-dossier/).fill('web');
  await dialog.getByRole('button', { name: '+ Ajouter une commande' }).click();
  const buildCfg = dialog.getByRole('group', { name: 'Commande 2' });
  await buildCfg.getByLabel('Nom').fill('Build');
  await buildCfg.getByLabel('Commande').fill('exit 2');
  await dialog.getByRole('button', { name: 'Enregistrer' }).click();

  const serveur = page.locator('.run', { hasText: 'Serveur' });
  const build = page.locator('.run', { hasText: 'Build' });
  await expect(serveur).toContainText('prêt');
  await page.getByRole('button', { name: 'Tout lancer', exact: true }).click();
  await expect(serveur).toContainText('en cours');
  await expect(build).toContainText('planté (code 2)', { timeout: 30_000 });
  await expect(page.getByText("« Build » s'est arrêté en erreur (code 2)")).toBeVisible();
  // Run in its subfolder.
  await expect.poll(() => fs.existsSync(marker), { timeout: 30_000 }).toBe(true);

  // Its log replaces the conversation; stopping it is not a crash.
  await serveur.click();
  const view = page.locator('main.rv');
  await expect(view.locator('.xterm')).toBeVisible();
  await expect(view).toContainText('en cours');
  await view.getByRole('button', { name: 'Stopper' }).click();
  await expect(serveur).toContainText('arrêté', { timeout: 15_000 });
  await expect(page.getByText("« Serveur » s'est arrêté en erreur")).toHaveCount(0);

  fs.rmSync(marker);
  await view.getByRole('button', { name: 'Relancer' }).click();
  await expect(serveur).toContainText('en cours');
  await expect.poll(() => fs.existsSync(marker), { timeout: 30_000 }).toBe(true);
  await page.getByRole('button', { name: 'Tout arrêter', exact: true }).click();
  await expect(serveur).toContainText('arrêté', { timeout: 15_000 });

  // Back to the agent.
  await page.locator('.card').first().click();
  await expect(view).toHaveCount(0);
});

test('stats record the turns of the app’s agents', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Tâche terminée')).toBeVisible();
  await page.getByRole('button', { name: 'Stats' }).click();
  await expect(page.getByText('Statistiques')).toBeVisible();
  const prompts = page.locator('.kpi').filter({ hasText: 'Prompts' });
  await expect(prompts).toContainText('1');
  await expect(page.getByText('demo-api').last()).toBeVisible();
});

test('the proxy configured in the settings reaches Claude', async ({ app }) => {
  const { page } = app;
  await page.getByTitle('Réglages (Ctrl+,)').click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Réseau' }).click();
  await dialog.getByPlaceholder('aucun').fill('http://proxy.local:3128');
  await dialog.getByRole('button', { name: 'Enregistrer' }).click();
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  expect(app.launches().at(-1)?.proxy).toBe('http://proxy.local:3128');
});

test('Ctrl+N creates another agent and Ctrl+J jumps to the one waiting', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'question');
  await expect(page.getByTestId('question-pending')).toBeVisible();
  await page.keyboard.press('Control+n');
  await expect(page.locator('.card')).toHaveCount(2);
  await expect(page.getByText('Agent prêt')).toBeVisible();
  await page.keyboard.press('Control+j');
  await expect(page.getByTestId('question-pending')).toBeVisible();
});

test('a project’s color is picked from its tab’s menu, not from the sidebar', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await expect(page.locator('aside.side')).not.toContainText('Couleur');
  const tab = page.locator('.tab', { hasText: 'demo-api' });
  await tab.click({ button: 'right' });
  const third = page.getByRole('menuitemradio', { name: 'Couleur 3' });
  const color = await third.evaluate((el) => getComputedStyle(el).backgroundColor);
  await third.click();
  await expect(page.getByRole('menu')).toBeHidden();
  await expect(tab.locator('.swatch')).toHaveCSS('background-color', color);
  await tab.click({ button: 'right' });
  await expect(page.getByRole('menuitemradio', { name: 'Couleur 3' })).toHaveAttribute('aria-checked', 'true');
});

test('project tabs can be reordered (drag and drop logic)', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await addProject(page, app.repo, { name: 'second', firstAgent: false });
  const tabs = page.locator('.tabs .tab .name');
  await expect(tabs).toHaveText(['demo-api', 'second']);
  await page
    .locator('.tabs .tab')
    .nth(1)
    .dragTo(page.locator('.tabs .tab').nth(0), { targetPosition: { x: 5, y: 10 } });
  await expect(tabs).toHaveText(['second', 'demo-api']);
});

test('a file is browsed, edited and saved in the embedded editor', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Parcourir' }).click();
  await page.getByRole('treeitem', { name: /src/ }).click();
  await page.getByRole('treeitem', { name: /app\.ts/ }).click();
  await expect(page.getByRole('tab', { name: /app\.ts/ })).toBeVisible();
  const code = page.locator('.cm-content');
  await expect(code).toContainText('const a = 1;');
  await code.click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('export const b = 2;');
  await expect(page.getByText(/● Non enregistré/)).toBeVisible();
  await page.keyboard.press('Control+S');
  await expect(page.getByText('Enregistré', { exact: true })).toBeVisible();
  expect(fs.readFileSync(path.join(app.repo, 'src', 'app.ts'), 'utf8')).toBe('const a = 1;\nexport const b = 2;');
  await page.getByRole('button', { name: '← Conversation' }).click();
  await expect(page.locator('main.conv')).toBeVisible();
});

test('a file is created from the editor’s tree, folders included, then written and saved', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Parcourir' }).click();
  await page.getByRole('treeitem', { name: /src/ }).click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Nouveau fichier…' }).click();
  const field = page.getByRole('textbox', { name: 'Nom du nouveau fichier' });
  await field.fill('app.ts');
  await expect(page.getByRole('alert')).toHaveText('« app.ts » existe déjà à cet endroit.');
  await field.fill('lib/util.ts');
  await field.press('Enter');
  await expect(page.getByRole('tab', { name: /util\.ts/ })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('treeitem', { name: /util\.ts/ })).toHaveAttribute('aria-selected', 'true');
  expect(fs.readFileSync(path.join(app.repo, 'src', 'lib', 'util.ts'), 'utf8')).toBe('');
  // The cursor is in the file created: typing goes there right away.
  await expect(page.locator('.cm-editor')).toHaveClass(/cm-focused/);
  await page.keyboard.type('export const u = 1;');
  await page.keyboard.press('Control+S');
  await expect(page.getByText('Enregistré', { exact: true })).toBeVisible();
  expect(fs.readFileSync(path.join(app.repo, 'src', 'lib', 'util.ts'), 'utf8')).toBe('export const u = 1;');
});

test('a file of an agent’s worktree opens in the editor from the uncommitted files', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true });
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'const a = 42;\n');
  await page.getByRole('button', { name: /Fichiers/ }).click();
  // The row's editor button shows on hover.
  await page.locator('.filerow').filter({ hasText: 'app.ts' }).hover();
  await page.getByRole('button', { name: 'Ouvrir app.ts dans l’éditeur' }).click();
  await expect(page.locator('.cm-content')).toContainText('const a = 42;');
  await expect(page.getByText('1 ligne modifiée vs main')).toBeVisible();
});

test('a ticket is taken by an agent that loops until its criteria are met, then merged once validated', async ({ app }) => {
  const { page } = app;
  // The window of the CI runners, close to the narrowest the app allows (1000): the board must fit in it.
  await page.setViewportSize({ width: 1028, height: 779 });
  // A long name (19 characters, its key still DEM): the header's left side is at its widest.
  await addProject(page, app.repo, { firstAgent: false, name: 'demonstration-d-api' });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('button', { name: 'Nouveau ticket' })).toBeVisible();
  // What of the board sticks out of its box: its header, and the cards of its columns.
  const sticksOut = () =>
    page.evaluate(() => {
      const boxes = [...document.querySelectorAll('main.board > .head, main.board .cards')];
      if (boxes.length !== 5) return [`found ${boxes.length} boxes of the board, not its header and 4 columns`];
      return boxes.filter((e) => e.scrollWidth > e.clientWidth).map((e) => `${e.className} (${e.scrollWidth} > ${e.clientWidth})`);
    });
  // What lies where it should not in the header: the settings button keeps its label, and neither the
  // places nor the button run over what follows them.
  const headerOverlaps = () =>
    page.evaluate(() => {
      const r = (selector: string) => document.querySelector(`main.board ${selector}`)!.getBoundingClientRect();
      const [places, cfg, label, auto] = [r('.places'), r('.cfg'), r('.cfg .k'), r('.auto')];
      const problems: string[] = [];
      if (label.right > cfg.right + 0.5) problems.push(`label ends at ${label.right}, its button at ${cfg.right}`);
      if (cfg.right > auto.left + 0.5) problems.push(`button ends at ${cfg.right}, the switch starts at ${auto.left}`);
      if (places.right > cfg.left + 0.5) problems.push(`places end at ${places.right}, the button starts at ${cfg.left}`);
      return problems;
    });
  expect(await sticksOut()).toEqual([]);
  expect(await headerOverlaps()).toEqual([]);
  // The longest texts the places can say (the real ones need a quota, no Claude Code, or a target branch
  // that cannot start a ticket), set on the text the board manages and then given back.
  const say = (text: string | null, missing: boolean) =>
    page.evaluate(
      ([t, m]) => {
        const places = document.querySelector('main.board .places')!;
        places.firstChild!.nodeValue = t;
        places.classList.toggle('missing', m);
      },
      [text, missing] as const,
    );
  const usual = await page.locator('main.board .places').textContent();
  for (const [text, missing] of [
    ['Toutes les places sont prises', false],
    ['Quota atteint — reprise à 14:35', false],
    ['Claude Code introuvable — aucun ticket ne démarre', true],
    ["develop n'a encore aucun commit — aucun ticket ne démarre", true],
    ['Branche cible release/2026-10 introuvable — aucun ticket ne démarre', true],
  ] as const) {
    await say(text, missing);
    expect(await headerOverlaps(), text).toEqual([]);
    expect(await sticksOut(), text).toEqual([]);
  }
  await say(usual, false);
  // In a wide window, a long summary of what validating does is shown whole, not cut as in a narrow one.
  const summary = page.locator('main.board .cfg .v');
  const usualSummary = await summary.textContent();
  const cut = () => summary.evaluate((e) => e.scrollWidth > e.clientWidth);
  await summary.evaluate((e) => (e.firstChild!.nodeValue = 'merge squash → feature/une-branche-au-nom-long'));
  expect(await cut()).toBe(true);
  await page.setViewportSize({ width: 1600, height: 900 });
  await expect.poll(cut).toBe(false);
  expect(await headerOverlaps()).toEqual([]);
  await summary.evaluate((e, t) => (e.firstChild!.nodeValue = t), usualSummary);
  await page.setViewportSize({ width: 1028, height: 779 });
  // Its settings button opens the app's settings on the Kanban of this project, whole in the narrowest window.
  await page.locator('main.board .cfg').click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await expect(settings.getByRole('tab', { name: 'Kanban' })).toHaveAttribute('aria-selected', 'true');
  await expect(settings.getByRole('group', { name: 'Projet' }).getByRole('button', { name: 'demonstration-d-api' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  expect(
    await settings.evaluate((e) => {
      const r = e.getBoundingClientRect();
      return r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight;
    }),
  ).toBe(true);
  // Its footer's « Annuler » is not its conflict policy's: Escape closes it too, changing nothing.
  await page.keyboard.press('Escape');
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Ajouter le fichier du ticket');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  // The fake agent meets criterion n at loop n: two loops for the two default criteria.
  const review = page.getByRole('region', { name: 'À tester' });
  // Exact: the progress list (« Fichier dem-1.txt écrit ») and the agent's name also hold the key.
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  await expect(review.getByText('2/2 critères')).toBeVisible();
  // A met criterion is a plain line, not a green button.
  await expect(review.getByRole('list', { name: 'Critères' }).getByRole('listitem').first()).toHaveCSS(
    'background-color',
    'rgba(0, 0, 0, 0)',
  );
  // The card's foot keeps the agent's name readable and its figures (cost, criteria met) inside the card: at this
  // width they go under the name, which they neither squeeze to nothing nor push out of the card.
  const foot = () =>
    review.locator('.foot').evaluate((e) => {
      const card = e.closest('.card')!.getBoundingClientRect();
      const box = (selector: string) => e.querySelector(selector)!.getBoundingClientRect();
      const problems: string[] = [];
      if (box('.name').width < 60) problems.push(`the agent's name is ${box('.name').width} px wide`);
      for (const child of e.querySelectorAll('.who, .figures, .figures > .k')) {
        const r = child.getBoundingClientRect();
        if (r.left < card.left || r.right > card.right)
          problems.push(`${child.className} lies ${r.left}..${r.right}, the card ${card.left}..${card.right}`);
      }
      return problems;
    });
  expect(await foot()).toEqual([]);
  expect(await sticksOut()).toEqual([]);
  await review.getByRole('button', { name: 'Valider et merger' }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('⤵ Mergé dans main · squash')).toBeVisible({ timeout: 60_000 });
  expect(await sticksOut()).toEqual([]);
  // The agent's file is on the project's branch, committed with a generated message.
  expect(fs.readFileSync(path.join(app.repo, 'dem-1.txt'), 'utf8')).toBe('Boucle 2\n');
  const subject = execFileSync('git', ['log', '-1', '--format=%s'], { cwd: app.repo, encoding: 'utf8' }).trim();
  expect(subject).toBe('feat: travail du faux claude [DEM-1]');
  // Its agent's conversation tells the loop it went through and its report.
  await done.getByRole('button', { name: /DEM-1/ }).click();
  await expect(page.getByText(/Boucle 2\/5\. Critères non atteints : 2/)).toBeVisible();
  await expect(page.getByText('Bilan des critères').first()).toBeVisible();
});

test('a ticket’s test launches leave the launch section, their processes stopped, once it is done', async ({ app }) => {
  const { page } = app;
  // The test server stays up without answering on its address (on the ticket's free ports): no browser opens.
  // It writes its pid outside the repository.
  const pidFile = path.join(app.data, 'serveur.pid');
  fs.writeFileSync(
    path.join(app.repo, 'serveur.js'),
    `require('node:fs').writeFileSync(${JSON.stringify(pidFile)}, String(process.pid));\nsetInterval(() => {}, 1000);\n`,
  );
  execFileSync('git', ['add', '-A'], { cwd: app.repo });
  execFileSync('git', ['commit', '-qm', 'Serveur de test'], { cwd: app.repo });
  const alive = (pid: number) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Page [ok] [recette]');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });

  await review.getByRole('button', { name: '▶ Tester' }).click();
  const dialog = page.getByRole('dialog', { name: 'Tester DEM-1' });
  // The agent wrote this recipe: it is shown first, and nothing of it runs until « Lancer ».
  await expect(dialog.getByText(/Ces commandes ont été écrites par/)).toBeVisible();
  await expect(dialog.getByText('node serveur.js')).toBeVisible();
  // What is read is in the order the shell reads it, whatever letters it holds.
  for (const text of [dialog.locator('.cmd').first(), dialog.locator('.facts dd').first()]) {
    await expect(text).toHaveCSS('unicode-bidi', 'bidi-override');
    await expect(text).toHaveCSS('direction', 'ltr');
  }
  expect(fs.existsSync(pidFile)).toBe(false);
  await dialog.getByRole('button', { name: 'Lancer' }).click();
  await expect(dialog.getByText(/en attente de localhost:\d+…/)).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => fs.existsSync(pidFile), { timeout: 30_000 }).toBe(true);
  const pid = Number(fs.readFileSync(pidFile, 'utf8'));
  expect(alive(pid)).toBe(true);
  await dialog.getByRole('button', { name: 'Fermer' }).last().click();
  // Its terminal, in the launch section under its agent's name.
  const runs = page.locator('.runs');
  await expect(runs.locator('.group')).toHaveText(/^dem-1/);
  await expect(runs.locator('.run', { hasText: 'web' })).toContainText('en cours');

  await review.getByRole('button', { name: 'Valider et merger' }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('⤵ Mergé dans main · squash')).toBeVisible({ timeout: 60_000 });
  // Done: its terminal is gone from the section, its process with it, and neither crashed.
  await expect(runs.locator('.group')).toHaveCount(0);
  await expect(runs.locator('.run')).toHaveCount(0);
  await expect.poll(() => alive(pid), { timeout: 15_000 }).toBe(false);
  await expect(page.getByText(/s'est arrêté en erreur/)).toHaveCount(0);
});
