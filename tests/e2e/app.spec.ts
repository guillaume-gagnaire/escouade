import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator } from '@playwright/test';
import { addProject, expect, send, test, unfoldSections, openSettingsPage, settingsEntry, closeSettings } from './fixture';

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
  await expect(page.locator('footer')).toContainText('5 h');
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
  await expect(page.getByRole('tab', { name: /^demo-api/ }).locator('.ask')).toHaveText('1');
  await card.getByRole('button', { name: 'SQLite' }).click();
  await expect(page.getByText('Choix retenu : SQLite')).toBeVisible();
  await expect(page.locator('footer')).toContainText('0 en attente');
  // Answered: its choices, the one taken marked.
  await expect(page.locator('.opt-r.picked', { hasText: 'SQLite' })).toBeVisible();
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
  const refused = page.locator('.req-card.done', { hasText: 'rm -rf build' });
  await expect(refused.locator('.by')).toContainText('Refusé');
  await expect(refused.locator('.cmdl .badge')).toHaveText('Bash');
});

test('uncommitted changes show up beside the project’s name, in the files panel and the diff', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'const a = 2;\nconst b = 3;\n');
  await expect(page.locator('header .delta')).toHaveText('Δ 1');
  // Ctrl+Maj+B: the header puts « Fichiers » away in a narrow column (the runner's 1024 px screen), « … » then has it.
  await page.keyboard.press('Control+Shift+B');
  await page.getByRole('button', { name: 'Tout le projet' }).click();
  await expect(page.locator('aside.git-side')).toContainText('app.ts');
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
  // Ctrl+Maj+B: the header puts « Fichiers » away in a narrow column (the runner's 1024 px screen), « … » then has it.
  await page.keyboard.press('Control+Shift+B');
  await page.getByRole('button', { name: 'Tout le projet' }).click();
  const panel = page.locator('aside.git-side');
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
  await expect(sync).toContainText('main');
  await expect(sync).toContainText('0 commit à tirer');

  // Someone else pushes a commit.
  const other = path.join(root, 'other');
  git(root, 'clone', '-q', remote, other);
  fs.writeFileSync(path.join(other, 'README.md'), 'depuis ailleurs\n');
  git(other, 'add', '-A');
  git(other, 'commit', '-qm', 'readme');
  git(other, 'push', '-q');

  await sync.click();
  await page.getByRole('dialog', { name: 'Branches' }).getByRole('button', { name: /Fetch/ }).click();
  await expect(sync).toContainText('1 commit à tirer');
  await sync.click();
  await page
    .getByRole('dialog', { name: 'Branches' })
    .getByRole('button', { name: /Récupérer/ })
    .click();
  await expect(sync).toContainText('0 commit à tirer');
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

test('the status bar counts the running agent processes, with their memory and CPU', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  // The first agent's process starts with the project.
  const procs = page.locator('footer .it', { hasText: /agents? ·/ });
  await expect(procs).toHaveText(/^1 agent · \d+ Mo · \d+ % CPU$/);
  // Measured: the fake CLI is cmd.exe running node.
  const memory = Number((await procs.textContent())!.match(/(\d+) Mo/)![1]);
  expect(memory).toBeGreaterThan(10);
  await expect(procs).toHaveAttribute('data-tip', /: \d+ Mo · \d+ %/);
});

test('the tooltip of the resources is the app’s: its look, one agent a line, above the status bar, gone when the pointer leaves', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const procs = page.locator('footer .it', { hasText: /agents? ·/ });
  await expect(procs).toHaveText(/^1 agent ·/);
  // No `title` left for the system to show its own.
  await expect(procs).not.toHaveAttribute('title');
  await procs.hover();
  const tip = page.getByRole('tooltip');
  await expect(tip).toHaveText(/^Processus des agents en cours[^\n]*\n[^\n]+ : \d+ Mo · \d+ %$/);
  const look = await tip.evaluate((el) => {
    const probe = document.createElement('div');
    // The relief of what floats (the 2.0 design).
    probe.style.background = 'var(--raise-2)';
    document.body.append(probe);
    const raise = getComputedStyle(probe).backgroundColor;
    probe.remove();
    const s = getComputedStyle(el);
    return { background: s.backgroundColor, raise, whiteSpace: s.whiteSpace, position: s.position };
  });
  expect(look).toEqual({ background: look.raise, raise: look.raise, whiteSpace: 'pre-line', position: 'fixed' });
  const [box, item] = [(await tip.boundingBox())!, (await procs.boundingBox())!];
  expect(box.y + box.height).toBeLessThanOrEqual(item.y);
  expect(box.x).toBeGreaterThanOrEqual(8);
  await page.mouse.move(400, 300);
  await expect(tip).toBeHidden();
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
  await expect(page.locator('header.head .fig', { hasText: 'Contexte' })).toContainText('/ 200 k');
});

