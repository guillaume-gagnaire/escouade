import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';

// A harness project by project (ESC-404): in a private project, the first agent on Claude Code asks first, « Annuler »
// starts nothing, « Activer » turns its plugin on for the project and the agent goes; in a project that is not private
// where Claude Code is turned off, it is not offered and no agent starts there. Run at the CI's 1024 × 728 too
// (ESCOUADE_E2E_VIEWPORT).

/** The launches of the fake `claude` that are an agent's process: not a one-shot question (`-p`), nor a reading of the models. */
const agentLaunches = (launches: { argv: string[] }[]) =>
  launches.filter((l) => !l.argv.includes('-p') && !l.argv.includes('--no-session-persistence')).length;

test('a private project asks before its first Claude Code agent: « Annuler » starts nothing, « Activer » starts it', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { firstAgent: false });
  const dialog = await openSettingsPage(page, 'Confidentialité');
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  await closeSettings(dialog);

  const ask = page.getByRole('dialog', { name: 'Activer Claude Code pour ce projet privé ?' });
  const agents = page.locator('.agents .card');
  await page.locator('button.new-agent').click();
  await expect(ask).toBeVisible();
  // What its plugin will get, and its permissions, as the install window says them.
  await expect(ask).toContainText('Ce que l’agent écrit et lit dans ce projet');
  await expect(ask.locator('.asks li').first()).toBeVisible();
  await ask.getByRole('button', { name: 'Annuler' }).click();
  await expect(ask).toBeHidden();
  await expect(agents).toHaveCount(0);
  expect(agentLaunches(app.launches())).toBe(0);

  await page.locator('button.new-agent').click();
  await ask.getByRole('button', { name: 'Activer' }).click();
  await expect(ask).toBeHidden();
  await expect(agents).toHaveCount(1);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();
  // Asked once: the next agent goes at once.
  await page.locator('button.new-agent').click();
  await expect(agents).toHaveCount(2);
  await expect(ask).toBeHidden();
});

test('a project where Claude Code is turned off does not offer it, and its agent says so with « Activer »', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();

  // Vue Plugins › Claude Code › Projets: turned off for demo-api.
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .getByRole('complementary', { name: 'Plugins' })
    .getByRole('button', { name: /Installés/ })
    .click();
  await page
    .getByRole('button', { name: /^Claude Code/ })
    .first()
    .click();
  await page.getByRole('tab', { name: /Projets/ }).click();
  const sw = page.getByRole('tabpanel').getByRole('switch', { name: 'Actif dans demo-api' });
  await expect(sw).toHaveAttribute('aria-checked', 'true');
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'false');

  // Back to the project: not offered in its menu « Harness », its agent held, with « Activer ».
  await page.getByRole('tab', { name: /^demo-api/ }).click();
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  const menu = page.getByRole('menu');
  await expect(menu).toBeVisible();
  await expect(menu.getByRole('menuitem', { name: /^Claude Code/ })).toHaveCount(0);
  await page.keyboard.press('Escape');
  const held = page.getByTestId('harness-held');
  await expect(held).toContainText('Claude Code est coupé pour ce projet');
  // Nothing goes to it.
  const before = agentLaunches(app.launches());
  await page.locator('button.new-agent').click();
  await expect(page.locator('.agents .card')).toHaveCount(1);
  expect(agentLaunches(app.launches())).toBe(before);
  // « Activer »: the same agreement, then it talks again.
  await held.getByRole('button', { name: 'Activer' }).click();
  const ask = page.getByRole('dialog', { name: 'Activer Claude Code pour demo-api ?' });
  await ask.getByRole('button', { name: 'Activer' }).click();
  await expect(held).toBeHidden();
  await send(page, 'Encore');
  await expect(page.getByText('Bonjour, tu as dit : Encore')).toBeVisible();
});
