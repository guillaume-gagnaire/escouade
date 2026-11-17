// ESC-485: Prime Agent as the manifest of plugins/prime-agent/ describes it, served by the generic ACP plugin. The
// manifest is installed from a folder under an id of anyone's (`dev.escouade.*` installs signed alone), its agent the fake
// of tests/fixtures/fake-prime-agent.mjs, found on the PATH as `prime-agent`: it speaks as Prime Agent 0.10.3 did in the
// study (one tool, a cell of Python; no mode; no permission ever asked; `loadSession: false`). It answers, its cell
// shows as Python with its code, a turn stops, and after a restart it answers on a new session, said, given its
// instructions again (ESC-491). Without « Autoriser le mode Bypass », its agents are refused, and say why (ESC-489).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const MANIFEST = path.join(ROOT, 'plugins', 'prime-agent', 'escouade-plugin.json');
const BIN = path.join(ROOT, 'tests', 'fixtures', 'prime-agent-bin');
const OWNER = 'dev.example.prime-agent';
const ENGINE = `plugin:${OWNER}:prime-agent`;

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

/** The manifest of plugins/prime-agent/ as it is, under an id of anyone's, in a folder of the test's. */
function pluginFolder(beside: string): string {
  const dir = path.join(path.dirname(beside), 'prime-agent-plugin');
  fs.mkdirSync(dir, { recursive: true });
  const manifest = JSON.parse(fs.readFileSync(MANIFEST, 'utf8'));
  manifest.id = OWNER;
  delete manifest.store;
  // Its manifest asks for 2.2.2 (the fix of the gap, ESC-489); the app under test is the version of this tree.
  manifest.escouade.min = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')).version.replace(/-.*/, '');
  fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify(manifest, null, 2));
  return dir;
}

/** The plugin installed from its folder, the program its agents run and its installers shown first; then a project, and a new agent. */
async function installAndOpen(page: Page, repo: string) {
  const read = await invoke<{ token: string; asks: string[] }>(page, 'plugin_inspect_folder', { path: pluginFolder(repo) });
  expect('ok' in read, JSON.stringify(read)).toBe(true);
  const preview = (read as { ok: { token: string; asks: string[] } }).ok;
  expect(preview.asks).toContain('harness:prime-agent --mode acp --no-session');
  // Its official installer, shown before anything is installed.
  expect(preview.asks.some((a) => a.startsWith('harnessInstall:') && a.includes('app.primeintellect.ai/prime-agent/install'))).toBe(true);
  expect('ok' in (await invoke(page, 'plugin_install', { token: preview.token }))).toBe(true);
  await addProject(page, repo);
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: /Prime Agent/ }).click();
  await expect(page.locator('main.conv header.head')).toContainText('Prime Agent');
}

/** The machine's account of Prime Agent, with what `extra` says (it has no folder of its own: no `accountVar`). */
function account(root: string, extra: Record<string, unknown> = {}) {
  return {
    accounts: [
      {
        id: `principal-${ENGINE}`,
        name: 'Principal',
        engine: ENGINE,
        configDir: path.join(root, 'prime-agent-home'),
        claudePath: '',
        active: true,
        ...extra,
      },
    ],
  };
}

test.describe('an agent of Prime Agent, Bypass allowed', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: (root) => ({ ...account(root, { defaultMode: 'bypassPermissions' }), allowBypass: true, agentMode: 'acceptEdits' }) },
  });

  test('answers, shows its cell as Python with its code, stops a turn, and answers on a new session after a restart', async ({ app }) => {
    // Two starts of the app.
    test.slow();
    const { page } = app;
    await installAndOpen(page, app.repo);
    // Its one mode, the one it runs in, shown (ESC-489).
    await expect(page.getByRole('button', { name: /^Mode/ })).toContainText('Bypass');

    await send(page, 'bonjour');
    await expect(page.getByText('Prime Agent a reçu : bonjour')).toBeVisible();

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
    await expect(conv.getByText('La commande tourne').first()).toBeVisible();
    await page.getByRole('button', { name: 'Stop' }).click();
    await expect(page.getByRole('button', { name: 'Stop' })).toHaveCount(0);
    await send(page, 'encore');
    await expect(page.getByText('Prime Agent a reçu : encore')).toBeVisible();

    // Restarted, a new process cannot load its session (`loadSession: false`): it answers on a new one, said (ESC-491).
    const page2 = await app.restart();
    // Its agent, named from its first message (its harness is only the label of an image on its card).
    await page2.locator('.card', { hasText: 'bonjour' }).first().click();
    await send(page2, 'tu te souviens ?');
    await expect(page2.getByText('Prime Agent a reçu : tu te souviens ?')).toBeVisible();
    await expect(page2.getByText(/Prime Agent ne reprend pas sa session/).first()).toBeVisible();
  });
});