test('a local command shows as typed with its output under it, and the wake-up of a loop as a wake-up, no tag shown (ESC-154)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const msgs = page.locator('.msgs');
  await send(page, '/cost');
  // Its lines and their columns kept, without its ANSI codes.
  const output = page.locator('.notice.output');
  await expect(output).toHaveText('Total cost:            $0.05\nTotal duration (API):  1.2s\nUsage:                 100 input, 20 output');
  await send(page, '/cost echec');
  await expect(page.locator('.notice.error')).toHaveText('Erreur de /cost : indisponible');
  // A ticket driven by a loop: the wake-up runs `/loop` again (Claude Code 2.1.309), then the ticket's return.
  await send(page, '/loop suivre ESC-1');
  await expect(page.getByText('Le ticket est revenu (suivre ESC-1).')).toBeVisible();
  await expect(page.locator('.event', { hasText: 'Réveil' })).toContainText('/loop suivre ESC-1');
  // The commands as typed, once each; nothing of the caveat.
  await expect(page.locator('.bubble .text')).toHaveText(['/cost', '/cost echec', '/loop suivre ESC-1']);
  await expect(msgs).not.toContainText('Caveat');
  expect(await msgs.innerText()).not.toMatch(/<\/?(local-command|command-|bash-)|\u001b/);
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
  const pane = page.locator('aside.git-side.docked');
  await expect(pane).toContainText('Aucun fichier modifié par cet agent.');
  await send(page, 'edit');
  await expect(page.getByText('Fichier modifié.')).toBeVisible();
  fs.writeFileSync(path.join(app.repo, 'src', 'app.ts'), 'const a = 2;\nconst b = 3;\n');
  await expect(pane.getByRole('region', { name: 'Diff de src/app.ts' })).toContainText('const b = 3;');
  const conv = (await page.locator('main.conv').boundingBox())!;
  const side = (await pane.boundingBox())!;
  // As the preview lays it out: 44 % of the work, never under 340 pixels.
  expect(Math.abs(side.width - Math.max(340, 0.44 * (conv.width + side.width)))).toBeLessThan(2);
  await page.keyboard.press('Control+Shift+L');
  await expect(pane).toBeHidden();
});

test('the history shows the repository’s commits and opens a commit’s diff', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Conversation et fichiers côte à côte' }).click();
  await page.getByRole('tab', { name: 'Historique' }).click();
  const pane = page.locator('aside.git-side.docked');
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
  await expect(card.locator('[data-tip*="Remote control : connecté"]')).toBeVisible();
  // The fake CLI then plays a message sent from the phone, and Claude answers it.
  await expect(page.locator('.msg', { hasText: 'Message depuis le téléphone' })).toContainText('depuis claude.ai');
  await expect(page.getByText('Bonjour, tu as dit : Message depuis le téléphone')).toBeVisible();
  await card.click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: 'Ouvrir sur claude.ai' })).toBeVisible();
  await page.getByRole('menuitem', { name: 'Désactiver le remote control' }).click();
  await expect(card.locator('[data-tip*="Remote control"]')).toBeHidden();
});

test('the setting for all agents gives each one remote control, which one is set apart from by hand', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const card = page.locator('.card').first();
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  await dialog.getByRole('switch', { name: 'Remote control pour tous les agents' }).click();
  // The project's own switch never says otherwise: on and grayed, with the reason.
  await openSettingsPage(dialog, 'Confidentialité');
  const own = dialog.getByRole('switch', { name: 'Remote control pour les agents de ce projet' });
  await expect(own).toHaveAttribute('aria-checked', 'true');
  await expect(own).toBeDisabled();
  await expect(dialog.getByText('Activé pour tous les projets dans Réglages › Agents')).toBeVisible();
  await closeSettings(dialog);
  // The agent there was gets it, and its card says where it comes from.
  await expect(card.locator('[data-tip*="Remote control : connecté"][data-tip*="activé par le réglage de tous les agents"]')).toBeVisible();
  // Set apart by hand: off in spite of the setting.
  await card.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Désactiver le remote control' }).click();
  await expect(card.locator('[data-tip*="Remote control"]')).toBeHidden();
  await card.click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: /Activer le remote control/ })).toBeVisible();
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
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).click();
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
  // The header's « Commandes de lancement… »: in a short window (the runner's), the section folds and « Configurer » with it.
  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  // It opens on « Lancement » (annexe B); the worktrees are on « Git et worktrees ».
  await openSettingsPage(dialog, 'Git et worktrees');
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
  await closeSettings(dialog);

  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
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

