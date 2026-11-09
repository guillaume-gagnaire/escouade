import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, type App, openSettingsPage, closeSettings } from './fixture';

// « Vue Claude Code » (ESC-106): Claude Code's own interface, played by the fake `claude`, which refuses `--resume` of a session
// it does not know (« No conversation found with session ID: … ») and keeps its transcript in Principal's folder (`claude/`).
// A marker file in that folder makes it crash at once (`fake-tui-crash`) or refuse every `--resume` (`fake-no-conversation`).

/** The launches of Claude Code's interface (not the stream's: a stream has `stream-json`). */
function interfaces(app: App) {
  return app.launches().filter((l) => l.argv.includes('--settings') && !l.argv.includes('stream-json'));
}

/** The launches of the stream's process (not the readings of an account's models, without a session: ESC-124). */
function streams(app: App) {
  return app.launches().filter((l) => l.argv.includes('stream-json') && !l.argv.includes('--no-session-persistence'));
}

const flag = (argv: string[], name: string) => argv.find((a) => a.startsWith(`${name}=`))?.slice(name.length + 1);

/**
 * Back in Escouade's view, the loss is told once: the stream does not resume the session the interface (launched with
 * `argv`) named and never wrote (no message sent in it), which would fail and tell it again (ESC-113).
 */
async function toldOnceBackInEscouadesView(app: App, argv: string[]) {
  const before = streams(app).length;
  await app.page.getByRole('button', { name: 'Vue Escouade' }).click();
  await expect.poll(() => streams(app).length, { timeout: 20_000 }).toBeGreaterThan(before);
  const named = flag(argv, '--session-id');
  expect(streams(app).some((l) => flag(l.argv, '--resume') === named)).toBe(false);
  // The time a stream that failed on it would take to say so.
  await app.page.waitForTimeout(2000);
  await expect(app.page.getByText(/Session Claude introuvable/)).toHaveCount(1);
}

/** The marker, in Principal's Claude Code folder. */
const marker = (app: App, name: string) => path.join(app.root, 'claude', name);

/** Opens the view of the first agent; returns the field its interface takes the keys in. */
async function openInterface(page: Page) {
  await page.getByRole('button', { name: 'Vue Claude Code' }).click();
  const field = page.locator('main [data-pastes="images"] .xterm textarea');
  await expect(field).toBeAttached({ timeout: 20_000 });
  return field;
}

/** `text` pasted in the interface, then Enter: a message of the user's. */
async function say(field: ReturnType<Page['locator']>, text: string) {
  await field.evaluate((el, text) => {
    const data = new DataTransfer();
    data.setData('text/plain', text);
    el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
  }, text);
  await field.press('Enter');
}

/** The prompts Claude Code got, in its transcripts. */
function prompts(app: App): string[] {
  const projects = path.join(app.root, 'claude', 'projects');
  if (!fs.existsSync(projects)) return [];
  return fs
    .readdirSync(projects, { recursive: true, encoding: 'utf8' })
    .filter((f) => f.endsWith('.jsonl'))
    .flatMap((f) => fs.readFileSync(path.join(projects, f), 'utf8').trim().split('\n'))
    .map((l) => JSON.parse(l))
    .filter((e) => e.type === 'user' && typeof e.message?.content === 'string')
    .map((e) => e.message.content);
}

test('an interface that stops as soon as it starts is never started again on its own: a clear message, and « Relancer »', async ({
  app,
}) => {
  const { page } = app;
  fs.mkdirSync(path.dirname(marker(app, 'fake-tui-crash')), { recursive: true });
  fs.writeFileSync(marker(app, 'fake-tui-crash'), '');
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Vue Claude Code' }).click();
  await expect.poll(() => interfaces(app).length, { timeout: 20_000 }).toBe(1);

  const view = page.locator('main .tui');
  await expect(view.getByText(/s’est arrêté dès son lancement/)).toBeVisible();
  const restart = view.getByRole('button', { name: 'Relancer' });
  await expect(restart).toBeVisible();
  // Left alone: no second launch (it used to start again and again, the window starting each one as the last quit).
  await page.waitForTimeout(5000);
  expect(interfaces(app)).toHaveLength(1);

  // « Relancer » starts it for real: the crash over, it stays.
  fs.rmSync(marker(app, 'fake-tui-crash'));
  await restart.click();
  await expect.poll(() => interfaces(app).length, { timeout: 20_000 }).toBe(2);
  await expect(view.getByText(/s’est arrêté/)).toHaveCount(0);
  await page.waitForTimeout(2000);
  expect(interfaces(app)).toHaveLength(2);
});