test.describe('an agent of Prime Agent in its Bypass mode, Bypass not allowed', () => {
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
    await expect(page.getByText('Prime Agent a reçu : bonjour')).toHaveCount(0);
  });
});

test.describe('a new agent of Prime Agent, Bypass not allowed', () => {
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
    await expect(page.getByText(/Prime Agent ne tourne qu’en Bypass/).first()).toBeVisible();
    await expect(page.getByText('Prime Agent a reçu : bonjour')).toHaveCount(0);
  });
});

// ESC-491: the agent of a ticket on a harness that loads no session (`loadSession: false`), the app started again: its
// conversation says it starts again on a new session, and the new one is given the ticket's instructions, as at its
// first launch, with what Escouade knows of what was done.
const LOG = path.join(os.tmpdir(), `escouade-e2e-prime-resume-${process.pid}.jsonl`);

/** The prompts the fake was sent, by their texts. */
function prompts(): string[] {
  if (!fs.existsSync(LOG)) return [];
  return fs
    .readFileSync(LOG, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((l) => JSON.parse(l))
    .filter((e) => e.dir === 'c2s' && e.msg?.method === 'session/prompt')
    .map((e) => (e.msg.params.prompt ?? []).map((b: { text?: string }) => b.text ?? '').join('\n'));
}

test.describe('the agent of a ticket on Prime Agent, the app started again', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}`, FAKE_PRIME_AGENT_LOG: LOG },
    preStart: {
      run: (root) => {
        fs.rmSync(LOG, { force: true });
        return { ...account(root, { defaultMode: 'bypassPermissions' }), allowBypass: true, agentMode: 'acceptEdits' };
      },
    },
  });

  test('says its session is new, and the new one is given the ticket’s instructions again', async ({ app }) => {
    // Two starts of the app.
    test.slow();
    const { page } = app;
    // A first agent of Prime Agent: the gesture by which a harness of anyone's is looked for (`registry::gesture`).
    await installAndOpen(page, app.repo);
    await page
      .getByRole('group', { name: 'Vue du projet' })
      .getByRole('button', { name: /^Kanban/ })
      .click();
    await page.getByRole('button', { name: 'Nouveau ticket' }).click();
    await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Réparer calc');
    await page.getByRole('group', { name: 'Harness' }).getByRole('button').click();
    await page.getByRole('menuitemradio', { name: /Prime Agent/ }).click();
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    // Started by the autopilot: its first message carries the ticket's protocol.
    await expect.poll(() => prompts().find((t) => t.includes('<instructions>')) ?? '', { timeout: 60_000 }).toContain('DEM-1');
    const before = prompts().length;

    const page2 = await app.restart();
    // Its agent, named after its ticket.
    await page2.locator('.card', { hasText: /dem-1/i }).first().click();
    await send(page2, 'tu te souviens ?');
    await expect(page2.getByText(/Prime Agent ne reprend pas sa session : l’agent repart d’une session neuve\./).first()).toBeVisible({
      timeout: 60_000,
    });
    await expect(page2.getByText(/Ses consignes lui sont redonnées/).first()).toBeVisible();
    // The new session's first message: the ticket's instructions again, and what was done.
    await expect
      .poll(
        () =>
          prompts()
            .slice(before)
            .find((t) => t.includes('<instructions>')) ?? '',
        { timeout: 30_000 },
      )
      .toContain('DEM-1');
    const again = prompts()
      .slice(before)
      .find((t) => t.includes('<instructions>'))!;
    expect(again).toContain('# Reprise');
  });
});