test('a new agent’s worktree is made at its first message, under the name the fake claude gives it, and set up there (ESC-156)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, atFirstMessage: true, firstAgent: false });
  // The header's « Commandes de lancement… »: in a short window (the runner's), the section folds and « Configurer » with it.
  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  // It opens on « Lancement » (annexe B); the worktrees are on « Git et worktrees ».
  await openSettingsPage(dialog, 'Git et worktrees');
  await expect(dialog.getByRole('switch', { name: 'Créer le worktree au premier message, au nom de l’agent' })).toBeChecked();
  await dialog.getByRole('button', { name: 'Ajouter une commande de préparation' }).click();
  await dialog
    .getByRole('group', { name: 'Commande de préparation 1' })
    .getByLabel('Commande', { exact: true })
    .fill("Set-Content prepare.txt ((Split-Path -Leaf (Get-Location)) + ' ' + $env:ESCOUADE_BRANCH)");
  await closeSettings(dialog);

  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  const card = page.locator('.card').first();
  await expect(card).toContainText('worktree au premier message');
  await expect(page.getByText('Le worktree sera créé au premier message, au nom de l’agent.')).toBeVisible();
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  const made = () => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees) : []);
  expect(made()).toEqual([]);

  await send(page, 'Ajoute un bouton');
  // Named first, then its worktree under that name, set up there, then its turn.
  await expect(page.getByText(/^Worktree préparé \(1 commande, \d+ s\)\.$/)).toBeVisible({ timeout: 30_000 });
  await expect(page.getByText('Bonjour, tu as dit : Ajoute un bouton')).toBeVisible();
  expect(made()).toEqual(['ajoute-un-fake']);
  const wt = path.join(worktrees, 'ajoute-un-fake');
  expect(fs.readFileSync(path.join(wt, 'prepare.txt'), 'utf8').trim()).toBe('ajoute-un-fake escouade/ajoute-un-fake');
  await expect(card.locator('.name')).toHaveText('ajoute-un-fake');
  await expect(card.locator('.branch')).toHaveText('ajoute-un-fake');
  await expect(page.getByText('Le worktree sera créé au premier message, au nom de l’agent.')).toBeHidden();
});

test('a project’s worktrees go to the folder and on the branch prefix its settings name', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  // The header's « Commandes de lancement… »: in a short window (the runner's), the section folds and « Configurer » with it.
  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  // It opens on « Lancement » (annexe B); the worktrees are on « Git et worktrees ».
  await openSettingsPage(dialog, 'Git et worktrees');
  // Beside the repository, as the repositories that want no worktree in them have it.
  await dialog.getByRole('textbox', { name: 'Dossier des worktrees' }).fill('../demo-api-worktrees');
  const beside = path.join(path.dirname(app.repo), 'demo-api-worktrees');
  // (The home folder shortened to ~.)
  await expect(dialog.getByText(/^Le prochain : .*[\\/]demo-api-worktrees[\\/]<nom>$/)).toBeVisible();
  const prefix = dialog.getByRole('textbox', { name: 'Préfixe des branches des agents' });
  await prefix.fill('gg /');
  await expect(prefix).toHaveAttribute('aria-invalid', 'true');
  await prefix.fill('gg/');
  await closeSettings(dialog);

  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await expect.poll(() => (fs.existsSync(beside) ? fs.readdirSync(beside) : [])).toEqual(['agent-1']);
  const wt = path.join(beside, 'agent-1');
  const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
  expect(git(wt, 'rev-parse', '--abbrev-ref', 'HEAD')).toBe('gg/agent-1');
  // Nothing in the repository: neither a folder nor a line of its status.
  expect(fs.existsSync(path.join(app.repo, '.claude'))).toBe(false);
  expect(git(app.repo, 'status', '--porcelain')).toBe('');
  // The sidebar shows its branch without the project's prefix, the editor its folder from the project's.
  await expect(page.locator('.card').first().locator('.branch')).toHaveText('agent-1');
  await page.locator('.card').first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Ouvrir dans l’éditeur' }).click();
  await expect(page.getByText('../demo-api-worktrees/agent-1', { exact: true })).toBeVisible();
});

test('a terminal runs commands in the project folder', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('.xterm')).toBeVisible();
  await page.waitForTimeout(2500); // PowerShell start-up
  await page.locator('.xterm').click();
  await page.keyboard.type('Set-Content e2e-terminal.txt ok');
  await page.keyboard.press('Enter');
  await expect.poll(() => fs.existsSync(path.join(app.repo, 'e2e-terminal.txt')), { timeout: 20_000 }).toBe(true);
});

test('a keystroke whose echo comes late is noted in the log, with where the time went', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('.xterm')).toBeVisible();
  await page.waitForTimeout(2500); // PowerShell start-up
  await page.locator('.xterm').click();
  // A program that answers each key 800 ms late: the time is the program's, the log must say so.
  await page.keyboard.type('while ($true) { $k = [Console]::ReadKey($true); Start-Sleep -Milliseconds 800; [Console]::Write($k.KeyChar) }');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(1500);
  const log = () => fs.readFileSync(path.join(app.data, 'app.log'), 'utf8');
  const slow =
    /echo of a 1-byte write shown after (\d+) ms: written (\d+) ms after the backend got it, first output (\d+) ms after, the echo (\d+) ms after/g;
  const slowEchoes = () => [...log().matchAll(slow)];
  // A key of the command slow to show (a loaded machine) took the terminal's one report in 10 s: wait for the next.
  if (slowEchoes().length) await page.waitForTimeout(10_500);
  const before = slowEchoes().length;
  await page.keyboard.press('x');
  await expect.poll(() => slowEchoes().length, { timeout: 15_000 }).toBeGreaterThan(before);
  const [, total, , , program] = slowEchoes().at(-1)!.map(Number);
  expect(total).toBeGreaterThanOrEqual(800);
  expect(program).toBeGreaterThanOrEqual(700);
  // Which renderer draws the terminal is there too.
  expect(log()).toMatch(/\((pwsh|powershell)\): renderer (webgl|dom)/);
});

