import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

// « Récupérer automatiquement (git fetch) » (ESC-561): a repository whose remote moves on shows it by itself, at the
// interval set, the Git panel saying when it was fetched; switched off, it no longer does. A minute of the interval is a
// second here (`ESCOUADE_E2E_FETCH_MINUTE_MS`, a debug build's): the default 5 min are 5 s.
test.use({ appEnv: { ESCOUADE_E2E_FETCH_MINUTE_MS: '1000' } });

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

test('the background fetch shows the commits to pull with no gesture, and switched off it no longer does', async ({ app }) => {
  test.setTimeout(150_000);
  const { page } = app;
  await addProject(page, app.repo);
  // A local remote, and a clone of it that pushes what the project does not have.
  const bare = `${app.repo}-remote.git`;
  const other = path.join(path.dirname(app.repo), `${path.basename(app.repo)}-other`);
  git(app.repo, 'init', '-q', '--bare', '-b', 'main', bare);
  git(app.repo, 'remote', 'add', 'origin', bare);
  git(app.repo, 'push', '-qu', 'origin', 'main');
  git(path.dirname(app.repo), 'clone', '-q', bare, other);
  const push = (msg: string) => {
    git(other, '-c', 'user.name=e2e', '-c', 'user.email=e2e@example.invalid', 'commit', '-q', '--allow-empty', '-m', msg);
    git(other, 'push', '-q');
  };
  push('theirs 1');

  // No gesture: the background fetch (15 s after the start, then every 5 « minutes ») brings it.
  const branch = page.locator('footer .sync');
  await expect(branch.getByText('1 commit à tirer')).toBeAttached({ timeout: 60_000 });
  // At the interval set, again: a second commit shows by itself.
  push('theirs 2');
  await expect(branch.getByText('2 commits à tirer')).toBeAttached({ timeout: 30_000 });

  // The Git panel says when it was fetched, beside « Fetch ».
  await branch.click();
  const picker = page.getByRole('dialog', { name: 'Branches' });
  await expect(picker.getByTestId('fetch-line')).toContainText(/Récupéré (à l’instant|il y a \d+ min)/);
  await page.keyboard.press('Escape');

  // Switched off: a commit pushed no longer shows, interval after interval.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Tous les projets');
  const toggle = settings.getByRole('switch', { name: 'Récupérer automatiquement (git fetch)' });
  await expect(toggle).toBeChecked();
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await closeSettings(settings);
  // A fetch whose git had already started when it was switched off may end: it is not a new one.
  await page.waitForTimeout(3_000);
  push('theirs 3');
  // Three intervals and more.
  await page.waitForTimeout(18_000);
  await expect(branch.getByText('2 commits à tirer')).toBeAttached();
  // Back on (no restart): it shows.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  await openSettingsPage(settings, 'Tous les projets');
  await settings.getByRole('switch', { name: 'Récupérer automatiquement (git fetch)' }).click();
  await closeSettings(settings);
  await expect(branch.getByText('3 commits à tirer')).toBeAttached({ timeout: 30_000 });
});
