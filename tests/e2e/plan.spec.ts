import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

// The fake CLI plays a superpowers plan run (« plan superpowers »: tests/fixtures/superpowers-run.md): the plan and
// its ledger written in the agent's folder, a task list, a subagent that implements and one that reviews each task,
// one that goes on in the background, and a question in the middle of the third task.

const PLAN = 'docs/superpowers/plans/2026-10-10-demo.md';

/** The “Plan” banner of the conversation and what it holds. */
function banner(page: Page) {
  const root = page.locator('main.conv .plan');
  return {
    root,
    head: root.getByRole('button', { name: /^Plan : / }),
    list: root.getByRole('list', { name: 'Tâches du plan' }),
    row: (n: number) => root.locator(`li[data-plan-task="${String(n).padStart(2, '0')}"]`),
    grip: root.getByRole('separator', { name: 'Hauteur de la liste des tâches' }),
  };
}

/** Waits for the question of the third task, and answers it. */
async function answer(page: Page) {
  const card = page.getByTestId('question-pending');
  await expect(card).toContainText('Quelle base de données ?');
  await card.getByRole('button', { name: 'SQLite' }).click();
}

test('a superpowers plan run shows the “Plan” banner between the header and the messages, then goes to 3/3', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'plan superpowers');

  // The third task waits for an answer: two tasks done out of three, one subagent still working in the background.
  await expect(page.getByTestId('question-pending')).toContainText('Quelle base de données ?');
  const plan = banner(page);
  await expect(plan.head).toBeVisible();
  await expect(plan.root).toContainText('2/3 tâches');
  await expect(plan.root).toContainText('67 %');
  await expect(plan.root).toContainText('1 sous-agent actif');
  // The title is the plan's own (the H1 of the file the agent wrote, without “Implementation Plan”).
  await expect(plan.head).toHaveAccessibleName('Plan : Démo');
  await expect(plan.root.locator('.title')).toHaveText('Démo');
  // The agent’s own list is the list: the file is named in the tooltip when the file is the list (below).
  await expect(plan.root.locator('.title')).toHaveAttribute('title', 'Démo');

  // Between the agent's header and the messages.
  const header = (await page.locator('main.conv header.head').boundingBox())!;
  const box = (await plan.root.boundingBox())!;
  const messages = (await page.locator('main.conv .scroll').boundingBox())!;
  expect(box.y).toBeGreaterThanOrEqual(header.y + header.height - 1);
  expect(messages.y).toBeGreaterThanOrEqual(box.y + box.height - 1);

  // The card of the agent says as much, in a line of its own with a bar.
  const mini = page.locator('.card .plan-mini').first();
  await expect(mini.locator('.plan-text')).toHaveText('Plan 2/3 · 1 sous-agent');
  await expect(mini.locator('.plan-bar span')).toHaveAttribute('style', /width: 66\.\d+%|width: 67%/);

  // Closed, then open, with the keyboard.
  const head = plan.head;
  if ((await head.getAttribute('aria-expanded')) === 'true') {
    await head.focus();
    await page.keyboard.press('Enter');
  }
  await expect(head).toHaveAttribute('aria-expanded', 'false');
  await expect(plan.list).toBeHidden();
  await head.focus();
  await page.keyboard.press('Enter');
  await expect(head).toHaveAttribute('aria-expanded', 'true');
  await expect(plan.list).toBeVisible();

  // The list: a rank, a state in words and the title of each task.
  await expect(plan.row(1)).toContainText('Terminé');
  await expect(plan.row(1)).toContainText('Écrire la fonction');
  await expect(plan.row(2)).toContainText('Terminé');
  // The task that was to be looked at is the one the agent waits on, and the row says so.
  const third = plan.row(3);
  await expect(third).toHaveClass(/\bfocus\b/);
  await expect(third).toContainText('Bloqué');
  await expect(third).toContainText('Brancher dans l’interface');
  await expect(third).toContainText('En attente de ta réponse');
  await expect(plan.row(1)).not.toContainText('En attente de ta réponse');

  // The handle under the list sets its height, with the arrow keys; Entrée gives the usual size back.
  const grip = plan.grip;
  await expect(grip).toBeVisible();
  const before = Number(await grip.getAttribute('aria-valuenow'));
  await grip.focus();
  await page.keyboard.press('ArrowDown');
  await expect(grip).toHaveAttribute('aria-valuenow', String(before + 16));
  await expect(plan.list).toHaveClass(/\bsized\b/);
  await page.keyboard.press('Enter');
  await expect(plan.list).not.toHaveClass(/\bsized\b/);

  // Answered: the plan is done, the banner says so and the background subagent ends by itself.
  await answer(page);
  await expect(page.getByText('Plan terminé')).toBeVisible();
  await expect(plan.root).toContainText('3/3 tâches');
  await expect(plan.root).toContainText('100 %');
  for (const n of [1, 2, 3]) await expect(plan.row(n)).toContainText('Terminé');
  await expect(plan.row(3)).not.toContainText('En attente de ta réponse');
  await expect(plan.root).not.toContainText('sous-agent actif');
  await expect(mini.locator('.plan-text')).toHaveText('Plan 3/3');
  await expect(mini.locator('.plan-bar span')).toHaveAttribute('style', /width: 100%/);
  // It stays as it was left.
  await expect(head).toHaveAttribute('aria-expanded', 'true');

  // The files were written for real, in the agent's folder (the repository); the run ended by removing its workspace.
  expect(fs.readFileSync(path.join(app.repo, ...PLAN.split('/')), 'utf8')).toContain('# Démo Implementation Plan');
  expect(fs.existsSync(path.join(app.repo, '.superpowers', 'sdd', '2026-10-10-demo'))).toBe(false);
  // The agent was started with the task list tools (the setting is on by default), the question that names it was not.
  const [agent, ...asked] = app.launches().filter((l) => !l.argv.includes('-p'));
  expect(agent).toMatchObject({ todoTools: '1' });
  expect(asked).toHaveLength(0);
  expect(
    app
      .launches()
      .filter((l) => l.argv.includes('-p'))
      .every((l) => l.todoTools === null),
  ).toBe(true);
});