test('a terminal opens in the worktree of an agent from the + of the terminals and from its conversation’s header', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees) : [])).toEqual(['agent-1']);
  const wt = path.join(worktrees, 'agent-1');
  // The + builds its menu when clicked: the app must know of the worktree by then, not only the disk.
  await expect(page.locator('.card').first().locator('.branch')).toHaveText('agent-1');
  /** Writes a file from the terminal shown, which must be in the worktree. */
  const write = async (file: string) => {
    const term = page.locator('.xterm:visible');
    await expect(term).toBeVisible();
    await page.waitForTimeout(2500); // PowerShell start-up
    await term.click();
    await page.keyboard.type(`Set-Content ${file} ok`);
    await page.keyboard.press('Enter');
    await expect.poll(() => fs.existsSync(path.join(wt, file)), { timeout: 20_000 }).toBe(true);
    expect(fs.existsSync(path.join(app.repo, file))).toBe(false);
  };

  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: 'Dans le worktree de…' }).click();
  await page.getByRole('menuitem', { name: 'agent-1', exact: true }).click();
  await write('e2e-plus.txt');

  // From « … » of the agent's header: its menu, « Ouvrir un terminal » in its worktree.
  await page.locator('.card').first().click();
  await page.getByRole('button', { name: 'Actions de l’agent' }).click();
  await page.getByRole('menuitem', { name: 'Ouvrir un terminal' }).click();
  await write('e2e-header.txt');
});

test('in the Claude Code view, a pasted image goes to Claude Code as a file, and pasted text as before', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Vue Claude Code' }).click();
  await expect.poll(() => app.launches().some((l) => l.argv.includes('--settings')), { timeout: 20_000 }).toBe(true);
  // Its pastes go through Escouade once its interface started (a fixed wait was too short on CI).
  const field = page.locator('main [data-pastes="images"] .xterm textarea');
  await expect(field).toBeAttached({ timeout: 20_000 });
  // What WebView2 gives a paste: a screenshot is a PNG file without text; text has no file.
  const paste = (content: { png?: number[]; text?: string }) =>
    field.evaluate((el, { png, text }) => {
      const data = new DataTransfer();
      if (png) data.items.add(new File([new Uint8Array(png)], 'image.png', { type: 'image/png' }));
      if (text) data.setData('text/plain', text);
      el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
    }, content);
  const png = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];
  await paste({ png });
  const images = path.join(app.root, 'claude', 'fake-tui-images.jsonl');
  await expect.poll(() => fs.existsSync(images), { timeout: 15_000 }).toBe(true);
  const [image] = fs
    .readFileSync(images, 'utf8')
    .trim()
    .split('\n')
    .map((l) => JSON.parse(l));
  expect([...Buffer.from(image.data, 'base64')]).toEqual(png);
  expect(path.dirname(image.path)).toBe(path.join(app.data, 'tui', 'pastes'));

  await paste({ text: ' que vois-tu ?' });
  await field.press('Enter');
  // The prompt Claude Code got, in its transcript (none while its folder is not made yet: `expect.poll` stops at a throw).
  const projects = path.join(app.root, 'claude', 'projects');
  const prompts = () =>
    (fs.existsSync(projects) ? fs.readdirSync(projects, { recursive: true, encoding: 'utf8' }) : [])
      .filter((f) => f.endsWith('.jsonl'))
      .flatMap((f) =>
        fs
          .readFileSync(path.join(app.root, 'claude', 'projects', f), 'utf8')
          .trim()
          .split('\n'),
      )
      .map((l) => JSON.parse(l))
      .filter((e) => e.type === 'user' && typeof e.message?.content === 'string')
      .map((e) => e.message.content);
  await expect.poll(prompts, { timeout: 15_000 }).toContain('[Image #1] que vois-tu ?');
});

