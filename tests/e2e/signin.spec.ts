import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, send, test } from './fixture';

// A turn stopped by a sign-in Claude Code could not refresh (ESC-162). Principal is the app's own Claude Code folder, a
// test one; the fake `claude` fails a turn as Claude Code does while that folder holds a `fake-oauth-lost` file, and the
// process that failed so fails every turn after (its refresh token dead in its memory). Never the user's sign-in.

/** A sign-in in date, `tag` telling one from another. */
const signedIn = (tag: string) =>
  JSON.stringify({ claudeAiOauth: { accessToken: `tok-${tag}`, refreshToken: `ref-${tag}`, expiresAt: 4_102_444_800_000 } });

const principalDir = (root: string) => path.join(root, 'claude');

/** Whether a launch of the fake `claude` is an agent's process (not `claude -p`, nor a reading of the models). */
const isAgent = (l: { argv: string[] }) => !l.argv.includes('-p') && !l.argv.includes('--no-session-persistence');

test.describe('a good sign-in on disk', () => {
  test.use({
    appEnv: { ESCOUADE_SIGNIN_RETRY_MS: '200' },
    preStart: {
      run: (root) => {
        fs.mkdirSync(principalDir(root), { recursive: true });
        fs.writeFileSync(path.join(principalDir(root), '.credentials.json'), signedIn('a'));
        fs.writeFileSync(path.join(principalDir(root), 'fake-oauth-lost'), '1');
      },
    },
  });

  test('the agent goes on by itself, its process started again on its session, in one neutral line', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await send(page, 'Bonjour');
    await expect(page.getByText('Session du compte rafraîchie : l’agent reprend.')).toBeVisible();
    await expect(page.getByText('Bonjour, tu as dit : continue')).toBeVisible();
    // No red card, no sign-in to do.
    await expect(page.getByText("Le tour s'est terminé en erreur")).toBeHidden();
    await expect(page.getByTestId('turn-sign-in')).toBeHidden();
    await expect(page.getByText(/OAuth session expired/)).toBeHidden();
    const agents = app.launches().filter(isAgent);
    expect(agents).toHaveLength(2);
    expect(agents[1].argv.some((a) => a.startsWith('--resume='))).toBe(true);
  });
});

test.describe('a sign-in emptied by Claude Code', () => {
  test.use({
    preStart: {
      run: (root) => {
        fs.mkdirSync(principalDir(root), { recursive: true });
        // As Claude Code leaves it when its refresh token is refused.
        fs.writeFileSync(
          path.join(principalDir(root), '.credentials.json'),
          JSON.stringify({ claudeAiOauth: { accessToken: '', refreshToken: '', expiresAt: 0 } }),
        );
        fs.writeFileSync(path.join(principalDir(root), 'fake-oauth-lost'), '1');
      },
    },
  });

  test('the card names the account and opens its sign-in; signed in again, the agent goes on by itself', async ({ app }) => {
    test.setTimeout(120_000);
    const { page } = app;
    await addProject(page, app.repo);
    await send(page, 'Bonjour');
    const card = page.getByTestId('turn-sign-in');
    await expect(card).toBeVisible();
    await expect(card).toContainText('Connexion à refaire');
    await expect(card).toContainText(/Le compte .+ doit se reconnecter : Claude Code n’a pas pu rafraîchir sa session\./);
    await card.getByRole('button', { name: 'Se reconnecter' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('dialog')).not.toHaveAttribute('aria-label', 'Réglages');
    // Closed, it goes back to Réglages › Comptes, closed in turn.
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog', { name: 'Réglages' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeHidden();
    // Signed in again (by hand, elsewhere): looked at every 20 s, the agent goes on.
    fs.writeFileSync(path.join(principalDir(app.root), '.credentials.json'), signedIn('b'));
    await expect(page.getByText(/^Compte .+ reconnecté : l’agent reprend\.$/)).toBeVisible({ timeout: 45_000 });
    await expect(page.getByText('Bonjour, tu as dit : continue')).toBeVisible();
    await expect(card).toBeHidden();
  });
});