test('a session with no transcript is replaced by a new one when the interface starts again, and the agent says so', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  const field = await openInterface(page);
  await say(field, 'Bonjour');
  await expect.poll(() => prompts(app), { timeout: 15_000 }).toContain('Bonjour');
  const [first] = interfaces(app);
  const session = flag(first.argv, '--session-id');
  expect(session).toBeTruthy();

  // The interface ends (/exit); the transcript of its session is gone before it starts again.
  await say(field, '/exit');
  const view = page.locator('main .tui');
  await expect(view.getByText('Claude Code s’est arrêté.')).toBeVisible({ timeout: 15_000 });
  fs.rmSync(path.join(app.root, 'claude', 'projects'), { recursive: true, force: true });
  await view.getByRole('button', { name: 'Relancer' }).click();

  // A new session of its own (not a `--resume` that Claude Code would answer with « No conversation found »), alive.
  await expect.poll(() => interfaces(app).length, { timeout: 20_000 }).toBe(2);
  const [, second] = interfaces(app);
  expect(second.argv.some((a) => a.startsWith('--resume'))).toBe(false);
  expect(flag(second.argv, '--session-id')).toBeTruthy();
  expect(flag(second.argv, '--session-id')).not.toBe(session);
  await page.waitForTimeout(2000);
  expect(interfaces(app)).toHaveLength(2);
  await expect(view.getByText(/s’est arrêté/)).toHaveCount(0);

  // The conversation tells it.
  await toldOnceBackInEscouadesView(app, second.argv);
});

test('a session Claude Code cannot find, though its transcript is there, starts again once in a new session, never in a loop', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const field = await openInterface(page);
  await say(field, 'Bonjour');
  await expect.poll(() => prompts(app), { timeout: 15_000 }).toContain('Bonjour');
  const session = flag(interfaces(app)[0].argv, '--session-id');
  await say(field, '/exit');
  const view = page.locator('main .tui');
  await expect(view.getByText('Claude Code s’est arrêté.')).toBeVisible({ timeout: 15_000 });

  // The transcript is in the account's folder, yet Claude Code answers « No conversation found » to the resume.
  fs.writeFileSync(marker(app, 'fake-no-conversation'), '');
  await view.getByRole('button', { name: 'Relancer' }).click();

  // The resume that fails, then one start in a new session, which stays.
  await expect.poll(() => interfaces(app).length, { timeout: 30_000 }).toBe(3);
  const [, resumed, restarted] = interfaces(app);
  expect(flag(resumed.argv, '--resume')).toBe(session);
  expect(restarted.argv.some((a) => a.startsWith('--resume'))).toBe(false);
  expect(flag(restarted.argv, '--session-id')).toBeTruthy();
  expect(flag(restarted.argv, '--session-id')).not.toBe(session);
  await page.waitForTimeout(5000);
  expect(interfaces(app)).toHaveLength(3);
  await expect(view.getByText(/s’est arrêté/)).toHaveCount(0);

  await toldOnceBackInEscouadesView(app, restarted.argv);
});

test('« Vue des nouveaux agents » set to the harness’s terminal opens a new agent in Claude Code’s interface (ESC-34)', async ({ app }) => {
  const { page } = app;
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Agents');
  const choice = dialog.getByRole('group', { name: 'Vue des nouveaux agents' });
  await expect(choice.getByRole('button', { name: 'Escouade' })).toHaveAttribute('aria-pressed', 'true');
  await choice.getByRole('button', { name: 'Terminal du harness' }).click();
  await closeSettings(dialog);

  // The project's first agent starts in it: its interface, and the way back to Escouade's view in its header.
  await addProject(page, app.repo);
  await expect.poll(() => interfaces(app).length, { timeout: 20_000 }).toBe(1);
  await expect(page.locator('main [data-pastes="images"] .xterm')).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole('button', { name: 'Vue Escouade' })).toBeVisible();
});
