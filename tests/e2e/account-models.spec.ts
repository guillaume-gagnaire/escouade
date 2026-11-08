// The models of an account read without an agent (ESC-186), for Copilot CLI, Cursor CLI and OpenCode as for Codex
// (codex.spec.ts): the fiche of the account offers its harness's list before any agent ran, in the runner's window.

import { addProject, expect, FAKE_COPILOT, FAKE_CURSOR, FAKE_OPENCODE, test, openSettingsPage } from './fixture';

const CASES: { harness: string; env: Record<string, string>; models: string[] }[] = [
  { harness: 'Copilot CLI', env: { ESCOUADE_COPILOT: FAKE_COPILOT }, models: ['GPT-6 Luna', 'Claude Sonnet 4.5', 'GPT-5 mini'] },
  { harness: 'Cursor CLI', env: { ESCOUADE_CURSOR: FAKE_CURSOR }, models: ['Auto', 'Claude Opus 5.5', 'GPT-5.5', 'Composer 2.5'] },
  { harness: 'OpenCode', env: { ESCOUADE_OPENCODE: FAKE_OPENCODE }, models: ['gpt-oss 20b', 'Qwen3 Coder 30b', 'Devstral Small'] },
];

for (const c of CASES)
  test.describe(`a ${c.harness} account`, () => {
    test.use({ appEnv: c.env });

    test(`offers the models of its account before any agent, in the runner’s window (ESC-186)`, async ({ app }) => {
      const { page } = app;
      await page.setViewportSize({ width: 1024, height: 728 });
      await addProject(page, app.repo);
      await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
      const dialog = page.getByRole('dialog', { name: 'Réglages' });
      await openSettingsPage(dialog, 'Comptes et harness');
      const fiche = dialog.getByRole('list', { name: `Comptes ${c.harness}` }).getByRole('listitem', { name: 'Principal' });
      const model = fiche.getByRole('group', { name: 'Nouveaux agents' }).getByRole('combobox', { name: 'Modèle' });
      await model.scrollIntoViewIfNeeded();
      await expect(model).toBeInViewport();
      for (const m of c.models) await expect(model.getByRole('option', { name: new RegExp(m, 'i') }).first()).toBeAttached();
    });
  });
