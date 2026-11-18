// ESC-484: Unharnessed as the manifest of plugins/unharnessed/ describes it, served by the generic ACP plugin. The
// manifest is installed from a folder under an id of anyone's (`dev.escouade.*` installs signed alone), its agent the
// fake of tests/fixtures/fake-unharnessed.mjs, found on the PATH as `unharnessed`: it speaks as Unharnessed 0.9.8 did in
// the study (one tool, a cell of Python; no mode; no permission ever asked). It answers, its cell shows as Python with
// its code, a turn stops, a turn it begins by itself (a wake) is a turn of its own, and after a restart its session is
// taken back by `--resume` (ESC-491). Without « Autoriser le mode Bypass », its agents are refused, and say why (ESC-489).

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const MANIFEST = path.join(ROOT, 'plugins', 'unharnessed', 'escouade-plugin.json');
const BIN = path.join(ROOT, 'tests', 'fixtures', 'unharnessed-bin');
const OWNER = 'dev.example.unharnessed';
const ENGINE = `plugin:${OWNER}:unharnessed`;

type Result<T> = { ok: T } | { err: unknown };

async function invoke<T>(page: Page, cmd: string, args: Record<string, unknown> = {}): Promise<Result<T>> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(
    async ([cmd, args]) => {
      const t = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      try {
        return { ok: await t.invoke(cmd as string, args) };
      } catch (err) {
        return { err };
      }
    },
    [cmd, args] as const,
  ) as never;
}

/** The manifest of plugins/unharnessed/ as it is, under an id of anyone's, in a folder of the test's. */
function pluginFolder(beside: string): string {
  const dir = path.join(path.dirname(beside), 'unharnessed-plugin');
  fs.mkdirSync(dir, { recursive: true });
  const manifest = JSON.parse(fs.readFileSync(MANIFEST, 'utf8'));
  manifest.id = OWNER;
  delete manifest.store;
  // Its manifest asks for 2.2.2 (the fix of the gap, ESC-489); the app under test is the version of this tree.
  manifest.escouade.min = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')).version.replace(/-.*/, '');
  fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify(manifest, null, 2));
  return dir;
}

/** The plugin installed from its folder, the program its agents run shown first; then a project, and a new agent. */
async function installAndOpen(page: Page, repo: string) {
  const read = await invoke<{ token: string; asks: string[] }>(page, 'plugin_inspect_folder', { path: pluginFolder(repo) });
  expect('ok' in read, JSON.stringify(read)).toBe(true);
  const preview = (read as { ok: { token: string; asks: string[] } }).ok;
  expect(preview.asks).toContain('harness:unharnessed --mode acp');
  // Nothing to install it: no command of anyone's shown.
  expect(preview.asks.some((a) => a.startsWith('harnessInstall:'))).toBe(false);
  expect('ok' in (await invoke(page, 'plugin_install', { token: preview.token }))).toBe(true);
  await addProject(page, repo);
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: /Unharnessed/ }).click();
  await expect(page.locator('main.conv header.head')).toContainText('Unharnessed');
}

/** An account of Unharnessed, its folder of its own (`UNHARNESSED_CODING_AGENT_DIR`), with what `extra` says. */
function account(root: string, extra: Record<string, unknown> = {}) {
  return {
    accounts: [
      {
        id: `principal-${ENGINE}`,
        name: 'Principal',
        engine: ENGINE,
        configDir: path.join(root, 'unharnessed-home'),
        claudePath: '',
        active: true,
        ...extra,
      },
    ],
  };
}