test('a superpowers plan run without a task list takes its tasks from the plan file and the ledger', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'plan superpowers sans liste');

  // No task tool in the conversation: the banner comes from the plan and the ledger.
  await expect(page.getByTestId('question-pending')).toContainText('Quelle base de données ?');
  const plan = banner(page);
  await expect(plan.head).toHaveAccessibleName('Plan : Démo');
  await expect(plan.root.locator('.title')).toHaveAttribute('title', `Démo\nPlan : ${PLAN}`);
  await expect(plan.root).toContainText('2/3 tâches');
  await expect(plan.root).toContainText('1 sous-agent actif');
  await expect(plan.row(1)).toContainText('Écrire la fonction');
  await expect(plan.row(1)).toContainText('Terminé');
  await expect(plan.row(2)).toContainText('Terminé');
  await expect(plan.row(3)).toContainText('Brancher dans l’interface');
  // The steps the plan counts: none of the third task's is checked.
  await expect(plan.row(3).locator('.rpct')).toHaveAttribute('title', '0/2 étapes');
  await expect(plan.row(3)).toContainText('En attente de ta réponse');
  await expect(page.locator('.card .plan-mini .plan-text').first()).toHaveText('Plan 2/3 · 1 sous-agent');
  await expect(page.locator('main.conv')).not.toContainText('Tâche : Écrire la fonction');

  await answer(page);
  await expect(page.getByText('Plan terminé')).toBeVisible();
  await expect(plan.root).toContainText('3/3 tâches');
  await expect(plan.root).toContainText('100 %');
  await expect(page.locator('.card .plan-mini .plan-text').first()).toHaveText('Plan 3/3');
});

test('a superpowers plan run cut in the middle of a subagent counts none as running', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'plan superpowers interrompu');
  const plan = banner(page);
  await expect(plan.head).toBeVisible();
  await expect(plan.root).toContainText('0/3 tâches');
  await expect(page.getByText(/Le tour s'est terminé en erreur/)).toBeVisible();
  // The implementer was cut: it is no longer at work, and nothing says it is.
  await expect(plan.root).not.toContainText('sous-agent actif');
  await expect(page.locator('.card .plan-mini .plan-text').first()).toHaveText('Plan 0/3');
});
