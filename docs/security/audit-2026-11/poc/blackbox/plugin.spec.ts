// Audit 2026-11, black box: the malicious test plugin (`../malicious-plugin`), copied in the test's folder and loaded in
// the app as a folder in development. It tries what its permissions do not cover and writes what Escouade answered on its
// journal, which the app shows; then it tries to take the app down (memory, a turn that never ends, a flood of output).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { expect, test, type App } from '../../../../../tests/e2e/fixture';
import { filesHolding, instanceProcesses, invoke, keep, memoryMb } from './helpers';

const SOURCE = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'malicious-plugin');
const ID = 'dev.audit.malveillant';
/** What the plugin is given as its one secret: looked for afterwards in everything the app wrote. */
const SECRET = 'audit-secret-7f3a9c41d2e8b605';

type Run = { status: { phase: string; restarts?: number }; journal: { kind?: string; text?: string; line?: string }[] };

/** The plugin copied in the test's folder, in `mode`, loaded in development and started. */
async function load(app: App, mode: string): Promise<string> {
  const folder = path.join(app.root, `plugin-${mode}`);
  fs.cpSync(SOURCE, folder, { recursive: true });
  fs.writeFileSync(path.join(folder, 'mode.json'), JSON.stringify({ mode }));
  const read = await invoke<{ token: string }>(app.page, 'plugin_inspect_dev', { path: folder });
  await invoke(app.page, 'plugin_install', { token: read.token });
  await invoke(app.page, 'plugin_set_secret', { id: ID, key: 'token', value: SECRET }).catch(() => {});
  await invoke(app.page, 'plugin_start', { id: ID }).catch(() => {});
  return folder;
}

const run = (page: Page) => invoke<Run>(page, 'plugin_run', { id: ID });
const lines = (r: Run) => r.journal.map((e) => String(e.text ?? e.line ?? JSON.stringify(e)));

test('the malicious plugin gets nothing its permissions do not cover', async ({ app }) => {
  const folder = await load(app, 'probe');
  let journal: string[] = [];
  await expect
    .poll(
      async () => {
        journal = lines(await run(app.page));
        return journal.some((l) => l.includes('AUDIT probe => done'));
      },
      { timeout: 120_000 },
    )
    .toBe(true);
  const tries = journal
    .filter((l) => l.includes('AUDIT '))
    .map((l) => l.slice(l.indexOf('AUDIT ') + 6))
    .map((l) => {
      const [name, ...rest] = l.split(' => ');
      return { name, outcome: rest.join(' => ').split(app.root).join('<dossier du test>') };
    });
  const allowed = tries.filter((t) => t.outcome.startsWith('OK'));
  keep('plugin-probe', { tries, allowed: allowed.map((t) => t.name) });
  // What it may do: what its manifest declares, and nothing else (a plugin sets on its programs the variables its
  // manifest declares alone, and never one of the refused families).
  const expected = [
    'fs.temp-write (allowed)',
    'proc.declared (allowed)',
    'proc.env-declared (allowed)',
    'proc.env-git-author (allowed)',
    'proc.env-ssh-askpass (declared, passes)',
    'secret.own (allowed)',
  ];
  expect(allowed.map((t) => t.name).sort()).toEqual(expected);
  // The one write it tried outside its root left nothing.
  expect(fs.existsSync(path.join(app.data, 'audit-marker.txt'))).toBe(false);
  expect(fs.existsSync(path.join(folder, '..', 'audit-marker.txt'))).toBe(false);
  // Its secret: where is it, in what the app wrote? (The plugin itself wrote it on its journal: masked or not.)
  await app.page.waitForTimeout(2500);
  const holding = filesHolding(app.data, SECRET);
  const shownInJournal = journal.filter((l) => l.includes(SECRET)).length;
  keep('plugin-secret', {
    filesOfTheDataFolderHoldingTheSecret: holding,
    journalLinesShowingIt: shownInJournal,
    journalLineOfTheSecret: journal.find((l) => l.includes('secret.own')) ?? null,
  });
  expect(shownInJournal, 'the journal shows the secret in clear').toBe(0);
  // Uninstalled: its secret goes with it (the sandbox's own entry of the keychain), nothing of the test is left there.
  await invoke(app.page, 'plugin_remove', { id: ID, keep: false });
});