test('launch commands run in their own terminals, with their status live', async ({ app }) => {
  const { page } = app;
  const marker = path.join(app.repo, 'web', 'serveur.txt');
  fs.mkdirSync(path.dirname(marker));
  await addProject(page, app.repo);

  // The project's settings, on its tab.
  // The header's « Commandes de lancement… »: in a short window (the runner's), the section folds and « Configurer » with it.
  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await expect(settingsEntry(dialog, 'Lancement')).toHaveAttribute('aria-current', 'page');
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const serveurCfg = dialog.getByRole('group', { name: 'Commande 1' });
  await serveurCfg.getByLabel('Nom').fill('Serveur');
  await serveurCfg.getByLabel('Commande').fill('Set-Content serveur.txt ok; Start-Sleep 600');
  await serveurCfg.getByLabel(/Sous-dossier/).fill('web');
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const buildCfg = dialog.getByRole('group', { name: 'Commande 2' });
  await buildCfg.getByLabel('Nom').fill('Build');
  await buildCfg.getByLabel('Commande').fill('exit 2');
  await closeSettings(dialog);
  await unfoldSections(page);
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

test('a launch command runs in the worktree of an agent, with its ports, and stops with it (ESC-19)', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true });
  const worktrees = path.join(app.repo, '.claude', 'worktrees');
  await expect.poll(() => (fs.existsSync(worktrees) ? fs.readdirSync(worktrees).length : 0)).toBe(1);
  const wt = path.join(worktrees, fs.readdirSync(worktrees)[0]);
  const agentName = (await page.locator('.card .name').first().textContent())!.trim();
  // A folder of the project only, not of the agent's branch.
  fs.mkdirSync(path.join(app.repo, 'docs'));

  // The header's « Commandes de lancement… »: in a short window (the runner's), the section folds and « Configurer » with it.
  await page.getByRole('button', { name: 'Commandes de lancement…', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const serveurCfg = dialog.getByRole('group', { name: 'Commande 1' });
  await serveurCfg.getByLabel('Nom').fill('Serveur');
  await serveurCfg.getByLabel('Commande').fill('Set-Content serveur.txt "port=$env:ESCOUADE_PORT_BASE"; Start-Sleep 600');
  await serveurCfg.getByLabel(/Sous-dossier/).fill('src');
  await dialog.getByRole('button', { name: 'Ajouter une commande', exact: true }).click();
  const docsCfg = dialog.getByRole('group', { name: 'Commande 2' });
  await docsCfg.getByLabel('Nom').fill('Docs');
  await docsCfg.getByLabel('Commande').fill('Start-Sleep 600');
  await docsCfg.getByLabel(/Sous-dossier/).fill('docs');
  await closeSettings(dialog);

  // ▶ asks where (on hover).
  await unfoldSections(page);
  const serveur = page.locator('.run').filter({ has: page.locator('.name', { hasText: /^Serveur$/ }) });
  await serveur.hover();
  await serveur.getByRole('button', { name: 'Lancer' }).click();
  await expect(page.getByRole('menuitem', { name: 'Dans le projet' })).toBeVisible();
  await page.getByRole('menuitem', { name: `Dans le worktree de ${agentName}` }).click();
  // In the same folder of the worktree, with the agent's block of ports, under its name.
  const inWt = page.locator('.run', { hasText: `Serveur · ${agentName}` });
  await expect(inWt).toContainText('en cours');
  const marker = path.join(wt, 'src', 'serveur.txt');
  await expect.poll(() => (fs.existsSync(marker) ? fs.readFileSync(marker, 'utf8') : ''), { timeout: 30_000 }).toMatch(/port=41\d0/);
  expect(fs.existsSync(path.join(app.repo, 'src', 'serveur.txt'))).toBe(false);
  await expect(serveur).toContainText('prêt');
  // Its log says where it runs.
  await inWt.click();
  await expect(page.locator('main.rv h1')).toHaveText('Serveur');
  await expect(page.locator('main.rv .crumbs')).toContainText(`dans le worktree de ${agentName}`);

  // The same command in the project: the user is told first, and runs both.
  await serveur.hover();
  await serveur.getByRole('button', { name: 'Lancer' }).click();
  await page.getByRole('menuitem', { name: 'Dans le projet' }).click();
  const confirm = page.getByRole('dialog', { name: '« Serveur » tourne déjà' });
  await expect(confirm).toContainText(`dans le worktree de ${agentName}`);
  await confirm.getByRole('button', { name: 'Lancer quand même' }).click();
  await expect(serveur).toContainText('en cours');
  await expect(inWt).toContainText('en cours');
  await expect.poll(() => fs.existsSync(path.join(app.repo, 'src', 'serveur.txt')), { timeout: 30_000 }).toBe(true);

  // A folder the worktree lacks: refused, with why.
  const docs = page.locator('.run').filter({ has: page.locator('.name', { hasText: /^Docs$/ }) });
  await docs.hover();
  await docs.getByRole('button', { name: 'Lancer' }).click();
  await page.getByRole('menuitem', { name: `Dans le worktree de ${agentName}` }).click();
  await expect(page.getByText(`Le dossier « docs » de « Docs » n'existe pas dans le worktree de « ${agentName} »`)).toBeVisible();
  await expect(page.locator('.run', { hasText: `Docs · ${agentName}` })).toHaveCount(0);

  // Archived, its agent takes it along; the project's run goes on.
  await page.locator('.card').first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: /^Archiver/ }).click();
  await expect(inWt).toContainText('arrêté', { timeout: 15_000 });
  await expect(page.getByText(/s'est arrêté en erreur/)).toHaveCount(0);
  await expect(serveur).toContainText('en cours');
  await page.getByRole('button', { name: 'Tout arrêter', exact: true }).click();
  await expect(serveur).toContainText('arrêté', { timeout: 15_000 });
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
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Avancé');
  await dialog.getByPlaceholder('aucun').fill('http://proxy.local:3128');
  await closeSettings(dialog);
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  expect(app.launches().at(-1)?.proxy).toBe('http://proxy.local:3128');
});

test('Ctrl+N creates another agent, Ctrl+J gives the focus to the card of the one waiting, and Enter opens it', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'question');
  await expect(page.getByTestId('question-pending')).toBeVisible();
  await page.keyboard.press('Control+n');
  await expect(page.locator('.card')).toHaveCount(2);
  await expect(page.getByText('Agent prêt')).toBeVisible();
  await page.keyboard.press('Control+j');
  await expect(page.locator('aside.side .agent.waiting')).toBeFocused();
  await expect(page.getByTestId('question-pending')).toBeHidden();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('question-pending')).toBeVisible();
});