test.describe('an agent of Unharnessed, Bypass allowed', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: (root) => ({ ...account(root), allowBypass: true, agentMode: 'acceptEdits' }) },
  });

  test('answers, shows its cell as Python with its code, stops a turn, has turns of its own, and takes its session back after a restart', async ({
    app,
  }) => {
    // Two starts of the app.
    test.slow();
    const { page } = app;
    await installAndOpen(page, app.repo);
    // Its one mode, the one it runs in, shown (ESC-489).
    await expect(page.getByRole('button', { name: /^Mode/ })).toContainText('Bypass');

    await send(page, 'bonjour');
    await expect(page.getByText('Unharnessed a reçu : bonjour')).toBeVisible();

    // Its one tool: a cell of Python, its code on its row, its output once opened.
    await send(page, 'python');
    await expect(page.getByText('2 ** 100 vaut 1267650600228229401496703205376.')).toBeVisible();
    const conv = page.locator('main.conv');
    await expect(conv.getByText('Python', { exact: true }).first()).toBeVisible();
    await expect(conv.getByText('print(2 ** 100)').first()).toBeVisible();
    await conv.getByText('print(2 ** 100)').first().click();
    await expect(conv.getByText('1267650600228229401496703205376', { exact: true }).first()).toBeVisible();
    // It asked nothing: no card.
    await expect(page.getByTestId('permission-pending')).toHaveCount(0);

    // A turn under way stops (`session/cancel`), and the next message goes.
    await send(page, 'long');
    await expect(conv.getByText("r = await bash('sleep 90')").first()).toBeVisible();
    await page.getByRole('button', { name: 'Stop' }).click();
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);
    await send(page, 'encore');
    await expect(page.getByText('Unharnessed a reçu : encore')).toBeVisible();

    // A turn it begins by itself once its turn is over (a wake): a turn of its own, the agent at work then at rest; its
    // words are not those of the next answer (ESC-491).
    await send(page, 'réveil');
    await expect(page.getByText('Réveillé par la fin de sa commande.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);
    await send(page, 'après');
    await expect(page.getByText('Unharnessed a reçu : après')).toBeVisible();
    await expect(conv.locator('.md', { hasText: 'Réveillé par la fin de sa commande.' })).not.toContainText('Unharnessed a reçu');

    // Restarted, a new process takes its session back (`--resume <id>`, `acp.resume`): nothing to say.
    const page2 = await app.restart();
    await page2.locator('.card', { hasText: 'bonjour' }).first().click();
    await send(page2, 'tu te souviens ?');
    await expect(page2.getByText('Unharnessed a reçu : tu te souviens ?')).toBeVisible();
    await expect(page2.getByText(/n’a pas pu être reprise|ne reprend pas sa session/)).toHaveCount(0);
  });
});

test.describe('an agent of Unharnessed in its Bypass mode, Bypass not allowed', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: (root) => ({ ...account(root, { defaultMode: 'bypassPermissions' }), allowBypass: false, agentMode: 'acceptEdits' }) },
  });

  test('does not start, and says why', async ({ app }) => {
    const { page } = app;
    await installAndOpen(page, app.repo);
    await send(page, 'bonjour');
    // Its one mode, Bypass, not allowed: the refusal says it runs only in it, and what to do (ESC-489).
    await expect(page.getByText(/ne tourne qu’en Bypass/).first()).toBeVisible();
    await expect(page.getByText('Unharnessed a reçu : bonjour')).toHaveCount(0);
  });
});

test.describe('a new agent of Unharnessed, Bypass not allowed', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: (root) => ({ ...account(root), allowBypass: false, agentMode: 'acceptEdits' }) },
  });

  // The gap ESC-484 showed, closed by ESC-489: a harness whose one mode is Bypass has its agents judged in it.
  test('does not start, and says why and what to do', async ({ app }) => {
    const { page } = app;
    await installAndOpen(page, app.repo);
    await expect(page.getByRole('button', { name: /^Mode/ })).toContainText('Bypass');
    await send(page, 'bonjour');
    await expect(page.getByText(/Unharnessed ne tourne qu’en Bypass/).first()).toBeVisible();
    await expect(page.getByText('Unharnessed a reçu : bonjour')).toHaveCount(0);
  });
});