// A plugin is refused the variables by which a program runs something else, even declared. And a folder loaded in
// development is read as anyone's, whatever its id: under an id of anyone's or under `dev.escouade.*`, the variables
// never reach a program, by the manifest that is refused or at run time.
test('a folder in development that declares refused variables never sets them, whatever its id', async ({ app }) => {
  const GIT = ['GIT_CONFIG_COUNT', 'GIT_CONFIG_KEY_0', 'GIT_CONFIG_VALUE_0'];
  const OURS = 'dev.escouade.audit-poc';
  const cases = [
    { name: `${ID}, declares GIT_CONFIG_*`, id: ID, env: ['AUDIT_NOTE', ...GIT] },
    { name: `${OURS}, declares GIT_CONFIG_*`, id: OURS, env: ['AUDIT_NOTE', ...GIT] },
    { name: `${OURS}, declares no refused variable`, id: OURS, env: ['AUDIT_NOTE'] },
  ];
  const seen: Record<string, { manifestRead: boolean; installed: boolean; said?: string; atRunTime?: string }> = {};
  for (const [n, c] of cases.entries()) {
    const folder = path.join(app.root, `plugin-declared-${n}`);
    fs.cpSync(SOURCE, folder, { recursive: true });
    fs.writeFileSync(path.join(folder, 'mode.json'), JSON.stringify({ mode: 'declared' }));
    const file = path.join(folder, 'escouade-plugin.json');
    const manifest = JSON.parse(fs.readFileSync(file, 'utf8'));
    manifest.id = c.id;
    manifest.permissions.env = c.env;
    fs.writeFileSync(file, JSON.stringify(manifest, null, 2));
    const short = (e: unknown) =>
      (e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e))
        .split(app.root)
        .join('<dossier du test>')
        .slice(0, 500);
    // A refusal of the core is an object: read in the page, where it is one.
    const read = await app.page.evaluate(async (p) => {
      const tauri = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      try {
        return { preview: (await tauri.invoke('plugin_inspect_dev', { path: p })) as { token: string } };
      } catch (e) {
        return { refusal: e };
      }
    }, folder);
    const token = 'preview' in read ? read.preview.token : '';
    seen[c.name] = { manifestRead: !!token, installed: false, said: 'refusal' in read ? short(read.refusal) : undefined };
    if (!token) continue;
    const installed = await invoke(app.page, 'plugin_install', { token }).then(
      () => true,
      (e) => ((seen[c.name].said = short(e)), false),
    );
    seen[c.name].installed = installed;
    if (!installed) continue;
    await invoke(app.page, 'plugin_start', { id: c.id }).catch(() => {});
    let line: string | undefined;
    await expect
      .poll(
        async () => {
          const journal = lines(await invoke<Run>(app.page, 'plugin_run', { id: c.id }));
          line = journal.find((l) => l.includes('AUDIT proc.env-git-config (declared) => '));
          return !!line;
        },
        { timeout: 120_000 },
      )
      .toBe(true);
    seen[c.name].atRunTime = line!.slice(line!.indexOf(' => ') + 4).slice(0, 200);
    await invoke(app.page, 'plugin_remove', { id: c.id, keep: false });
  }
  keep('plugin-env-declared', seen);
  // Of anyone's: whatever its manifest was read as, the variables are refused when it runs.
  const theirs = seen[cases[0].name];
  expect(theirs.atRunTime ?? theirs.said ?? '', 'a plugin of anyone’s set GIT_CONFIG_*').not.toMatch(/^OK/);
  // A folder in development never holds the id of Escouade's own, so never its hand.
  for (const c of cases.slice(1)) {
    const ours = seen[c.name];
    expect(ours.atRunTime ?? ours.said ?? '', `${c.name}: it set GIT_CONFIG_*`).not.toMatch(/^OK/);
    expect(ours.installed, `${c.name}: a folder in development took the id of Escouade's own`).toBe(false);
  }
});