test('a project’s color is picked from its tile’s menu, not from the sidebar', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await expect(page.locator('aside.side')).not.toContainText('Couleur');
  const tile = page.getByRole('tab', { name: /^demo-api/ });
  await tile.click({ button: 'right' });
  const third = page.getByRole('menuitemradio', { name: 'Couleur 3' });
  const color = await third.evaluate((el) => getComputedStyle(el).backgroundColor);
  await third.click();
  await expect(page.getByRole('menu')).toBeHidden();
  await expect(tile).toHaveCSS('background-color', color);
  await tile.click({ button: 'right' });
  await expect(page.getByRole('menuitemradio', { name: 'Couleur 3' })).toHaveAttribute('aria-checked', 'true');
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
  await expect(page.getByText('Non enregistré', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+S');
  await expect(page.getByText('Enregistré', { exact: true })).toBeVisible();
  expect(fs.readFileSync(path.join(app.repo, 'src', 'app.ts'), 'utf8')).toBe('const a = 1;\nexport const b = 2;');
  await page.getByRole('button', { name: 'Conversation', exact: true }).click();
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
  // The folder is there before git has checked its files out: its file is waited for.
  await expect.poll(() => fs.existsSync(path.join(wt, 'src', 'app.ts'))).toBe(true);
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'const a = 42;\n');
  // Ctrl+Maj+B: the header puts « Fichiers » away in a narrow column (the runner's 1024 px screen), « … » then has it.
  await page.keyboard.press('Control+Shift+B');
  // The row's editor button shows on hover.
  await page.locator('.filerow').filter({ hasText: 'app.ts' }).hover();
  await page.getByRole('button', { name: 'Ouvrir app.ts dans l’éditeur' }).click();
  await expect(page.locator('.cm-content')).toContainText('const a = 42;');
  await expect(page.locator('.crumbs .vs')).toHaveText('1 ligne modifiée par rapport à main');
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
      // Two rows since ESC-390: the settings end inside the header, the switch above them.
      if (cfg.right > auto.right + 0.5) problems.push(`button ends at ${cfg.right}, past the switch at ${auto.right}`);
      if (cfg.top < auto.bottom) problems.push(`button at ${cfg.top}, over the switch's row ending at ${auto.bottom}`);
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
    ['Aucun harness trouvé — aucun ticket ne démarre', true],
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
  await expect(settingsEntry(settings, 'Kanban')).toHaveAttribute('aria-current', 'page');
  await expect(settingsEntry(settings, 'demonstration-d-api')).toHaveAttribute('aria-expanded', 'true');
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
  // Both criteria met, each checked.
  await expect(review.locator('.crit li.met')).toHaveCount(2);
  // A met criterion is a plain line, not a green button.
  await expect(review.getByRole('list', { name: 'Critères' }).getByRole('listitem').first()).toHaveCSS(
    'background-color',
    'rgba(0, 0, 0, 0)',
  );
  // The card's first line keeps its key, and its loops and cost, inside the card; its buttons too.
  const inside = () =>
    review.locator('.card').evaluate((card) => {
      const c = card.getBoundingClientRect();
      const problems: string[] = [];
      for (const child of card.querySelectorAll('.top > *, .row > *')) {
        const r = child.getBoundingClientRect();
        if (r.left < c.left || r.right > c.right)
          problems.push(`${child.className} lies ${r.left}..${r.right}, the card ${c.left}..${c.right}`);
      }
      return problems;
    });
  expect(await inside()).toEqual([]);
  expect(await sticksOut()).toEqual([]);
  await review.getByRole('button', { name: 'Valider et merger' }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('Mergé dans main · squash')).toBeVisible({ timeout: 60_000 });
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

  await review.getByRole('button', { name: 'Tester' }).click();
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
  await expect(done.getByText('Mergé dans main · squash')).toBeVisible({ timeout: 60_000 });
  // Done: its terminal is gone from the section, its process with it, and neither crashed.
  await expect(runs.locator('.group')).toHaveCount(0);
  await expect(runs.locator('.run')).toHaveCount(0);
  await expect.poll(() => alive(pid), { timeout: 15_000 }).toBe(false);
  await expect(page.getByText(/s'est arrêté en erreur/)).toHaveCount(0);
});

test('a deleted ticket’s worktree holding work stays, its archived agent says why, and « Nettoyer » removes it once nothing is left (ESC-143)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Fichier [ok]');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  // The fake agent left its file there, not committed.
  const root = path.join(app.repo, '.claude', 'worktrees');
  const worktree = path.join(
    root,
    fs.readdirSync(root).find((n) => n.includes('dem-1'))!,
  );
  expect(fs.existsSync(path.join(worktree, 'dem-1.txt'))).toBe(true);
  await review.getByRole('button', { name: /DEM-1/ }).first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Supprimer' }).click();
  const confirm = page.getByRole('dialog', { name: 'Supprimer le ticket DEM-1 ?' });
  await expect(confirm).toContainText(
    "Son agent est archivé et son worktree supprimé, sauf s'il garde des changements non commités ou des commits ni mergés ni poussés.",
  );
  await confirm.getByRole('button', { name: 'Supprimer' }).click();
  await expect(review.getByText('DEM-1', { exact: true })).toHaveCount(0);
  // Kept, and its archived agent's card says why.
  await page.getByRole('button', { name: 'Archivés (1)' }).click();
  const archived = page.locator('.card.archived');
  await expect(archived).toContainText('worktree gardé : 1 fichier(s) non commité(s)');
  expect(fs.existsSync(path.join(worktree, 'dem-1.txt'))).toBe(true);
  // Its file thrown away by hand: « Nettoyer » removes the worktree, and says so.
  fs.rmSync(path.join(worktree, 'dem-1.txt'));
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Git et worktrees');
  await settings.getByRole('button', { name: 'Nettoyer' }).click();
  await expect(settings.getByRole('status').filter({ hasText: 'worktree' })).toHaveText('1 worktree supprimé.', { timeout: 30_000 });
  await expect.poll(() => fs.existsSync(worktree)).toBe(false);
  await closeSettings(settings);
  await expect(archived).not.toContainText('worktree gardé');
});

