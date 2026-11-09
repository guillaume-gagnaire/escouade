import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, send, test, type App } from './fixture';

// The interactive commands of Claude Code typed in the composer (ESC-401): none goes to the agent as a message. /login
// opens the sign-in of the agent's account (Principal, the app's own test folder: never the user's sign-in), then the agent
// goes on there with its session; a command Escouade has no screen for opens in the Claude Code view, typed there, and the
// conversation goes on back in Escouade's view.

/** A sign-in in date, `tag` telling one from another. */
const signedIn = (tag: string) =>
  JSON.stringify({ claudeAiOauth: { accessToken: `tok-${tag}`, refreshToken: `ref-${tag}`, expiresAt: 4_102_444_800_000 } });

const principalDir = (root: string) => path.join(root, 'claude');

/** The launches of the stream's process (not the readings of an account's models, nor `claude -p`). */
const streams = (app: App) =>
  app.launches().filter((l) => l.argv.includes('stream-json') && !l.argv.includes('--no-session-persistence') && !l.argv.includes('-p'));

/** The launches of Claude Code's interface. */
const interfaces = (app: App) => app.launches().filter((l) => l.argv.includes('--settings') && !l.argv.includes('stream-json'));

/** The prompts Claude Code got, in its transcripts. */
function prompts(app: App): string[] {
  const projects = path.join(principalDir(app.root), 'projects');
  if (!fs.existsSync(projects)) return [];
  return fs
    .readdirSync(projects, { recursive: true, encoding: 'utf8' })
    .filter((f) => f.endsWith('.jsonl'))
    .flatMap((f) => fs.readFileSync(path.join(projects, f), 'utf8').trim().split('\n'))
    .map((l) => JSON.parse(l))
    .filter((e) => e.type === 'user' && typeof e.message?.content === 'string')
    .map((e) => e.message.content);
}

test.describe('the interactive commands of Claude Code', () => {
  test.use({
    preStart: {
      run: (root) => {
        fs.mkdirSync(principalDir(root), { recursive: true });
        fs.writeFileSync(path.join(principalDir(root), '.credentials.json'), signedIn('a'));
      },
    },
  });

  test('/login opens the sign-in of the agent’s account; signed in, the agent goes on there with its session', async ({ app }) => {
    test.setTimeout(120_000);
    const { page } = app;
    await addProject(page, app.repo);
    await send(page, 'Bonjour');
    await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();

    // The « / » menu says where it opens, before it is pressed.
    const box = page.getByRole('textbox').last();
    await box.fill('/logi');
    await expect(page.getByRole('option', { name: /\/login.*s’ouvre dans Comptes/ })).toBeVisible();
    await box.fill('');

    await send(page, '/login');
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).not.toHaveAttribute('aria-label', 'Réglages');
    // Its sign-in runs in the terminal of « Se connecter » (the fake `claude` there); the user signs in.
    fs.writeFileSync(path.join(principalDir(app.root), '.credentials.json'), signedIn('b'));
    await expect(dialog.getByText(/^Connecté/)).toBeVisible({ timeout: 20_000 });
    await dialog.getByRole('button', { name: 'Terminé' }).click();
    // Back to the conversation, not to the settings.
    await expect(page.getByRole('dialog')).toBeHidden();
    await expect(page.getByText(/^Compte .+ connecté : l’agent reprend sur ce compte, avec sa session\.$/)).toBeVisible();

    // Nothing was sent to the agent; its next message starts a process on its session.
    const before = streams(app).length;
    await send(page, 'Encore');
    await expect(page.getByText('Bonjour, tu as dit : Encore')).toBeVisible();
    await expect.poll(() => streams(app).length).toBeGreaterThan(before);
    expect(
      streams(app)
        .at(-1)!
        .argv.some((a) => a.startsWith('--resume=')),
    ).toBe(true);
    await expect(page.getByText(/tu as dit : \/login/)).toHaveCount(0);
  });

  test('a command Escouade has no screen for opens in the Claude Code view, typed there; back in Escouade, the conversation goes on', async ({
    app,
  }) => {
    test.setTimeout(120_000);
    const { page } = app;
    await addProject(page, app.repo);
    await send(page, 'Bonjour');
    await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible();

    await send(page, '/hooks');
    await expect(
      page.getByText('/hooks s’ouvre dans la vue Claude Code. « Vue Escouade » ramène à la conversation, qui continue.'),
    ).toBeVisible();
    await expect.poll(() => interfaces(app).length, { timeout: 20_000 }).toBe(1);
    await expect(page.locator('main [data-pastes="images"] .xterm')).toBeVisible();
    // Typed in its interface, on the agent's session.
    await expect.poll(() => prompts(app), { timeout: 20_000 }).toContain('/hooks');

    await page.getByRole('button', { name: 'Vue Escouade' }).click();
    // Its composer back, the interface's terminal gone.
    await expect(page.getByRole('textbox', { name: /^Envoyer un message/ })).toBeVisible();
    await expect(page.locator('main .xterm')).toHaveCount(0);
    await send(page, 'Et maintenant');
    await expect(page.getByText('Bonjour, tu as dit : Et maintenant')).toBeVisible({ timeout: 20_000 });
    // Never a message of the user's in Escouade's stream.
    await expect(page.getByText(/tu as dit : \/hooks/)).toHaveCount(0);
  });
});