// SEC-01: « N'installer que des versions vérifiées » is on for a new installation (a test of the project
// says it; the instance of these tests starts from a settings file, as an installation that existed before, which keeps
// its choice: the setting is turned on here). What it covers is the store's versions: a folder of anyone's, copied or
// loaded in development, installs with it on (the report says so).
test.describe('« only verified versions » on', () => {
  test.use({ preStart: { run: () => ({ pluginsVerifiedOnly: true }) } });

  test('it covers the store alone: a folder still installs', async ({ app }) => {
    const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8')) as { pluginsVerifiedOnly?: boolean };
    expect(settings.pluginsVerifiedOnly).toBe(true);
    const seen: Record<string, unknown> = { pluginsVerifiedOnly: settings.pluginsVerifiedOnly };
    for (const [how, command] of [
      ['a folder copied (« Installer depuis un dossier »)', 'plugin_inspect_folder'],
      ['a folder in development', 'plugin_inspect_dev'],
    ] as const) {
      const folder = path.join(app.root, `plugin-${command}`);
      fs.cpSync(SOURCE, folder, { recursive: true });
      fs.writeFileSync(path.join(folder, 'mode.json'), JSON.stringify({ mode: 'hold' }));
      const short = (e: unknown) =>
        (e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e))
          .split(app.root)
          .join('<dossier du test>')
          .slice(0, 300);
      const installed = await invoke<{ token: string }>(app.page, command, { path: folder })
        .then((read) => invoke(app.page, 'plugin_install', { token: read.token }))
        .then(
          () => 'installed',
          (e) => `refused: ${short(e)}`,
        );
      seen[how] = installed;
      if (installed === 'installed') await invoke(app.page, 'plugin_remove', { id: ID, keep: false });
    }
    keep('plugin-verified-only', seen);
  });
});

for (const mode of ['grow', 'loop', 'flood'] as const) {
  test(`the malicious plugin cannot take the app down (${mode})`, async ({ app }) => {
    const pid = instanceProcesses(app.root).find((p) => p.name.toLowerCase() === 'escouade.exe')!.pid;
    const before = memoryMb(pid);
    let peak = before;
    await load(app, mode);
    const seen: { atMs: number; phase: string; restarts?: number; privateMemoryMb: number }[] = [];
    const started = Date.now();
    // 40 seconds of it: the instance is stopped, started again within its budget, then left stopped.
    while (Date.now() - started < 40_000) {
      const r = await run(app.page).catch(() => null);
      const mb = memoryMb(pid);
      peak = Math.max(peak, mb);
      if (r && (seen.length === 0 || seen.at(-1)!.phase !== r.status.phase || seen.at(-1)!.restarts !== r.status.restarts)) {
        seen.push({ atMs: Date.now() - started, phase: r.status.phase, restarts: r.status.restarts, privateMemoryMb: mb });
      }
      await app.page.waitForTimeout(250);
    }
    const last = await run(app.page);
    const after = memoryMb(pid);
    const said = lines(last)
      .filter((l) => /stopped|crash|memory|past|arr[êe]t|plant/i.test(l))
      .slice(-4);
    keep(`plugin-dos-${mode}`, {
      privateMemoryMb: { before, peak, after },
      phases: seen,
      finalPhase: last.status.phase,
      journalSaid: said.map((l) => l.slice(0, 200)),
    });
    // The app still answers, its window still there.
    expect(await invoke<unknown[]>(app.page, 'plugins_list')).toBeTruthy();
    expect(await app.page.evaluate(() => document.readyState)).toBe('complete');
  });
}