test('a ticket is written in a large window, a long description included, and added with Ctrl+Enter', async ({ app }) => {
  const { page } = app;
  // The window of the CI runners, close to the narrowest the app allows: the window must fit in it.
  await page.setViewportSize({ width: 1028, height: 779 });
  execFileSync('git', ['branch', 'feat/e2e-ticket'], { cwd: app.repo });
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  // Nothing starts: the ticket stays "À faire", to be opened again.
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  const plus = page.getByRole('button', { name: 'Nouveau ticket' });
  // A first ticket (DEM-1), for the new one to come after.
  await plus.click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Le premier');
  await page.keyboard.press('Control+Enter');
  await expect(page.getByRole('button', { name: /DEM-1/ })).toBeVisible();
  await plus.click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  const title = dialog.getByRole('textbox', { name: 'Titre du ticket' });
  const description = dialog.getByRole('textbox', { name: 'Description' });
  await expect(title).toBeFocused();
  await expect(dialog.getByText('Ctrl+Entrée pour ajouter')).toBeVisible();
  // In a short window the loops, the branch and the dependencies show at once: the window's body has nothing to scroll.
  for (const name of ['Boucles max', 'Branche', 'Après']) {
    await expect(dialog.getByRole('group', { name })).toBeVisible();
  }
  expect(await dialog.locator('.body').evaluate((body) => body.scrollHeight - body.clientHeight)).toBeLessThanOrEqual(1);
  // In a tall one the footer stays at the foot of the window, whatever the content leaves free.
  await page.setViewportSize({ width: 1400, height: 1100 });
  const [foot, whole] = await Promise.all([dialog.locator('.foot').boundingBox(), dialog.boundingBox()]);
  expect(Math.abs(foot!.y + foot!.height - (whole!.y + whole!.height))).toBeLessThanOrEqual(2);
  await page.setViewportSize({ width: 1028, height: 779 });
  // Large, and inside the window.
  const box = (await dialog.boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(740);
  expect(box.height).toBeGreaterThanOrEqual(600);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(1028);
  expect(box.y + box.height).toBeLessThanOrEqual(779);
  const long = Array.from(
    { length: 80 },
    (_, i) => `Ligne ${i + 1} : une phrase assez longue pour remplir la ligne du champ et obliger à défiler`,
  ).join('\n');
  await title.fill('Un ticket écrit en grand');
  await description.fill(long);
  // The description keeps the room the window leaves it and scrolls in itself (ESC-189): the criteria under it, and the
  // buttons, stay in reach.
  expect(await description.evaluate((d) => d.scrollHeight > d.clientHeight + 100)).toBe(true);
  const [field, criteria] = await Promise.all([
    description.boundingBox(),
    dialog.getByRole('textbox', { name: "Critères d'acceptation" }).boundingBox(),
  ]);
  expect(criteria!.y).toBeGreaterThanOrEqual(field!.y + field!.height);
  await expect(dialog.getByRole('button', { name: 'Ajouter', exact: true })).toBeInViewport();
  // Escape asks first; « Annuler » brings the window back with everything in it.
  await page.keyboard.press('Escape');
  const ask = page.getByRole('dialog', { name: 'Abandonner les modifications ?' });
  await expect(ask).toBeVisible();
  await ask.getByRole('button', { name: 'Annuler' }).click();
  await expect(dialog).toBeVisible();
  await expect(title).toHaveValue('Un ticket écrit en grand');
  await expect(description).toHaveValue(long);
  // The branch picker opens over the window: a click on one of its rows would miss under the overlay.
  await dialog.getByRole('button', { name: /^Branche/ }).click();
  await page.getByRole('menuitem', { name: 'Reprendre une branche existante…' }).click();
  const picker = page.getByRole('dialog', { name: 'Choisir une branche' });
  await picker.getByText('feat/e2e-ticket').click();
  await expect(picker).toBeHidden();
  await expect(dialog.getByRole('button', { name: /^Branche/ })).toContainText('feat/e2e-ticket');
  await expect(dialog).toBeVisible();
  // Ctrl+Enter adds the ticket from wherever the focus is; the focus goes back to « + ».
  await page.keyboard.press('Control+Enter');
  await expect(dialog).toBeHidden();
  await expect(plus).toBeFocused();
  // Opened again from its card: nothing was lost on the way.
  await page.getByRole('button', { name: /DEM-2/ }).click();
  const edit = page.getByRole('dialog', { name: 'Modifier DEM-2' });
  await expect(edit.getByRole('textbox', { name: 'Description' })).toHaveValue(long);
  await expect(edit.getByRole('button', { name: /^Branche/ })).toContainText('feat/e2e-ticket');
  await expect(edit.getByText('Ctrl+Entrée pour enregistrer')).toBeVisible();
  // Nothing changed: Escape leaves at once.
  await page.keyboard.press('Escape');
  await expect(edit).toBeHidden();
  await expect(page.getByRole('button', { name: /DEM-2/ })).toBeFocused();
});

test('an image pasted or dropped in a ticket is kept in its folder, shown on its card, and reaches its agent with its first message', async ({
  app,
}) => {
  const { page } = app;
  // The window of the CI runners: the images do not make the form scroll.
  await page.setViewportSize({ width: 1028, height: 779 });
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  // Nothing starts before « Lancer ».
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Corriger le bouton [ok]');
  const png = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];
  const gif = [71, 73, 70, 56, 57, 97];
  // As WebView2 gives them: a screenshot pasted is a PNG file; a file dropped comes with the drop.
  const give = (field: Locator, kind: 'paste' | 'drop', file: { name: string; type: string; bytes: number[] }) =>
    field.evaluate(
      (el, { kind, name, type, bytes }) => {
        const data = new DataTransfer();
        data.items.add(new File([new Uint8Array(bytes)], name, { type }));
        const init = { bubbles: true, cancelable: true };
        el.dispatchEvent(
          kind === 'paste'
            ? new ClipboardEvent('paste', { ...init, clipboardData: data })
            : new DragEvent('drop', { ...init, dataTransfer: data }),
        );
      },
      { kind, ...file },
    );
  await give(dialog.getByRole('textbox', { name: 'Description' }), 'paste', { name: 'image.png', type: 'image/png', bytes: png });
  await give(dialog.getByRole('textbox', { name: "Critères d'acceptation" }), 'drop', {
    name: 'maquette.gif',
    type: 'image/gif',
    bytes: gif,
  });
  const images = dialog.getByRole('list', { name: 'Images du ticket' });
  await expect(images.getByRole('img', { name: 'image.png' })).toBeVisible();
  await expect(images.getByRole('img', { name: 'maquette.gif' })).toBeVisible();
  expect(await dialog.locator('.body').evaluate((body) => body.scrollHeight - body.clientHeight)).toBeLessThanOrEqual(1);
  await dialog.getByRole('button', { name: 'Retirer maquette.gif' }).click();
  await expect(images.getByRole('img')).toHaveCount(1);
  await page.keyboard.press('Control+Enter');
  await expect(dialog).toBeHidden();

  // The card says it has one; the image is a file of the ticket's folder, out of the JSON of the tickets.
  const card = page.getByRole('button', { name: 'DEM-1 Corriger le bouton [ok]', exact: true });
  await expect(card).toHaveAccessibleDescription('1 image jointe');
  await expect(card.locator('.clip')).toBeVisible();
  const folders = path.join(app.data, 'tickets');
  await expect.poll(() => fs.existsSync(folders) && fs.readdirSync(folders).length).toBe(1);
  const folder = path.join(folders, fs.readdirSync(folders)[0]);
  const files = fs.readdirSync(folder);
  expect(files).toHaveLength(1);
  expect([...fs.readFileSync(path.join(folder, files[0]))]).toEqual(png);
  const base64 = Buffer.from(png).toString('base64');
  await expect.poll(() => fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).toContain(files[0]);
  expect(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).not.toContain(base64);

  // Opened again, it shows it.
  await card.click();
  const edit = page.getByRole('dialog', { name: 'Modifier DEM-1' });
  await expect(edit.getByRole('img', { name: 'image.png' })).toHaveAttribute('src', `data:image/png;base64,${base64}`);
  await page.keyboard.press('Escape');
  await expect(edit).toBeHidden();

  // Its agent gets it before the text of its first message.
  await card.getByRole('button', { name: 'Lancer' }).click();
  const stdin = path.join(path.dirname(app.data), 'fake-claude.stdin.jsonl');
  const first = () =>
    fs.existsSync(stdin)
      ? fs
          .readFileSync(stdin, 'utf8')
          .trim()
          .split('\n')
          .map((l) => JSON.parse(l))
          .find((m) => Array.isArray(m.message?.content))?.message.content
      : undefined;
  await expect.poll(first, { timeout: 30_000 }).toBeTruthy();
  const content = first();
  expect(content[0]).toEqual({ type: 'image', source: { type: 'base64', media_type: 'image/png', data: base64 } });
  expect(content[1].text).toMatch(/^Ticket DEM-1 : Corriger le bouton \[ok\]/);
  await expect(card).toHaveAccessibleDescription('1 image jointe');
});

test.describe('the first start', () => {
  test.use({ preStart: { run: () => ({ telemetry: null }) } });

  test('asks about the usage statistics, and keeps the answer', async ({ app }) => {
    const { page } = app;
    const dialog = page.getByRole('dialog', { name: 'Aider à améliorer Escouade ?' });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Non merci' }).click();
    await expect(dialog).toBeHidden();
    const settings = () => JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
    await expect.poll(() => settings().telemetry).toBe(false);
    expect(fs.existsSync(path.join(app.data, 'telemetry-id'))).toBe(false);
  });
});
